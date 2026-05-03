use crate::schema::*;
use axum::extract::State;
use axum::response::IntoResponse;
use serde::Deserialize;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;
use tracing::{error, info};

pub const GOOGLE_CHAT_API_BASE: &str = "https://chat.googleapis.com/v1";
const GOOGLE_CHAT_MESSAGE_LIMIT: usize = 4096;

// --- Google Chat types (v2 envelope format) ---

#[derive(Debug, Deserialize)]
pub struct GoogleChatEnvelope {
    pub chat: Option<ChatPayload>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatPayload {
    pub user: Option<GoogleChatUser>,
    pub message_payload: Option<MessagePayload>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MessagePayload {
    pub message: Option<GoogleChatMessage>,
    pub space: Option<GoogleChatSpace>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GoogleChatMessage {
    pub name: String,
    pub text: Option<String>,
    pub argument_text: Option<String>,
    pub sender: Option<GoogleChatUser>,
    pub thread: Option<GoogleChatThread>,
    pub space: Option<GoogleChatSpace>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GoogleChatUser {
    pub name: String,
    pub display_name: String,
    #[serde(rename = "type")]
    pub user_type: String,
}

#[derive(Debug, Deserialize)]
pub struct GoogleChatThread {
    pub name: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GoogleChatSpace {
    pub name: String,
    #[serde(rename = "type")]
    pub space_type: Option<String>,
    pub space_type_renamed: Option<String>,
}

// --- Webhook handler ---

pub async fn webhook(
    State(state): State<Arc<crate::AppState>>,
    body: axum::body::Bytes,
) -> axum::response::Response {
    info!("googlechat webhook received ({} bytes)", body.len());

    let envelope: GoogleChatEnvelope = match serde_json::from_slice(&body) {
        Ok(e) => e,
        Err(e) => {
            let body_str = String::from_utf8_lossy(&body);
            error!(body = %body_str, "googlechat webhook parse error: {e}");
            return (axum::http::StatusCode::BAD_REQUEST, "bad request").into_response();
        }
    };

    let Some(chat) = envelope.chat else {
        return empty_json_response();
    };
    let Some(payload) = chat.message_payload else {
        return empty_json_response();
    };
    let Some(ref msg) = payload.message else {
        return empty_json_response();
    };

    let text = msg
        .argument_text
        .as_deref()
        .or(msg.text.as_deref())
        .unwrap_or("");
    if text.trim().is_empty() {
        return empty_json_response();
    }

    let sender = msg.sender.as_ref().or(chat.user.as_ref());
    let space = msg.space.as_ref().or(payload.space.as_ref());

    let is_bot = sender.map(|s| s.user_type == "BOT").unwrap_or(false);
    if is_bot {
        return empty_json_response();
    }

    let sender_id = sender.map(|s| s.name.clone()).unwrap_or_default();
    let display_name = sender
        .map(|s| s.display_name.clone())
        .unwrap_or_else(|| "Unknown".into());
    let sender_name = sender_id
        .strip_prefix("users/")
        .unwrap_or(&sender_id)
        .to_string();

    let space_name = space.map(|s| s.name.clone()).unwrap_or_default();
    let space_type = space
        .and_then(|s| s.space_type.clone())
        .unwrap_or_else(|| "ROOM".into());

    let thread_id = msg.thread.as_ref().map(|t| t.name.clone());

    let message_id = msg
        .name
        .rsplit('/')
        .next()
        .unwrap_or(&msg.name)
        .to_string();

    let gw_event = GatewayEvent::new(
        "googlechat",
        ChannelInfo {
            id: space_name.clone(),
            channel_type: space_type,
            thread_id,
        },
        SenderInfo {
            id: sender_id,
            name: sender_name.clone(),
            display_name,
            is_bot: false,
        },
        text,
        &message_id,
        vec![],
    );

    let json = serde_json::to_string(&gw_event).unwrap();
    info!(space = %space_name, sender = %sender_name, "googlechat → gateway");
    let _ = state.event_tx.send(json);
    empty_json_response()
}

fn empty_json_response() -> axum::response::Response {
    use axum::response::IntoResponse;
    (
        [(axum::http::header::CONTENT_TYPE, "application/json")],
        "{}",
    )
        .into_response()
}

// --- Token cache with JWT auto-refresh ---

pub struct GoogleChatTokenCache {
    token: RwLock<Option<(String, Instant, u64)>>,
    sa_email: String,
    private_key: String,
}

const TOKEN_REFRESH_MARGIN_SECS: u64 = 300;

impl GoogleChatTokenCache {
    pub fn new(sa_key_json: &str) -> Result<Self, String> {
        let key: serde_json::Value =
            serde_json::from_str(sa_key_json).map_err(|e| format!("invalid SA key JSON: {e}"))?;
        let email = key
            .get("client_email")
            .and_then(|v| v.as_str())
            .ok_or("missing client_email in SA key")?
            .to_string();
        let pkey = key
            .get("private_key")
            .and_then(|v| v.as_str())
            .ok_or("missing private_key in SA key")?
            .to_string();
        Ok(Self {
            token: RwLock::new(None),
            sa_email: email,
            private_key: pkey,
        })
    }

    pub async fn get_token(&self, client: &reqwest::Client) -> Result<String, String> {
        {
            let guard = self.token.read().await;
            if let Some((ref tok, ref ts, ttl)) = *guard {
                if ts.elapsed().as_secs() < ttl.saturating_sub(TOKEN_REFRESH_MARGIN_SECS) {
                    return Ok(tok.clone());
                }
            }
        }
        let mut guard = self.token.write().await;
        if let Some((ref tok, ref ts, ttl)) = *guard {
            if ts.elapsed().as_secs() < ttl.saturating_sub(TOKEN_REFRESH_MARGIN_SECS) {
                return Ok(tok.clone());
            }
        }
        let (new_token, expire) = self.refresh(client).await?;
        *guard = Some((new_token.clone(), Instant::now(), expire));
        info!("googlechat access token refreshed (expires in {expire}s)");
        Ok(new_token)
    }

    async fn refresh(&self, client: &reqwest::Client) -> Result<(String, u64), String> {
        let jwt = self.build_jwt().map_err(|e| format!("JWT build error: {e}"))?;
        let resp = client
            .post("https://oauth2.googleapis.com/token")
            .form(&[
                ("grant_type", "urn:ietf:params:oauth:grant-type:jwt-bearer"),
                ("assertion", &jwt),
            ])
            .send()
            .await
            .map_err(|e| format!("token exchange request failed: {e}"))?;

        let body: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| format!("token exchange parse failed: {e}"))?;

        let token = body
            .get("access_token")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                let err = body
                    .get("error_description")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown error");
                format!("token exchange failed: {err}")
            })?
            .to_string();

        let expires_in = body
            .get("expires_in")
            .and_then(|v| v.as_u64())
            .unwrap_or(3600);

        Ok((token, expires_in))
    }

    fn build_jwt(&self) -> Result<String, String> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| e.to_string())?
            .as_secs();

        let claims = serde_json::json!({
            "iss": self.sa_email,
            "scope": "https://www.googleapis.com/auth/chat.bot",
            "aud": "https://oauth2.googleapis.com/token",
            "iat": now,
            "exp": now + 3600,
        });

        let key = jsonwebtoken::EncodingKey::from_rsa_pem(self.private_key.as_bytes())
            .map_err(|e| format!("RSA key parse error: {e}"))?;
        let header = jsonwebtoken::Header::new(jsonwebtoken::Algorithm::RS256);
        jsonwebtoken::encode(&header, &claims, &key)
            .map_err(|e| format!("JWT encode error: {e}"))
    }
}

