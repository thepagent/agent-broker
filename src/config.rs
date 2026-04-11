use regex::Regex;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub telegram: TelegramConfig,
    pub agent: AgentConfig,
    #[serde(default)]
    pub pool: PoolConfig,
}

#[derive(Debug, Deserialize, PartialEq, Clone)]
#[serde(rename_all = "lowercase")]
pub enum ChatMode {
    /// Personal mode (original): any message in #general/All creates a new topic.
    /// Inside topics, all messages get a reply directly — no @mention needed.
    Personal,
    /// Team mode: in #general, plain messages are buffered silently; @mention replies in-place;
    /// `!kiro` creates a topic (restricted to topic_creator_id if set).
    /// Inside topics, Kiro listens silently and only replies when @mentioned.
    Team,
}

impl Default for ChatMode {
    fn default() -> Self { ChatMode::Personal }
}

#[derive(Debug, Deserialize)]
pub struct TelegramConfig {
    pub bot_token: String,
    #[serde(default)]
    pub allowed_users: Vec<i64>,
    /// If set, only this user ID can trigger `!kiro` topic creation (team mode).
    pub topic_creator_id: Option<i64>,
    /// Who can run `!model` / `!cmd`. In team mode defaults to topic_creator_id.
    /// In personal mode this is ignored (allowed_users already gates everything).
    #[serde(default)]
    pub model_admin_ids: Vec<i64>,
    /// `personal` (default) or `team`. See ChatMode for details.
    #[serde(default)]
    pub mode: ChatMode,
}

#[derive(Debug, Deserialize)]
pub struct AgentConfig {
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default = "default_working_dir")]
    pub working_dir: String,
    #[serde(default)]
    pub env: HashMap<String, String>,
    /// MCP servers to pass to kiro-cli on session/new and session/load.
    /// Each entry: { command, args, env }
    /// If empty, kiro uses its own ~/.kiro/mcp.json defaults.
    #[serde(default)]
    pub mcp_servers: Vec<McpServerConfig>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct McpServerConfig {
    pub name: String,
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub env: HashMap<String, String>,
}

#[derive(Debug, Deserialize)]
pub struct PoolConfig {
    #[serde(default = "default_max_sessions")]
    pub max_sessions: usize,
    /// Number of recent user messages to keep in memory per session for crash recovery.
    #[serde(default = "default_crash_history_size")]
    pub crash_history_size: usize,
}

fn default_working_dir() -> String { "/tmp".into() }
fn default_max_sessions() -> usize { 10 }
fn default_crash_history_size() -> usize { 20 }

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            max_sessions: default_max_sessions(),
            crash_history_size: default_crash_history_size(),
        }
    }
}

fn expand_env_vars(raw: &str) -> String {
    let re = Regex::new(r"\$\{(\w+)\}").unwrap();
    re.replace_all(raw, |caps: &regex::Captures| {
        std::env::var(&caps[1]).unwrap_or_default()
    })
    .into_owned()
}

pub fn load_config(path: &Path) -> anyhow::Result<Config> {
    let raw = std::fs::read_to_string(path)
        .map_err(|e| anyhow::anyhow!("failed to read {}: {e}", path.display()))?;
    let expanded = expand_env_vars(&raw);
    let config: Config = toml::from_str(&expanded)
        .map_err(|e| anyhow::anyhow!("failed to parse {}: {e}", path.display()))?;
    Ok(config)
}
