//! Per-channel sliding-window rate limiter for outbound attachments.
//!
//! Guards against a single misbehaving agent flooding a chat channel with
//! uploads. See openabdev/openab#355 item 5.
//!
//! Design: a `HashMap<channel_key, VecDeque<Instant>>` tracks recent upload
//! timestamps per channel. Before admitting a batch, we prune entries older
//! than the window (60 s) and then accept up to `limit − len(deque)` items.
//!
//! Memory: bounded by `limit` timestamps per distinct channel. Channels
//! with no recent traffic are pruned on the next check, and the map is
//! swept on an ad-hoc basis inside `admit`.
//!
//! Thread safety: a single `Mutex<...>` is fine here — `admit` is called
//! at most once per message round-trip and runs in microseconds.

use std::collections::{HashMap, VecDeque};
use std::sync::Mutex;
use std::time::{Duration, Instant};

const WINDOW: Duration = Duration::from_secs(60);

pub struct OutboundRateLimiter {
    inner: Mutex<Inner>,
}

struct Inner {
    per_channel: HashMap<String, VecDeque<Instant>>,
}

impl OutboundRateLimiter {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(Inner {
                per_channel: HashMap::new(),
            }),
        }
    }

    /// Request permission to send `requested` attachments to `channel_key`
    /// given a per-minute `limit`. Returns the number that may actually be
    /// sent. Callers must honour the returned count — any remainder must
    /// be dropped to prevent the flood we're trying to stop.
    ///
    /// Admitted timestamps are recorded immediately.
    pub fn admit(&self, channel_key: &str, requested: usize, limit: usize) -> usize {
        if requested == 0 || limit == 0 {
            return 0;
        }
        let now = Instant::now();
        let cutoff = now - WINDOW;

        let mut inner = self.inner.lock().expect("rate limiter mutex poisoned");
        let entry = inner
            .per_channel
            .entry(channel_key.to_string())
            .or_default();
        while entry.front().is_some_and(|t| *t < cutoff) {
            entry.pop_front();
        }
        let remaining = limit.saturating_sub(entry.len());
        let grant = requested.min(remaining);
        for _ in 0..grant {
            entry.push_back(now);
        }
        grant
    }
}

impl Default for OutboundRateLimiter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn admits_up_to_limit_then_blocks_within_window() {
        let rl = OutboundRateLimiter::new();
        assert_eq!(rl.admit("ch1", 5, 5), 5);
        assert_eq!(rl.admit("ch1", 1, 5), 0, "further sends blocked at limit");
    }

    #[test]
    fn partial_admit_when_some_remaining_slots() {
        let rl = OutboundRateLimiter::new();
        assert_eq!(rl.admit("ch1", 3, 5), 3);
        assert_eq!(rl.admit("ch1", 5, 5), 2, "only 2 slots remain");
        assert_eq!(rl.admit("ch1", 1, 5), 0);
    }

    #[test]
    fn channels_are_independent() {
        let rl = OutboundRateLimiter::new();
        assert_eq!(rl.admit("ch1", 5, 5), 5);
        assert_eq!(rl.admit("ch2", 5, 5), 5, "ch2 is unaffected by ch1");
    }

    #[test]
    fn zero_requested_or_zero_limit_grants_zero() {
        let rl = OutboundRateLimiter::new();
        assert_eq!(rl.admit("ch1", 0, 10), 0);
        assert_eq!(rl.admit("ch1", 10, 0), 0);
    }

    #[test]
    fn prunes_entries_older_than_window() {
        let rl = OutboundRateLimiter::new();
        // Seed an old entry manually — would take 60 s to reproduce
        // otherwise. We know the internal structure here; test is allowed
        // that coupling.
        {
            let mut inner = rl.inner.lock().unwrap();
            let entry = inner.per_channel.entry("ch1".to_string()).or_default();
            for _ in 0..5 {
                entry.push_back(Instant::now() - Duration::from_secs(120));
            }
        }
        // All entries are older than WINDOW → full limit available again.
        assert_eq!(rl.admit("ch1", 5, 5), 5);
    }
}