// --- Reply handler ---

pub async fn handle_reply(
    reply: &GatewayReply,
    token_cache: Option<&GoogleChatTokenCache>,
    static_token: Option<&str>,
    client: &reqwest::Client,
) {
    if reply.command.as_deref() == Some("add_reaction")
        || reply.command.as_deref() == Some("remove_reaction")
    {
        return;
    }

    if reply.command.as_deref() == Some("create_topic") {
        return;
    }

    info!(
        space = %reply.channel.id,
        thread_id = ?reply.channel.thread_id,
        "gateway → googlechat"
    );

    let token = if let Some(cache) = token_cache {
        match cache.get_token(client).await {
            Ok(t) => t,
            Err(e) => {
                error!("googlechat token refresh failed: {e}");
                return;
            }
        }
    } else if let Some(t) = static_token {
        t.to_string()
    } else {
        info!(
            text = %reply.content.text,
            "googlechat reply (dry-run, no credentials configured)"
        );
        return;
    };

    let text = &reply.content.text;
    let chunks = split_text(text, GOOGLE_CHAT_MESSAGE_LIMIT);

    for chunk in chunks {
        send_message(client, &token, &reply.channel.id, reply.channel.thread_id.as_deref(), chunk).await;
    }
}

async fn send_message(
    client: &reqwest::Client,
    token: &str,
    space: &str,
    thread_id: Option<&str>,
    text: &str,
) {
    let mut url = format!("{}/{}/messages", GOOGLE_CHAT_API_BASE, space);

    let mut body = serde_json::json!({
        "text": text,
    });

    if let Some(thread_id) = thread_id {
        body["thread"] = serde_json::json!({
            "name": thread_id,
        });
        url.push_str("?messageReplyOption=REPLY_MESSAGE_FALLBACK_TO_NEW_THREAD");
    }

    let resp = client
        .post(&url)
        .bearer_auth(token)
        .json(&body)
        .send()
        .await;

    match resp {
        Ok(r) if !r.status().is_success() => {
            let status = r.status();
            let body = r.text().await.unwrap_or_default();
            error!(status = %status, body = %body, "googlechat send error");
        }
        Err(e) => error!("googlechat send error: {e}"),
        _ => {}
    }
}

fn split_text(text: &str, limit: usize) -> Vec<&str> {
    let mut chunks = Vec::new();
    let mut start = 0;
    while start < text.len() {
        if start + limit >= text.len() {
            chunks.push(&text[start..]);
            break;
        }
        let mut end = start + limit;
        while !text.is_char_boundary(end) {
            end -= 1;
        }
        let mut search_start = if end > start + 200 { end - 200 } else { start };
        while search_start < end && !text.is_char_boundary(search_start) {
            search_start += 1;
        }
        let break_at = text[search_start..end]
            .rfind('\n')
            .or_else(|| text[search_start..end].rfind(' '))
            .map(|pos| search_start + pos + 1)
            .unwrap_or(end);
        chunks.push(&text[start..break_at]);
        start = break_at;
    }
    chunks
}
