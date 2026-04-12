/// Platform-agnostic session key.
///
/// Format: `"{platform}:{thread_id}"`
///
/// Examples:
/// - `"discord:987654321"`        (Discord thread)
/// - `"slack:T01234:thread_ts"`   (Slack thread, future)
/// - `"telegram:-100123:42"`      (Telegram thread, future)
///
/// Thread IDs within Discord are globally unique, so no parent channel is needed.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SessionKey(String);

impl SessionKey {
    pub fn discord(thread_id: u64) -> Self {
        Self(format!("discord:{thread_id}"))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Returns a filesystem-safe version (colons replaced with underscores).
    /// Used as the JSONL transcript filename.
    pub fn to_filename(&self) -> String {
        self.0.replace(':', "_")
    }

    pub fn platform(&self) -> &str {
        self.0.split(':').next().unwrap_or("unknown")
    }
}

impl std::fmt::Display for SessionKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<SessionKey> for String {
    fn from(k: SessionKey) -> Self {
        k.0
    }
}
