use regex::Regex;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;

/// Controls whether the bot processes messages from other Discord bots.
///
/// Inspired by Hermes Agent's `DISCORD_ALLOW_BOTS` 3-value design:
/// - `Off` (default): ignore all bot messages (safe default, no behavior change)
/// - `Mentions`: only process bot messages that @mention this bot (natural loop breaker)
/// - `All`: process all bot messages (capped at `MAX_CONSECUTIVE_BOT_TURNS`)
///
/// The bot's own messages are always ignored regardless of this setting.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum AllowBots {
    #[default]
    Off,
    Mentions,
    All,
}

impl<'de> Deserialize<'de> for AllowBots {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        match s.to_lowercase().as_str() {
            "off" | "none" | "false" => Ok(Self::Off),
            "mentions" => Ok(Self::Mentions),
            "all" | "true" => Ok(Self::All),
            other => Err(serde::de::Error::unknown_variant(other, &["off", "mentions", "all"])),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct Config {
    pub discord: Option<DiscordConfig>,
    pub slack: Option<SlackConfig>,
    pub agent: AgentConfig,
    #[serde(default)]
    pub pool: PoolConfig,
    #[serde(default)]
    pub reactions: ReactionsConfig,
    #[serde(default)]
    pub stt: SttConfig,
    #[serde(default)]
    pub outbound: OutboundConfig,
}

/// Controls outbound file attachments — the `![alt](/path)` markdown marker
/// in agent responses that instructs the bot to upload a local file as a
/// native chat attachment. Disabled by default; operators must explicitly
/// opt in because this opens a path from the host filesystem to the chat
/// channel.
///
/// See openabdev/openab#298 for the feature rationale and openabdev/openab#355
/// for the security requirements this config implements.
#[derive(Debug, Clone, Deserialize)]
pub struct OutboundConfig {
    /// Master switch. Defaults to `false` so shipping this feature cannot
    /// surprise existing deployments.
    #[serde(default)]
    pub enabled: bool,
    /// Directories from which agents may send files. An outbound path must
    /// canonicalize (symlinks + `..` resolved) to live under one of these
    /// prefixes. Defaults to `["/tmp/", "/var/folders/"]` to preserve
    /// behavior for operators upgrading from the prior hard-coded list.
    #[serde(default = "default_outbound_allowed_dirs")]
    pub allowed_dirs: Vec<String>,
    /// Cap on file size per attachment, in megabytes. Discord's native
    /// upload limit is 25 MB; Slack is 1 GB. Default matches Discord so
    /// the feature is platform-safe out of the box.
    #[serde(default = "default_outbound_max_size_mb")]
    pub max_file_size_mb: u64,
    /// Cap on attachments per single agent response. Guards against a single
    /// agent message fanning out into hundreds of uploads.
    #[serde(default = "default_outbound_max_per_message")]
    pub max_per_message: usize,
    /// Sliding-window cap on attachments per channel per minute. Guards
    /// against a malfunctioning agent flooding a channel.
    #[serde(default = "default_outbound_max_per_minute")]
    pub max_per_minute_per_channel: usize,
}

impl Default for OutboundConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            allowed_dirs: default_outbound_allowed_dirs(),
            max_file_size_mb: default_outbound_max_size_mb(),
            max_per_message: default_outbound_max_per_message(),
            max_per_minute_per_channel: default_outbound_max_per_minute(),
        }
    }
}

impl OutboundConfig {
    /// Return the size cap as bytes for internal comparison. Config is
    /// expressed in MB for human ergonomics; callers that need to compare
    /// against `std::fs::Metadata::len()` use this.
    pub fn max_size_bytes(&self) -> u64 {
        self.max_file_size_mb.saturating_mul(1024 * 1024)
    }
}

fn default_outbound_allowed_dirs() -> Vec<String> {
    vec!["/tmp/".into(), "/var/folders/".into()]
}
fn default_outbound_max_size_mb() -> u64 {
    25
}
fn default_outbound_max_per_message() -> usize {
    10
}
fn default_outbound_max_per_minute() -> usize {
    30
}

#[derive(Debug, Clone, Deserialize)]
pub struct SttConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub api_key: String,
    #[serde(default = "default_stt_model")]
    pub model: String,
    #[serde(default = "default_stt_base_url")]
    pub base_url: String,
}

impl Default for SttConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            api_key: String::new(),
            model: default_stt_model(),
            base_url: default_stt_base_url(),
        }
    }
}

fn default_stt_model() -> String { "whisper-large-v3-turbo".into() }
fn default_stt_base_url() -> String { "https://api.groq.com/openai/v1".into() }

#[derive(Debug, Deserialize)]
pub struct DiscordConfig {
    pub bot_token: String,
    #[serde(default)]
    pub allowed_channels: Vec<String>,
    #[serde(default)]
    pub allowed_users: Vec<String>,
    #[serde(default)]
    pub allow_bot_messages: AllowBots,
    /// When non-empty, only bot messages from these IDs pass the bot gate.
    /// Combines with `allow_bot_messages`: the mode check runs first, then
    /// the allowlist filters further. Empty = allow any bot (mode permitting).
    /// Only relevant when `allow_bot_messages` is `"mentions"` or `"all"`;
    /// ignored when `"off"` since all bot messages are rejected before this check.
    #[serde(default)]
    pub trusted_bot_ids: Vec<String>,
    #[serde(default)]
    pub allow_user_messages: AllowUsers,
}

