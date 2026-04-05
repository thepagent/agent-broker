use crate::acp::connection::AcpConnection;
use crate::config::AgentConfig;
use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::path::PathBuf;
use tokio::sync::RwLock;
use tokio::time::Instant;
use tracing::{info, warn};

pub struct SessionPool {
    connections: RwLock<HashMap<String, AcpConnection>>,
    config: AgentConfig,
    max_sessions: usize,
}

impl SessionPool {
    pub fn new(config: AgentConfig, max_sessions: usize) -> Self {
        Self {
            connections: RwLock::new(HashMap::new()),
            config,
            max_sessions,
        }
    }

    /// Build the per-thread working directory path.
    fn thread_dir(&self, thread_id: &str) -> PathBuf {
        [&self.config.working_dir, "sessions", thread_id]
            .iter()
            .collect()
    }

    pub async fn get_or_create(&self, thread_id: &str) -> Result<()> {
        // Validate thread_id to prevent path traversal — Discord snowflake IDs
        // are numeric, so reject anything that isn't pure ASCII digits.
        if !thread_id.chars().all(|c| c.is_ascii_digit()) {
            return Err(anyhow!("invalid thread_id: {thread_id}"));
        }
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

        // Create a per-thread working directory so concurrent sessions don't interfere
        let thread_dir = self.thread_dir(thread_id);
        tokio::fs::create_dir_all(&thread_dir).await?;
        let thread_dir_str = thread_dir.to_string_lossy().to_string();

        let mut conn = AcpConnection::spawn(
            &self.config.command,
            &self.config.args,
            &thread_dir_str,
            &self.config.env,
        )
        .await?;

        conn.initialize().await?;
        conn.session_new(&thread_dir_str).await?;

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

    pub async fn cleanup_idle(&self, ttl_secs: u64) {
        let cutoff = Instant::now() - std::time::Duration::from_secs(ttl_secs);
        let mut conns = self.connections.write().await;
        let stale: Vec<String> = conns
            .iter()
            .filter(|(_, c)| c.last_active < cutoff || !c.alive())
            .map(|(k, _)| k.clone())
            .collect();
        for key in stale {
            info!(thread_id = %key, "cleaning up idle session");
            conns.remove(&key);
            // Child process killed via kill_on_drop when AcpConnection drops

            // Clean up the per-thread working directory
            let thread_dir = self.thread_dir(&key);
            match tokio::fs::remove_dir_all(&thread_dir).await {
                Ok(_) => {}
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(e) => warn!(thread_id = %key, error = %e, "failed to remove session directory"),
            }
        }
    }

    pub async fn shutdown(&self) {
        let mut conns = self.connections.write().await;
        let count = conns.len();
        // Clean up per-thread session directories before dropping connections
        for key in conns.keys().cloned().collect::<Vec<_>>() {
            let dir = self.thread_dir(&key);
            match tokio::fs::remove_dir_all(&dir).await {
                Ok(_) => {}
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(e) => warn!(thread_id = %key, error = %e, "failed to remove session directory"),
            }
        }
        conns.clear(); // kill_on_drop handles process cleanup
        info!(count, "pool shutdown complete");
    }
}
