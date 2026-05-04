use crate::acp::protocol::{ConfigOption, JsonRpcMessage, JsonRpcRequest, JsonRpcResponse, parse_config_options};
use anyhow::{anyhow, Result};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin};
use tokio::sync::{mpsc, oneshot, Mutex};
use tokio::task::JoinHandle;
use tracing::{debug, error, info};


/// Pick the most permissive selectable permission option from ACP options.
fn pick_best_option(options: &[Value]) -> Option<String> {
    let mut fallback: Option<&Value> = None;

    for kind in ["allow_always", "allow_once"] {
        if let Some(option) = options
            .iter()
            .find(|option| option.get("kind").and_then(|k| k.as_str()) == Some(kind))
        {
            return option
                .get("optionId")
                .and_then(|id| id.as_str())
                .map(str::to_owned);
        }
    }

    for option in options {
        let kind = option.get("kind").and_then(|k| k.as_str());
        if kind == Some("reject_once") || kind == Some("reject_always") {
            continue;
        }
        fallback = Some(option);
        break;
    }

    fallback
        .and_then(|option| option.get("optionId"))
        .and_then(|id| id.as_str())
        .map(str::to_owned)
}

/// Build a spec-compliant permission response with backward-compatible fallback.
fn build_permission_response(params: Option<&Value>) -> Value {
    match params
        .and_then(|p| p.get("options"))
        .and_then(|options| options.as_array())
    {
        None => json!({
            "outcome": {
                "outcome": "selected",
                "optionId": "allow_always"
            }
        }),
        Some(options) => {
            if let Some(option_id) = pick_best_option(options) {
                json!({
                    "outcome": {
                        "outcome": "selected",
                        "optionId": option_id
                    }
                })
            } else {
                json!({
                    "outcome": {
                        "outcome": "cancelled"
                    }
                })
            }
        }
    }
}

fn expand_env(val: &str) -> String {
    if val.starts_with("${") && val.ends_with('}') {
        let key = &val[2..val.len() - 1];
        std::env::var(key).unwrap_or_default()
    } else {
        val.to_string()
    }
}
use tokio::time::Instant;

/// A content block for the ACP prompt — either text or image.
#[derive(Debug, Clone)]
pub enum ContentBlock {
    Text { text: String },
    Image { media_type: String, data: String },
}

impl ContentBlock {
    pub fn to_json(&self) -> Value {
        match self {
            ContentBlock::Text { text } => json!({
                "type": "text",
                "text": text
            }),
            ContentBlock::Image { media_type, data } => json!({
                "type": "image",
                "data": data,
                "mimeType": media_type
            }),
        }
    }
}

pub struct AcpConnection {
    _proc: Child,
    /// PID of the direct child, used as the process group ID for cleanup.
    child_pgid: Option<i32>,
    stdin: Arc<Mutex<ChildStdin>>,
    next_id: AtomicU64,
    pending: Arc<Mutex<HashMap<u64, oneshot::Sender<JsonRpcMessage>>>>,
    notify_tx: Arc<Mutex<Option<mpsc::UnboundedSender<JsonRpcMessage>>>>,
    pub acp_session_id: Option<String>,
    pub supports_load_session: bool,
    pub config_options: Vec<ConfigOption>,
    pub last_active: Instant,
    pub session_reset: bool,
    _reader_handle: JoinHandle<()>,
}

