use crate::acp::connection::{AcpConnection, ModelInfo};
use crate::config::AgentConfig;
use anyhow::{anyhow, Result};
use std::collections::HashMap;
use tokio::sync::RwLock;
use tokio::time::Instant;
use tracing::{info, warn};

pub struct SessionPool {
    connections: RwLock<HashMap<String, AcpConnection>>,
    config: AgentConfig,
    max_sessions: usize,
    /// Snapshot of available models from the most recent session creation.
    /// Populated on first session_new() so slash command autocomplete can serve
    /// suggestions without spawning a fresh agent (which takes ~10s).
    cached_models: RwLock<Vec<ModelInfo>>,
    cached_current_model: RwLock<String>,
}

impl SessionPool {
    pub fn new(config: AgentConfig, max_sessions: usize) -> Self {
        Self {
            connections: RwLock::new(HashMap::new()),
            config,
            max_sessions,
            cached_models: RwLock::new(Vec::new()),
            cached_current_model: RwLock::new("auto".to_string()),
        }
    }

    pub async fn cached_models(&self) -> Vec<ModelInfo> {
        self.cached_models.read().await.clone()
    }

    pub async fn cached_current_model(&self) -> String {
        self.cached_current_model.read().await.clone()
    }

    /// Replace the cached model list. Used by the background refresh task
    /// in `main.rs` to keep `/model` autocomplete in sync with Copilot's
    /// actual available models (handles models added/removed upstream).
    pub async fn set_cached_models(&self, models: Vec<ModelInfo>) {
        if !models.is_empty() {
            *self.cached_models.write().await = models;
        }
    }

    pub async fn get_or_create(&self, thread_id: &str) -> Result<()> {
        // Check if alive connection exists
        {
            let conns = self.connections.read().await;
            if let Some(conn) = conns.get(thread_id) {
                if conn.alive() {
                    return Ok(());
                }
            }
        }

        // Need to create or rebuild
        let mut conns = self.connections.write().await;

        // Double-check after acquiring write lock
        if let Some(conn) = conns.get(thread_id) {
            if conn.alive() {
                return Ok(());
            }
            warn!(thread_id, "stale connection, rebuilding");
            conns.remove(thread_id);
        }

        if conns.len() >= self.max_sessions {
            return Err(anyhow!("pool exhausted ({} sessions)", self.max_sessions));
        }

        let mut conn = AcpConnection::spawn(
            &self.config.command,
            &self.config.args,
            &self.config.working_dir,
            &self.config.env,
        )
        .await?;

        conn.initialize().await?;
        conn.session_new(&self.config.working_dir).await?;

        // Refresh model cache snapshot for slash command autocomplete.
        if !conn.available_models.is_empty() {
            *self.cached_models.write().await = conn.available_models.clone();
        }
        *self.cached_current_model.write().await = conn.current_model.clone();

        let is_rebuild = conns.contains_key(thread_id);
        if is_rebuild {
            conn.session_reset = true;
        }

        conns.insert(thread_id.to_string(), conn);
        Ok(())
    }

    /// Get mutable access to a connection. Caller must have called get_or_create first.
    pub async fn with_connection<F, R>(&self, thread_id: &str, f: F) -> Result<R>
    where
        F: FnOnce(&mut AcpConnection) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<R>> + Send + '_>>,
    {
        let mut conns = self.connections.write().await;
        let conn = conns
            .get_mut(thread_id)
            .ok_or_else(|| anyhow!("no connection for thread {thread_id}"))?;
        f(conn).await
    }

    /// Drop a session for a specific thread. Returns true if one was removed.
    /// The underlying child process is killed via `kill_on_drop`.
    pub async fn drop_session(&self, thread_id: &str) -> bool {
        let mut conns = self.connections.write().await;
        conns.remove(thread_id).is_some()
    }

    pub async fn cleanup_idle(&self, ttl_secs: u64) {
        let now = Instant::now();
        let ttl = std::time::Duration::from_secs(ttl_secs);
        let mut conns = self.connections.write().await;
        let stale: Vec<String> = conns
            .iter()
            .filter(|(_, c)| now.saturating_duration_since(c.last_active) >= ttl || !c.alive())
            .map(|(k, _)| k.clone())
            .collect();
        for key in stale {
            info!(thread_id = %key, "cleaning up idle session");
            conns.remove(&key);
            // Child process killed via kill_on_drop when AcpConnection drops
        }
    }

    pub async fn shutdown(&self) {
        let mut conns = self.connections.write().await;
        let count = conns.len();
        conns.clear(); // kill_on_drop handles process cleanup
        info!(count, "pool shutdown complete");
    }
}
