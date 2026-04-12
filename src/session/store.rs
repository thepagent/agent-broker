//! File-based session persistence.
//!
//! Layout under `base_dir` (default `/data/sessions`):
//!
//! ```text
//! /data/sessions/
//! ├── index.json              ← all session metadata (atomic-write)
//! ├── discord_123456.jsonl    ← transcript for session "discord:123456"
//! └── discord_789012.jsonl
//! ```
//!
//! The JSONL transcript files are append-only, making them crash-safe: a partial
//! write only corrupts the last line, which `load_transcript` silently skips.
//!
//! `index.json` is written atomically (write-to-tmp + rename) so it is never
//! left in a half-written state after a crash or restart.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use tokio::io::AsyncWriteExt;
use tracing::warn;

/// Metadata for a single session, persisted in `index.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMeta {
    /// Full session key, e.g. `"discord:987654321"`.
    pub key: String,
    /// Platform name extracted from the key, e.g. `"discord"`.
    pub platform: String,
    /// Agent command used for this session, e.g. `"claude-code"`.
    pub agent: String,
    /// Unix timestamp (seconds) when the session was first created.
    pub created_at: u64,
    /// Unix timestamp (seconds) of the last activity.
    pub last_active: u64,
}

/// A single message in the transcript.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptEntry {
    /// `"user"` or `"assistant"`.
    pub role: String,
    /// Plain-text content of the message.
    pub content: String,
    /// Unix timestamp (seconds).
    pub ts: u64,
}

/// Maximum number of transcript entries returned for context restoration.
/// Older messages beyond this limit are ignored to keep the context prompt short.
const MAX_RESTORE_ENTRIES: usize = 20;

#[derive(Serialize, Deserialize, Default)]
struct Index {
    sessions: HashMap<String, SessionMeta>,
}

pub struct SessionStore {
    base_dir: PathBuf,
}

impl SessionStore {
    pub fn new(base_dir: impl Into<PathBuf>) -> Self {
        Self { base_dir: base_dir.into() }
    }

    /// Create the storage directory if it does not already exist.
    pub async fn init(&self) -> anyhow::Result<()> {
        tokio::fs::create_dir_all(&self.base_dir).await?;
        Ok(())
    }

    // ── index helpers ────────────────────────────────────────────────────────

    fn index_path(&self) -> PathBuf {
        self.base_dir.join("index.json")
    }

    fn transcript_path(&self, key: &str) -> PathBuf {
        // Replace colons so the key is a valid filename on all platforms.
        let filename = key.replace(':', "_");
        self.base_dir.join(format!("{filename}.jsonl"))
    }

    async fn load_index(&self) -> Index {
        match tokio::fs::read_to_string(self.index_path()).await {
            Ok(s) => serde_json::from_str(&s).unwrap_or_default(),
            Err(_) => Index::default(),
        }
    }

    async fn write_index(&self, idx: &Index) -> anyhow::Result<()> {
        let content = serde_json::to_string_pretty(idx)?;
        // Atomic write: write to a temp file then rename.
        let tmp = self.index_path().with_extension("tmp");
        tokio::fs::write(&tmp, content.as_bytes()).await?;
        tokio::fs::rename(&tmp, self.index_path()).await?;
        Ok(())
    }

    // ── public API ───────────────────────────────────────────────────────────

    /// Return metadata for all known sessions.
    pub async fn load_all(&self) -> HashMap<String, SessionMeta> {
        self.load_index().await.sessions
    }

    /// Insert or update session metadata in `index.json`.
    pub async fn upsert(&self, meta: SessionMeta) -> anyhow::Result<()> {
        let mut idx = self.load_index().await;
        idx.sessions.insert(meta.key.clone(), meta);
        self.write_index(&idx).await
    }

    /// Remove a session from `index.json` and delete its transcript file.
    pub async fn remove(&self, key: &str) -> anyhow::Result<()> {
        let mut idx = self.load_index().await;
        idx.sessions.remove(key);
        self.write_index(&idx).await?;
        let _ = tokio::fs::remove_file(self.transcript_path(key)).await;
        Ok(())
    }

    /// Append a single message line to the session's JSONL transcript.
    ///
    /// The file is created if it does not exist. Because lines are appended
    /// one at a time, a crash can only corrupt the last (incomplete) line,
    /// which `load_transcript` will silently skip.
    pub async fn append_message(&self, key: &str, role: &str, content: &str) -> anyhow::Result<()> {
        let entry = TranscriptEntry {
            role: role.to_string(),
            content: content.to_string(),
            ts: now_secs(),
        };
        let mut line = serde_json::to_string(&entry)?;
        line.push('\n');

        let mut file = tokio::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(self.transcript_path(key))
            .await?;
        file.write_all(line.as_bytes()).await?;
        Ok(())
    }

    /// Load the most recent transcript entries for context restoration.
    ///
    /// Returns at most [`MAX_RESTORE_ENTRIES`] entries (the tail of the file),
    /// keeping the context prompt small enough to avoid timeouts.
    pub async fn load_transcript(&self, key: &str) -> Vec<TranscriptEntry> {
        match tokio::fs::read_to_string(self.transcript_path(key)).await {
            Ok(s) => {
                let all: Vec<TranscriptEntry> = s
                    .lines()
                    .filter_map(|l| serde_json::from_str(l).ok())
                    .collect();
                // Return only the tail so context restoration stays concise.
                let skip = all.len().saturating_sub(MAX_RESTORE_ENTRIES);
                all.into_iter().skip(skip).collect()
            }
            Err(_) => vec![],
        }
    }
}

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