/// Build the inherited portion of env vars for the agent subprocess.
///
/// Decision tree:
/// ```text
/// if enabled:
///     if allow_list non-empty:        # allow-list mode: only those keys
///         pass allow_list keys from process env
///     elif deny_list non-empty:       # deny-list mode: all except those
///         pass all process env minus deny_list
///     else:                           # pure clear: nothing inherited
///         pass nothing
/// else:                               # escape hatch: full inherit
///     pass all process env, ignoring both lists
/// ```
///
/// `explicit` ([agent].env) always wins via highest precedence.
/// Returns (merged env map, list of keys that were inherited from the process).
fn build_agent_env(
    explicit: &std::collections::HashMap<String, String>,
    clear_env: &crate::config::ClearEnvConfig,
) -> (std::collections::HashMap<String, String>, Vec<String>) {
    let mut result: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    let mut inherited: Vec<String> = Vec::new();

    // 1. Explicit [agent].env always wins.
    for (k, v) in explicit {
        result.insert(k.clone(), expand_env(v));
    }

    // 2. Inherit from process env per the decision tree above.
    if clear_env.enabled {
        if !clear_env.allow_list.is_empty() {
            for key in &clear_env.allow_list {
                if !result.contains_key(key) {
                    if let Ok(v) = std::env::var(key) {
                        result.insert(key.clone(), v);
                        inherited.push(key.clone());
                    }
                }
            }
        } else if !clear_env.deny_list.is_empty() {
            let deny: std::collections::HashSet<&str> =
                clear_env.deny_list.iter().map(String::as_str).collect();
            for (k, v) in std::env::vars() {
                if deny.contains(k.as_str()) {
                    continue;
                }
                if !result.contains_key(&k) {
                    result.insert(k.clone(), v);
                    inherited.push(k);
                }
            }
        }
        // else: pure clear — nothing inherited beyond explicit + baseline.
    } else {
        // Escape hatch: full inherit, both lists ignored.
        for (k, v) in std::env::vars() {
            if !result.contains_key(&k) {
                result.insert(k.clone(), v);
                inherited.push(k);
            }
        }
    }

    (result, inherited)
}

