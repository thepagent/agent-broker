use crate::acp::connection::AcpConnection;
use crate::config::AgentConfig;
use crate::session::{SessionMeta, SessionStore};
use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::Instant;
use tracing::{info, warn};

pub struct SessionPool {
    connections: RwLock<HashMap<String, AcpConnection>>,
    config: AgentConfig,
    max_sessions: usize,
    store: Arc<SessionStore>,
}

impl SessionPool {
    pub fn new(config: AgentConfig, max_sessions: usize, store: Arc<SessionStore>) -> Self {
        Self {
            connections: RwLock::new(HashMap::new()),
            config,
            max_sessions,
            store,
        }
    }

    /// Return a reference to the session store (used by the platform adapter to
    /// record transcript entries).
    pub fn store(&self) -> &Arc<SessionStore> {
        &self.store
    }

    pub async fn get_or_create(&self, session_key: &str) -> Result<()> {
        // ── fast path: alive connection already in memory ────────────────────
        {
            let conns = self.connections.read().await;
            if let Some(conn) = conns.get(session_key) {
                if conn.alive() {
                    return Ok(());
                }
            }
        }

        // ── check persistent store before acquiring the write lock ───────────
        // Load metadata + transcript while we're NOT holding the lock, so the
        // (potentially slow) file I/O doesn't block other readers.
        let all_meta = self.store.load_all().await;
        let stored_meta = all_meta.get(session_key).cloned();
        let transcript = if stored_meta.is_some() {
            self.store.load_transcript(session_key).await
        } else {
            vec![]
        };

        // ── acquire write lock and create / rebuild ──────────────────────────
        let mut conns = self.connections.write().await;

        // Double-check: another task may have created the connection while we
        // were loading from disk.
        if let Some(conn) = conns.get(session_key) {
            if conn.alive() {
                return Ok(());
            }
            warn!(session_key, "stale connection, rebuilding");
            conns.remove(session_key);
        }

        if conns.len() >= self.max_sessions {
            return Err(anyhow!("pool exhausted ({} sessions)", self.max_sessions));
        }

        let is_restore = stored_meta.is_some();

        let mut conn = AcpConnection::spawn(
            &self.config.command,
            &self.config.args,
            &self.config.working_dir,
            &self.config.env,
        )
        .await?;

        conn.initialize().await?;
        conn.session_new(&self.config.working_dir).await?;

        // If restoring an existing session, replay history into the agent so it
        // has context about the previous conversation.
        if !transcript.is_empty() {
            let history: Vec<(String, String)> = transcript
                .into_iter()
                .map(|e| (e.role, e.content))
                .collect();
            if let Err(e) = conn.session_prime_context(&history).await {
                warn!(error = %e, session_key, "failed to prime context; continuing without history");
            } else {
                info!(session_key, messages = history.len(), "context primed from transcript");
            }
        }

        // Mark the connection so the platform adapter can show a "restored" or
        // "starting fresh" notice to the user.
        conn.session_reset = is_restore;

        // Persist metadata (create or update last_active).
        let now = now_secs();
        let meta = SessionMeta {
            key: session_key.to_string(),
            platform: session_key.split(':').next().unwrap_or("unknown").to_string(),
            agent: self.config.command.clone(),
            created_at: stored_meta.map(|m| m.created_at).unwrap_or(now),
            last_active: now,
        };
        if let Err(e) = self.store.upsert(meta).await {
            warn!(error = %e, session_key, "failed to persist session metadata");
        }

        conns.insert(session_key.to_string(), conn);
        Ok(())
    }

    /// Get mutable access to a connection via a closure.
    /// The caller must have called `get_or_create` first.
    pub async fn with_connection<F, R>(&self, session_key: &str, f: F) -> Result<R>
    where
        F: FnOnce(&mut AcpConnection) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<R>> + Send + '_>>,
    {
        let mut conns = self.connections.write().await;
        let conn = conns
            .get_mut(session_key)
            .ok_or_else(|| anyhow!("no connection for session {session_key}"))?;
        f(conn).await
    }

    pub async fn cleanup_idle(&self, ttl_secs: u64) {
        let cutoff = Instant::now() - std::time::Duration::from_secs(ttl_secs);
        let stale: Vec<String> = {
            let conns = self.connections.read().await;
            conns
                .iter()
                .filter(|(_, c)| c.last_active < cutoff || !c.alive())
                .map(|(k, _)| k.clone())
                .collect()
        };

        if stale.is_empty() {
            return;
        }

        let mut conns = self.connections.write().await;
        for key in stale {
            info!(session_key = %key, "cleaning up idle session");
            conns.remove(&key);
            // Child process killed via kill_on_drop when AcpConnection drops.
            // Remove from persistent store so it is not "restored" after cleanup.
            if let Err(e) = self.store.remove(&key).await {
                warn!(error = %e, session_key = %key, "failed to remove session from store");
            }
        }
    }

    pub async fn shutdown(&self) {
        let mut conns = self.connections.write().await;
        let count = conns.len();
        conns.clear(); // kill_on_drop handles process cleanup
        info!(count, "pool shutdown complete");
    }
}

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
