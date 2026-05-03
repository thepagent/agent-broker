# Google Chat Setup

Connect a Google Chat app to OpenAB via the Custom Gateway.

```
Google Chat ──POST──▶ Gateway (:8080) ◀──WebSocket── OAB Pod
                                          (OAB connects out)
```

## Prerequisites

- A running OAB instance (with kiro-cli or any ACP agent authenticated)
- The Custom Gateway deployed ([gateway/README.md](../gateway/README.md))
- A Google Cloud project with the Google Chat API enabled
- A Google Cloud Service Account with the Chat Bot scope

## 1. Create a Google Chat App

1. Go to the [Google Cloud Console](https://console.cloud.google.com/) and create or select a project.
2. Enable the **Google Chat API** under **APIs & Services → Library**.
3. Go to **APIs & Services → Google Chat API → Configuration**:
   - **App name**: your bot name (e.g. "OpenAB")
   - **Avatar URL**: any public image URL
   - **Description**: anything
   - **Interactive features**: Enable
   - **Connection settings**: select **App URL** and enter your gateway's webhook URL:
     ```
     https://your-gateway-host/webhook/googlechat
     ```
   - **Visibility**: select the users or domains that can use the bot
4. Click **Save**.

## 2. Create a Service Account

Google Chat uses a service account to authenticate outbound API calls (bot replies).

1. Go to **IAM & Admin → Service Accounts** → **Create Service Account**.
2. Name it (e.g. `openab-google-chat`) and grant it no special roles.
3. After creation, click the service account → **Keys** → **Add Key** → **Create New Key** → JSON.
4. Save the downloaded JSON file securely.

## 3. Generate an Access Token

The gateway needs an OAuth2 access token to send replies. Generate one from the service account key:

```bash
# Using Node.js
node -e "
const crypto = require('crypto');
const https = require('https');
const sa = require('./service-account-key.json');
const header = Buffer.from(JSON.stringify({alg:'RS256',typ:'JWT'})).toString('base64url');
const now = Math.floor(Date.now()/1000);
const claims = Buffer.from(JSON.stringify({
  iss: sa.client_email,
  scope: 'https://www.googleapis.com/auth/chat.bot',
  aud: 'https://oauth2.googleapis.com/token',
  iat: now, exp: now + 3600
})).toString('base64url');
const sig = crypto.sign('sha256', Buffer.from(header+'.'+claims), sa.private_key).toString('base64url');
const body = 'grant_type=urn%3Aietf%3Aparams%3Aoauth%3Agrant-type%3Ajwt-bearer&assertion='+header+'.'+claims+'.'+sig;
const req = https.request('https://oauth2.googleapis.com/token', {method:'POST',headers:{'Content-Type':'application/x-www-form-urlencoded'}}, res => {
  let d=''; res.on('data',c=>d+=c); res.on('end',()=>console.log(JSON.parse(d).access_token));
});
req.write(body); req.end();
"
```

> **Note:** Access tokens expire after 1 hour. For production, implement automatic token refresh. Phase 2 of this adapter will add built-in service account JWT token refresh.

## 4. Configure the Gateway

```bash
# Docker
docker run -d --name openab-gateway \
  -e GOOGLE_CHAT_ENABLED=true \
  -e GOOGLE_CHAT_ACCESS_TOKEN="ya29.c..." \
  -e GATEWAY_WS_TOKEN="your-ws-auth-token" \
  -p 8080:8080 \
  ghcr.io/openabdev/openab-gateway:latest

# Local development
export GOOGLE_CHAT_ENABLED=true
export GOOGLE_CHAT_ACCESS_TOKEN="ya29.c..."
cargo run --release
```

## 5. Expose the Gateway (for local dev)

Google Chat requires a public HTTPS endpoint for webhooks.

### Cloudflare Tunnel (quickest)

```bash
cloudflared tunnel --url http://localhost:8080
# Copy the https://xxx.trycloudflare.com URL
```

Then update the webhook URL in the Google Chat API Configuration page:
```
https://xxx.trycloudflare.com/webhook/googlechat
```

### Reverse proxy (production)

Use nginx, Caddy, or a cloud load balancer with TLS termination pointing to the gateway's `:8080`.

## 6. Configure OAB

```toml
[gateway]
url = "ws://openab-gateway:8080/ws"
platform = "googlechat"
allow_all_channels = true
allow_all_users = true

[agent]
command = "kiro-cli"
args = ["acp", "--trust-all-tools"]
working_dir = "/home/agent"
```

## Features

### Supported

- **DM chat** — send a direct message to the bot, get an AI agent response
- **Space chat** — add the bot to a Google Chat Space, @mention it to start a conversation
- **Thread replies** — in Spaces, bot replies are posted in the same thread as the user's message
- **`argument_text` extraction** — strips the @mention prefix to get the clean user message

### Not Supported

- **Reactions** — Google Chat API does not support message reactions on behalf of bots
- **Markdown rendering** — replies are sent as plain text (Google Chat uses its own card markup)
- **File/image attachments** — not yet implemented
- **Auto token refresh** — access tokens must be refreshed manually (1-hour TTL); built-in refresh is planned

## Environment Variables (Gateway)

| Variable | Required | Default | Description |
|---|---|---|---|
| `GOOGLE_CHAT_ENABLED` | Yes | `false` | Set to `true` or `1` to enable the adapter |
| `GOOGLE_CHAT_ACCESS_TOKEN` | No | — | OAuth2 access token for Chat API replies |
| `GOOGLE_CHAT_WEBHOOK_PATH` | No | `/webhook/googlechat` | Webhook endpoint path |

## Troubleshooting

| Problem | Fix |
|---|---|
| Bot doesn't respond | Check `GOOGLE_CHAT_ENABLED=true` is set. Check gateway logs for parse errors. |
| "not responding" in Google Chat | Ensure the gateway returns a `200` with `{}` body. Check gateway is reachable via the webhook URL. |
| Replies not sent | Check `GOOGLE_CHAT_ACCESS_TOKEN` is set and not expired (1-hour TTL). Regenerate from service account. |
| Replies not in thread | Verify the thread name is passed correctly. The gateway appends `?messageReplyOption=REPLY_MESSAGE_FALLBACK_TO_NEW_THREAD` automatically. |
| Bot responds to its own messages | Bot messages have `user_type: "BOT"` and are filtered out automatically. |
| Webhook returns 400 | Check the Google Chat API configuration uses **App URL** (not Dialogflow or Cloud Pub/Sub). The webhook expects the v2 envelope format with a `chat` wrapper. |

## References

- [Google Chat API Documentation](https://developers.google.com/workspace/chat/api/reference/rest)
- [Google Chat App Setup](https://developers.google.com/workspace/chat/overview)
- [Service Account Authentication](https://developers.google.com/workspace/chat/authenticate-authorize-chat-app)