impl AcpConnection {
    pub async fn spawn(
        command: &str,
        args: &[String],
        working_dir: &str,
        env: &std::collections::HashMap<String, String>,
        clear_env: &crate::config::ClearEnvConfig,
    ) -> Result<Self> {
        info!(cmd = command, ?args, cwd = working_dir, "spawning agent");

        let mut cmd = tokio::process::Command::new(command);
        cmd.args(args)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .current_dir(working_dir);
        // Create a new process group so we can kill the entire tree.
        // SAFETY: setpgid is async-signal-safe (POSIX.1-2008) and called
        // before exec. Return value checked — failure means the child won't
        // have its own process group, so kill(-pgid) would be unsafe.
        #[cfg(unix)]
        unsafe {
            cmd.pre_exec(|| {
                if libc::setpgid(0, 0) != 0 {
                    return Err(std::io::Error::last_os_error());
                }
                Ok(())
            });
        }
        #[cfg(windows)]
        {
            cmd.creation_flags(0x00000200); // CREATE_NEW_PROCESS_GROUP
        }
        // Always env_clear() for determinism; build_agent_env returns the
        // exact set of vars to add back per the configured policy.
        cmd.env_clear();
        // Baseline: preserve real HOME so agents can find OAuth/auth files
        // (~/.codex, ~/.claude, ~/.config/gh, etc.). working_dir is already
        // set via current_dir() above and is not necessarily the user's home
        // directory. PATH is required for the agent binary to find tools.
        cmd.env("HOME", std::env::var("HOME").unwrap_or_else(|_| working_dir.into()));
        cmd.env("PATH", std::env::var("PATH").unwrap_or_else(|_| "/usr/local/bin:/usr/bin:/bin".into()));
        #[cfg(unix)]
        {
            cmd.env("USER", std::env::var("USER").unwrap_or_else(|_| "agent".into()));
        }
        #[cfg(windows)]
        {
            // Windows requires SystemRoot for DLL loading and basic OS functionality.
            // USERPROFILE is the Windows equivalent of HOME.
            cmd.env("USERPROFILE", std::env::var("USERPROFILE").unwrap_or_else(|_| working_dir.into()));
            cmd.env("USERNAME", std::env::var("USERNAME").unwrap_or_else(|_| "agent".into()));
            if let Ok(v) = std::env::var("SystemRoot") { cmd.env("SystemRoot", v); }
            if let Ok(v) = std::env::var("SystemDrive") { cmd.env("SystemDrive", v); }
        }
        // Build inherited set per [agent.clear_env] policy.
        let (agent_env, inherited_keys) = build_agent_env(env, clear_env);
        for (k, v) in &agent_env {
            cmd.env(k, v);
        }
        if !clear_env.enabled {
            tracing::warn!(
                inherited_count = inherited_keys.len(),
                "[agent].clear_env.enabled = false -- the agent subprocess inherits the FULL OAB process environment. All inherited values are accessible to the agent and could be exfiltrated via prompt injection. Prefer enabled = true with allow_list or deny_list when possible."
            );
        } else if !agent_env.is_empty() {
            let explicit_keys: Vec<&String> = env.keys().collect();
            tracing::warn!(
                ?explicit_keys,
                inherited_count = inherited_keys.len(),
                "[agent].env / clear_env is set -- these values are accessible to the agent and could be exfiltrated via prompt injection"
            );
        }
        let mut proc = cmd
            .spawn()
            .map_err(|e| anyhow!("failed to spawn {command}: {e}"))?;
        let child_pgid = proc.id()
            .and_then(|pid| i32::try_from(pid).ok());

        let stdout = proc.stdout.take().ok_or_else(|| anyhow!("no stdout"))?;
        let stdin = proc.stdin.take().ok_or_else(|| anyhow!("no stdin"))?;
        let stdin = Arc::new(Mutex::new(stdin));

        let pending: Arc<Mutex<HashMap<u64, oneshot::Sender<JsonRpcMessage>>>> =
            Arc::new(Mutex::new(HashMap::new()));
        let notify_tx: Arc<Mutex<Option<mpsc::UnboundedSender<JsonRpcMessage>>>> =
            Arc::new(Mutex::new(None));

        let reader_handle = {
            let pending = pending.clone();
            let notify_tx = notify_tx.clone();
            let stdin_clone = stdin.clone();
            tokio::spawn(async move {
                let mut reader = BufReader::new(stdout);
                let mut line = String::new();
                loop {
                    line.clear();
                    match reader.read_line(&mut line).await {
                        Ok(0) => break, // EOF
                        Ok(_) => {}
                        Err(e) => {
                            error!("reader error: {e}");
                            break;
                        }
                    }
                    let msg: JsonRpcMessage = match serde_json::from_str(line.trim()) {
                        Ok(m) => m,
                        Err(_) => continue,
                    };
                    debug!(line = line.trim(), "acp_recv");

                    // Auto-reply session/request_permission
                    if msg.method.as_deref() == Some("session/request_permission") {
                        if let Some(id) = msg.id {
                            let title = msg
                                .params
                                .as_ref()
                                .and_then(|p| p.get("toolCall"))
                                .and_then(|t| t.get("title"))
                                .and_then(|t| t.as_str())
                                .unwrap_or("?");

                            let outcome = build_permission_response(msg.params.as_ref());
                            info!(title, %outcome, "auto-respond permission");
                            let reply = JsonRpcResponse::new(id, outcome);
                            if let Ok(data) = serde_json::to_string(&reply) {
                                let mut w = stdin_clone.lock().await;
                                let _ = w.write_all(format!("{data}\n").as_bytes()).await;
                                let _ = w.flush().await;
                            }
                        }
                        continue;
                    }

                    // Response (has id) → resolve pending AND forward to subscriber
                    if let Some(id) = msg.id {
                        let mut map = pending.lock().await;
                        if let Some(tx) = map.remove(&id) {
                            // Forward to subscriber so they see the completion
                            let sub = notify_tx.lock().await;
                            if let Some(ntx) = sub.as_ref() {
                                // Clone the essential fields for the subscriber
                                let _ = ntx.send(JsonRpcMessage {
                                    id: Some(id),
                                    method: None,
                                    result: msg.result.clone(),
                                    error: msg.error.clone(),
                                    params: None,
                                });
                            }
                            let _ = tx.send(msg);
                            continue;
                        }
                    }

                    // Notification → forward to subscriber
                    let sub = notify_tx.lock().await;
                    if let Some(tx) = sub.as_ref() {
                        let _ = tx.send(msg);
                    }
                }

                // Connection closed — resolve all pending with error
                let mut map = pending.lock().await;
                for (_, tx) in map.drain() {
                    let _ = tx.send(JsonRpcMessage {
                        id: None,
                        method: None,
                        result: None,
                        error: Some(crate::acp::protocol::JsonRpcError {
                            code: -1,
                            message: "connection closed".into(),
                        }),
                        params: None,
                    });
                }
                // Close the notify channel so rx.recv() returns None
                let mut sub = notify_tx.lock().await;
                *sub = None;
            })
        };

        Ok(Self {
            _proc: proc,
            child_pgid,
            stdin,
            next_id: AtomicU64::new(1),
            pending,
            notify_tx,
            acp_session_id: None,
            supports_load_session: false,
            config_options: Vec::new(),
            last_active: Instant::now(),
            session_reset: false,
            _reader_handle: reader_handle,
        })
    }

