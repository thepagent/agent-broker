use crate::acp::connection::AcpConnection;
use crate::config::AgentConfig;
use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use tokio::time::Instant;
use tracing::{info, warn};

pub struct SessionPool {
    connections: RwLock<HashMap<String, Arc<Mutex<AcpConnection>>>>,
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

    pub async fn get_or_create(&self, thread_id: &str) -> Result<()> {
        // Fast path: alive connection exists — only the read lock is needed.
        {
            let conns = self.connections.read().await;
            if let Some(conn_arc) = conns.get(thread_id) {
                if conn_arc.lock().await.alive() {
                    return Ok(());
                }
            }
        }

        // Slow path: create or rebuild.
        let mut conns = self.connections.write().await;

        // Double-check after acquiring the write lock.
        if let Some(conn_arc) = conns.get(thread_id) {
            if conn_arc.lock().await.alive() {
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

        let is_rebuild = conns.contains_key(thread_id);
        if is_rebuild {
            conn.session_reset = true;
        }

        conns.insert(thread_id.to_string(), Arc::new(Mutex::new(conn)));
        Ok(())
    }

    /// Run `f` against a mutable connection reference. Only this connection's
    /// per-session mutex is held for the callback's duration — the pool lock
    /// is released immediately, so concurrent sessions are not blocked.
    /// Caller must have called `get_or_create` first.
    pub async fn with_connection<F, R>(&self, thread_id: &str, f: F) -> Result<R>
    where
        F: FnOnce(&mut AcpConnection) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<R>> + Send + '_>>,
    {
        let conn_arc = {
            let conns = self.connections.read().await;
            conns
                .get(thread_id)
                .cloned()
                .ok_or_else(|| anyhow!("no connection for thread {thread_id}"))?
        };
        let mut conn = conn_arc.lock().await;
        f(&mut conn).await
    }

    pub async fn cleanup_idle(&self, ttl_secs: u64) {
        let cutoff = Instant::now() - std::time::Duration::from_secs(ttl_secs);

        // Snapshot the Arcs under the read lock, then release it before
        // awaiting any per-connection mutex. Otherwise a long-running
        // `session_prompt` would block `cleanup_idle` on the connection
        // mutex while it still held the pool write lock, re-introducing
        // exactly the starvation this refactor is meant to eliminate.
        let snapshot: Vec<(String, Arc<Mutex<AcpConnection>>)> = {
            let conns = self.connections.read().await;
            conns.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
        };

        // Probe each connection under its own mutex. `try_lock` skips
        // connections that are currently in use — they are by definition
        // not idle, so there is nothing to clean up for them this round.
        let mut stale = Vec::new();
        for (key, conn_arc) in &snapshot {
            let Ok(conn) = conn_arc.try_lock() else { continue };
            if conn.last_active < cutoff || !conn.alive() {
                stale.push(key.clone());
            }
        }

        if stale.is_empty() {
            return;
        }

        // Only now take the pool write lock to remove the stale entries.
        let mut conns = self.connections.write().await;
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