/// Controls whether the bot responds to user messages in threads without @mention.
///
/// - `Involved` (default): respond to thread messages only if the bot has participated
///   in the thread (posted at least one message, or the thread parent @mentions the bot).
///   Channel/MPDM messages always require @mention. DMs always process (implicit mention).
/// - `Mentions`: always require @mention, even in threads the bot is participating in.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum AllowUsers {
    #[default]
    Involved,
    Mentions,
}

impl<'de> Deserialize<'de> for AllowUsers {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        match s.to_lowercase().as_str() {
            "involved" => Ok(Self::Involved),
            "mentions" => Ok(Self::Mentions),
            other => Err(serde::de::Error::unknown_variant(other, &["involved", "mentions"])),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct SlackConfig {
    pub bot_token: String,
    pub app_token: String,
    #[serde(default)]
    pub allowed_channels: Vec<String>,
    #[serde(default)]
    pub allowed_users: Vec<String>,
    #[serde(default)]
    pub allow_bot_messages: AllowBots,
    /// Bot User IDs (U...) allowed to interact when allow_bot_messages is
    /// "mentions" or "all". Find via Slack UI: click bot profile → Copy member ID.
    /// Empty = allow any bot (mode permitting).
    #[serde(default)]
    pub trusted_bot_ids: Vec<String>,
    #[serde(default)]
    pub allow_user_messages: AllowUsers,
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
}

#[derive(Debug, Deserialize)]
pub struct PoolConfig {
    #[serde(default = "default_max_sessions")]
    pub max_sessions: usize,
    #[serde(default = "default_ttl_hours")]
    pub session_ttl_hours: u64,
}

#[derive(Debug, Deserialize)]
pub struct ReactionsConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub remove_after_reply: bool,
    #[serde(default)]
    pub emojis: ReactionEmojis,
    #[serde(default)]
    pub timing: ReactionTiming,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ReactionEmojis {
    #[serde(default = "emoji_queued")]
    pub queued: String,
    #[serde(default = "emoji_thinking")]
    pub thinking: String,
    #[serde(default = "emoji_tool")]
    pub tool: String,
    #[serde(default = "emoji_coding")]
    pub coding: String,
    #[serde(default = "emoji_web")]
    pub web: String,
    #[serde(default = "emoji_done")]
    pub done: String,
    #[serde(default = "emoji_error")]
    pub error: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ReactionTiming {
    #[serde(default = "default_debounce_ms")]
    pub debounce_ms: u64,
    #[serde(default = "default_stall_soft_ms")]
    pub stall_soft_ms: u64,
    #[serde(default = "default_stall_hard_ms")]
    pub stall_hard_ms: u64,
    #[serde(default = "default_done_hold_ms")]
    pub done_hold_ms: u64,
    #[serde(default = "default_error_hold_ms")]
    pub error_hold_ms: u64,
}

// --- defaults ---

fn default_working_dir() -> String { "/tmp".into() }
fn default_max_sessions() -> usize { 10 }
fn default_ttl_hours() -> u64 { 4 }
fn default_true() -> bool { true }

fn emoji_queued() -> String { "👀".into() }
fn emoji_thinking() -> String { "🤔".into() }
fn emoji_tool() -> String { "🔥".into() }
fn emoji_coding() -> String { "👨‍💻".into() }
fn emoji_web() -> String { "⚡".into() }
fn emoji_done() -> String { "🆗".into() }
fn emoji_error() -> String { "😱".into() }

fn default_debounce_ms() -> u64 { 700 }
fn default_stall_soft_ms() -> u64 { 10_000 }
fn default_stall_hard_ms() -> u64 { 30_000 }
fn default_done_hold_ms() -> u64 { 1_500 }
fn default_error_hold_ms() -> u64 { 2_500 }

impl Default for PoolConfig {
    fn default() -> Self {
        Self { max_sessions: default_max_sessions(), session_ttl_hours: default_ttl_hours() }
    }
}

impl Default for ReactionsConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            remove_after_reply: false,
            emojis: ReactionEmojis::default(),
            timing: ReactionTiming::default(),
        }
    }
}

impl Default for ReactionEmojis {
    fn default() -> Self {
        Self {
            queued: emoji_queued(), thinking: emoji_thinking(), tool: emoji_tool(),
            coding: emoji_coding(), web: emoji_web(), done: emoji_done(), error: emoji_error(),
        }
    }
}

impl Default for ReactionTiming {
    fn default() -> Self {
        Self {
            debounce_ms: default_debounce_ms(), stall_soft_ms: default_stall_soft_ms(),
            stall_hard_ms: default_stall_hard_ms(), done_hold_ms: default_done_hold_ms(),
            error_hold_ms: default_error_hold_ms(),
        }
    }
}

// --- loading ---

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