    fn next_id(&self) -> u64 {
        self.next_id.fetch_add(1, Ordering::Relaxed)
    }

    pub(crate) async fn send_raw(&self, data: &str) -> Result<()> {
        debug!(data = data.trim(), "acp_send");
        let mut w = self.stdin.lock().await;
        w.write_all(data.as_bytes()).await?;
        w.write_all(b"\n").await?;
        w.flush().await?;
        Ok(())
    }

    async fn send_request(&self, method: &str, params: Option<Value>) -> Result<JsonRpcMessage> {
        let id = self.next_id();
        let req = JsonRpcRequest::new(id, method, params);
        let data = serde_json::to_string(&req)?;

        let (tx, rx) = oneshot::channel();
        self.pending.lock().await.insert(id, tx);

        self.send_raw(&data).await?;

        let timeout_secs = if method == "session/new" { 120 } else { 30 };
        let resp = tokio::time::timeout(std::time::Duration::from_secs(timeout_secs), rx)
            .await
            .map_err(|_| anyhow!("timeout waiting for {method} response"))?
            .map_err(|_| anyhow!("channel closed waiting for {method}"))?;

        if let Some(err) = &resp.error {
            return Err(anyhow!("{err}"));
        }
        Ok(resp)
    }

    pub async fn initialize(&mut self) -> Result<()> {
        let resp = self
            .send_request(
                "initialize",
                Some(json!({
                    "protocolVersion": 1,
                    "clientCapabilities": {},
                    "clientInfo": {"name": "openab", "version": "0.1.0"},
                })),
            )
            .await?;

        let result = resp.result.as_ref();
        let agent_name = result
            .and_then(|r| r.get("agentInfo"))
            .and_then(|a| a.get("name"))
            .and_then(|n| n.as_str())
            .unwrap_or("unknown");
        self.supports_load_session = result
            .and_then(|r| r.get("agentCapabilities"))
            .and_then(|c| c.get("loadSession"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        info!(agent = agent_name, load_session = self.supports_load_session, "initialized");
        Ok(())
    }

    pub async fn session_new(&mut self, cwd: &str) -> Result<String> {
        let resp = self
            .send_request(
                "session/new",
                Some(json!({"cwd": cwd, "mcpServers": []})),
            )
            .await?;

        let session_id = resp.result.as_ref()
            .and_then(|r| r.get("sessionId"))
            .and_then(|s| s.as_str())
            .ok_or_else(|| anyhow!("no sessionId in session/new response"))?
            .to_string();

        info!(session_id = %session_id, "session created");
        self.acp_session_id = Some(session_id.clone());
        if let Some(result) = resp.result.as_ref() {
            self.config_options = parse_config_options(result);
            if !self.config_options.is_empty() {
                info!(count = self.config_options.len(), "parsed configOptions");
            }
        }
        Ok(session_id)
    }

    /// Set a config option (e.g. model, mode) via ACP session/set_config_option.
    /// Returns the updated list of all config options.
    pub async fn set_config_option(&mut self, config_id: &str, value: &str) -> Result<Vec<ConfigOption>> {
        let session_id = self
            .acp_session_id
            .as_ref()
            .ok_or_else(|| anyhow!("no session"))?
            .clone();

        let resp = self
            .send_request(
                "session/set_config_option",
                Some(json!({
                    "sessionId": session_id,
                    "configId": config_id,
                    "value": value,
                })),
            )
            .await;

        match resp {
            Ok(r) => {
                if let Some(result) = r.result.as_ref() {
                    self.config_options = parse_config_options(result);
                }
                info!(config_id, value, "config option set");
            }
            Err(_) => {
                // Fall back: send as a slash command (e.g. "/model claude-sonnet-4")
                let cmd = format!("/{config_id} {value}");
                info!(cmd, "set_config_option not supported, falling back to prompt");
                let _resp = self
                    .send_request(
                        "session/prompt",
                        Some(json!({
                            "sessionId": session_id,
                            "prompt": [{"type": "text", "text": cmd}],
                        })),
                    )
                    .await?;
                for opt in &mut self.config_options {
                    if opt.id == config_id {
                        opt.current_value = value.to_string();
                    }
                }
            }
        }

        Ok(self.config_options.clone())
    }

    /// Send a prompt with content blocks (text and/or images) and return a receiver
    /// for streaming notifications. The final message on the channel will have id set
    /// (the prompt response).
    pub async fn session_prompt(
        &mut self,
        content_blocks: Vec<ContentBlock>,
    ) -> Result<(mpsc::UnboundedReceiver<JsonRpcMessage>, u64)> {
        self.last_active = Instant::now();

        let session_id = self
            .acp_session_id
            .as_ref()
            .ok_or_else(|| anyhow!("no session"))?;

        let (tx, rx) = mpsc::unbounded_channel();
        *self.notify_tx.lock().await = Some(tx);

        let id = self.next_id();

        // Convert content blocks to JSON
        let prompt_json: Vec<Value> = content_blocks
            .iter()
            .map(|b| b.to_json())
            .collect();

        let req = JsonRpcRequest::new(
            id,
            "session/prompt",
            Some(json!({
                "sessionId": session_id,
                "prompt": prompt_json,
            })),
        );
        let data = serde_json::to_string(&req)?;

        let (resp_tx, _resp_rx) = oneshot::channel();
        self.pending.lock().await.insert(id, resp_tx);

        self.send_raw(&data).await?;
        Ok((rx, id))
    }

    /// Call after prompt streaming is done to clean up subscriber.
    pub async fn prompt_done(&mut self) {
        *self.notify_tx.lock().await = None;
        self.last_active = Instant::now();
    }

    /// Return a clone of the stdin handle for lock-free cancel.
    pub fn cancel_handle(&self) -> Arc<Mutex<ChildStdin>> {
        Arc::clone(&self.stdin)
    }

    pub fn alive(&self) -> bool {
        !self._reader_handle.is_finished()
    }

    /// Resume a previous session by ID. Returns Ok(()) if the agent accepted
    /// the load, or an error if it failed (caller should fall back to session/new).
    pub async fn session_load(&mut self, session_id: &str, cwd: &str) -> Result<()> {
        let resp = self
            .send_request(
                "session/load",
                Some(json!({"sessionId": session_id, "cwd": cwd, "mcpServers": []})),
            )
            .await?;
        // Accept any non-error response as success
        if resp.error.is_some() {
            return Err(anyhow!("session/load rejected"));
        }
        info!(session_id, "session loaded");
        self.acp_session_id = Some(session_id.to_string());
        if let Some(result) = resp.result.as_ref() {
            self.config_options = parse_config_options(result);
        }
        Ok(())
    }

    /// Kill the entire process group: SIGTERM → SIGKILL.
    /// Uses std::thread (not tokio::spawn) so SIGKILL fires even during
    /// runtime shutdown or panic unwinding.
    fn kill_process_group(&mut self) {
        let pgid = match self.child_pgid {
            Some(pid) if pid > 0 => pid,
            _ => return,
        };
        #[cfg(unix)]
        {
            // Stage 1: SIGTERM the process group
            unsafe { libc::kill(-pgid, libc::SIGTERM); }
            // Stage 2: SIGKILL after brief grace (std::thread survives runtime shutdown)
            std::thread::spawn(move || {
                std::thread::sleep(std::time::Duration::from_millis(1500));
                unsafe { libc::kill(-pgid, libc::SIGKILL); }
            });
        }
        #[cfg(not(unix))]
        {
            let _ = pgid; // suppress unused warning on Windows
        }
    }
}

impl Drop for AcpConnection {
    fn drop(&mut self) {
        self.kill_process_group();
    }
}

#[cfg(test)]
mod tests {
    use super::{build_agent_env, build_permission_response, pick_best_option};
    use serde_json::json;

    #[test]
    fn picks_allow_always_over_other_options() {
        let options = vec![
            json!({"kind": "allow_once", "optionId": "once"}),
            json!({"kind": "allow_always", "optionId": "always"}),
            json!({"kind": "reject_once", "optionId": "reject"}),
        ];

        assert_eq!(pick_best_option(&options), Some("always".to_string()));
    }

    #[test]
    fn falls_back_to_first_unknown_non_reject_kind() {
        let options = vec![
            json!({"kind": "reject_once", "optionId": "reject"}),
            json!({"kind": "workspace_write", "optionId": "workspace-write"}),
        ];

        assert_eq!(
            pick_best_option(&options),
            Some("workspace-write".to_string())
        );
    }

    #[test]
    fn selects_bypass_permissions_for_exit_plan_mode() {
        let options = vec![
            json!({"optionId": "bypassPermissions", "kind": "allow_always"}),
            json!({"optionId": "acceptEdits", "kind": "allow_always"}),
            json!({"optionId": "default", "kind": "allow_once"}),
            json!({"optionId": "plan", "kind": "reject_once"}),
        ];

        assert_eq!(
            pick_best_option(&options),
            Some("bypassPermissions".to_string())
        );
    }

    #[test]
    fn returns_none_when_only_reject_options_exist() {
        let options = vec![
            json!({"kind": "reject_once", "optionId": "reject-once"}),
            json!({"kind": "reject_always", "optionId": "reject-always"}),
        ];

        assert_eq!(pick_best_option(&options), None);
    }

    #[test]
    fn builds_cancelled_outcome_when_no_selectable_option_exists() {
        let response = build_permission_response(Some(&json!({
            "options": [
                {"kind": "reject_once", "optionId": "reject-once"}
            ]
        })));

        assert_eq!(response, json!({"outcome": {"outcome": "cancelled"}}));
    }

    #[test]
    fn builds_cancelled_when_options_array_is_empty() {
        let response = build_permission_response(Some(&json!({
            "options": []
        })));

        assert_eq!(response, json!({"outcome": {"outcome": "cancelled"}}));
    }

    #[test]
    fn falls_back_to_allow_always_when_options_are_missing() {
        let response = build_permission_response(Some(&json!({
            "toolCall": {"title": "legacy"}
        })));

        assert_eq!(
            response,
            json!({"outcome": {"outcome": "selected", "optionId": "allow_always"}})
        );
    }

    #[test]
    fn falls_back_to_allow_always_when_params_is_none() {
        let response = build_permission_response(None);

        assert_eq!(
            response,
            json!({"outcome": {"outcome": "selected", "optionId": "allow_always"}})
        );
    }

    fn make_clear_env(enabled: bool, allow: Vec<&str>, deny: Vec<&str>) -> crate::config::ClearEnvConfig {
        crate::config::ClearEnvConfig {
            enabled,
            allow_list: allow.into_iter().map(String::from).collect(),
            deny_list: deny.into_iter().map(String::from).collect(),
        }
    }

    #[test]
    fn explicit_env_takes_precedence_over_allow_list() {
        let key = "OAB_TEST_PRECEDENCE";
        std::env::set_var(key, "from_process");
        let mut explicit = std::collections::HashMap::new();
        explicit.insert(key.to_string(), "from_config".to_string());
        let clear_env = make_clear_env(true, vec![key], vec![]);

        let (result, inherited) = build_agent_env(&explicit, &clear_env);

        assert_eq!(result.get(key).unwrap(), "from_config");
        assert!(!inherited.contains(&key.to_string()));
        std::env::remove_var(key);
    }

    #[test]
    fn allow_list_copies_from_process() {
        let key = "OAB_TEST_INHERIT";
        std::env::set_var(key, "process_value");
        let explicit = std::collections::HashMap::new();
        let clear_env = make_clear_env(true, vec![key], vec![]);

        let (result, inherited) = build_agent_env(&explicit, &clear_env);

        assert_eq!(result.get(key).unwrap(), "process_value");
        assert!(inherited.contains(&key.to_string()));
        std::env::remove_var(key);
    }

    #[test]
    fn allow_list_skips_missing_vars() {
        let explicit = std::collections::HashMap::new();
        let clear_env = make_clear_env(true, vec!["OAB_TEST_NONEXISTENT_VAR_12345"], vec![]);

        let (result, inherited) = build_agent_env(&explicit, &clear_env);

        assert!(!result.contains_key("OAB_TEST_NONEXISTENT_VAR_12345"));
        assert!(inherited.is_empty());
    }

    #[test]
    fn enabled_true_with_empty_lists_inherits_nothing() {
        // Pure clear mode: enabled=true with both lists empty inherits nothing.
        let key = "OAB_TEST_PURE_CLEAR";
        std::env::set_var(key, "should_not_appear");
        let explicit = std::collections::HashMap::new();
        let clear_env = make_clear_env(true, vec![], vec![]);

        let (result, _inherited) = build_agent_env(&explicit, &clear_env);

        assert!(!result.contains_key(key));
        std::env::remove_var(key);
    }

    #[test]
    fn enabled_false_inherits_full_env_ignoring_lists() {
        // Escape hatch: full inherit, both lists ignored.
        let inherited_key = "OAB_TEST_FULL_INHERIT";
        let allow_only_key = "OAB_TEST_ALLOW_IGNORED";
        let deny_target_key = "OAB_TEST_DENY_IGNORED";
        std::env::set_var(inherited_key, "process_value");
        std::env::set_var(allow_only_key, "value_a");
        std::env::set_var(deny_target_key, "value_d");
        let explicit = std::collections::HashMap::new();
        let clear_env = make_clear_env(false, vec![allow_only_key], vec![deny_target_key]);

        let (result, inherited) = build_agent_env(&explicit, &clear_env);

        // All three are inherited because lists are ignored under enabled=false.
        assert_eq!(result.get(inherited_key).unwrap(), "process_value");
        assert_eq!(result.get(allow_only_key).unwrap(), "value_a");
        assert_eq!(result.get(deny_target_key).unwrap(), "value_d");
        assert!(inherited.contains(&inherited_key.to_string()));
        std::env::remove_var(inherited_key);
        std::env::remove_var(allow_only_key);
        std::env::remove_var(deny_target_key);
    }

    #[test]
    fn deny_list_strips_keys_when_enabled() {
        // deny_list mode: enabled=true, allow_list empty, deny_list non-empty
        // → inherit all process env minus deny_list keys.
        let kept = "OAB_TEST_KEPT";
        let stripped = "OAB_TEST_STRIPPED";
        std::env::set_var(kept, "kept_value");
        std::env::set_var(stripped, "should_be_stripped");
        let explicit = std::collections::HashMap::new();
        let clear_env = make_clear_env(true, vec![], vec![stripped]);

        let (result, _inherited) = build_agent_env(&explicit, &clear_env);

        assert_eq!(result.get(kept).unwrap(), "kept_value");
        assert!(!result.contains_key(stripped));
        std::env::remove_var(kept);
        std::env::remove_var(stripped);
    }

    #[test]
    fn allow_list_takes_priority_over_deny_list_when_both_set() {
        // When allow_list is non-empty under enabled=true, deny_list is
        // ignored entirely (allow-list-only mode).
        let allowed = "OAB_TEST_ALLOWED";
        let other = "OAB_TEST_NOT_ALLOWED";
        let listed_in_deny = "OAB_TEST_LISTED_IN_DENY";
        std::env::set_var(allowed, "allowed_value");
        std::env::set_var(other, "other_value");
        std::env::set_var(listed_in_deny, "deny_value");
        let explicit = std::collections::HashMap::new();
        let clear_env = make_clear_env(true, vec![allowed], vec![listed_in_deny]);

        let (result, _inherited) = build_agent_env(&explicit, &clear_env);

        // Only the allow_list key passes; deny_list and other keys are absent.
        assert_eq!(result.get(allowed).unwrap(), "allowed_value");
        assert!(!result.contains_key(other));
        assert!(!result.contains_key(listed_in_deny));
        std::env::remove_var(allowed);
        std::env::remove_var(other);
        std::env::remove_var(listed_in_deny);
    }
}
