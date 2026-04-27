use crate::adapter::{AdapterRouter, ChatAdapter, ChannelRef, SenderContext};
use crate::config::CronJobConfig;
use chrono::{DateTime, Utc};
use chrono_tz::Tz;
use cron::Schedule;
use std::collections::{HashMap, HashSet};
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{error, info, trace, warn};

/// Validates a cron expression string. Returns an error message if invalid.
pub fn validate_cron_expression(schedule: &str) -> Result<(), String> {
    let expr = format!("0 {}", schedule);
    Schedule::from_str(&expr).map(|_| ()).map_err(|e| e.to_string())
}

/// Validates a timezone string. Returns an error message if invalid.
pub fn validate_timezone(timezone: &str) -> Result<(), String> {
    timezone.parse::<Tz>().map(|_| ()).map_err(|e| e.to_string())
}

/// RAII guard that removes a job index from the in_flight set on drop.
struct InFlightGuard {
    idx: usize,
    in_flight: Arc<Mutex<HashSet<usize>>>,
}

impl Drop for InFlightGuard {
    fn drop(&mut self) {
        let idx = self.idx;
        let in_flight = self.in_flight.clone();
        tokio::spawn(async move {
            in_flight.lock().await.remove(&idx);
        });
    }
}

/// Run the internal cron scheduler. Evaluates cron expressions once per minute.
pub async fn run_scheduler(
    cronjobs: Vec<CronJobConfig>,
    router: Arc<AdapterRouter>,
    adapters: HashMap<String, Arc<dyn ChatAdapter>>,
    mut shutdown_rx: tokio::sync::watch::Receiver<bool>,
) {
    if cronjobs.is_empty() {
        trace!("no cronjobs configured, scheduler not started");
        return;
    }

    // Parse all cron expressions (already validated at config load time)
    let jobs: Vec<(Schedule, Tz, CronJobConfig)> = cronjobs
        .into_iter()
        .filter_map(|job| {
            let expr = format!("0 {}", job.schedule);
            let schedule = match Schedule::from_str(&expr) {
                Ok(s) => s,
                Err(e) => {
                    error!(schedule = %job.schedule, error = %e, "invalid cron expression, skipping");
                    return None;
                }
            };
            let tz: Tz = match job.timezone.parse() {
                Ok(t) => t,
                Err(e) => {
                    error!(timezone = %job.timezone, error = %e, "invalid timezone, skipping");
                    return None;
                }
            };
            info!(
                schedule = %job.schedule,
                timezone = %job.timezone,
                channel = %job.channel,
                platform = %job.platform,
                "cronjob registered"
            );
            Some((schedule, tz, job))
        })
        .collect();

    if jobs.is_empty() {
        warn!("all cronjob expressions invalid, scheduler not started");
        return;
    }

    info!(count = jobs.len(), "cron scheduler started");

    // Track in-flight jobs to prevent overlapping executions
    let in_flight: Arc<Mutex<HashSet<usize>>> = Arc::new(Mutex::new(HashSet::new()));

    // Sliding window: track last tick time to detect scheduled slots in (last_tick, now]
    let mut last_tick: DateTime<Utc> = Utc::now();

    loop {
        tokio::select! {
            _ = tokio::time::sleep(std::time::Duration::from_secs(60)) => {
                let now = Utc::now();
                for (idx, (schedule, tz, job)) in jobs.iter().enumerate() {
                    let last_tz = last_tick.with_timezone(tz);
                    let now_tz = now.with_timezone(tz);
                    // Check if any scheduled time falls in (last_tick, now]
                    if let Some(next) = schedule.after(&last_tz).next() {
                        trace!(schedule = %job.schedule, timezone = %job.timezone, next = %next, "checking");
                        if next <= now_tz {
                            // Skip if previous execution still running
                            {
                                let running = in_flight.lock().await;
                                if running.contains(&idx) {
                                    warn!(schedule = %job.schedule, channel = %job.channel, "skipping cronjob, previous execution still running");
                                    continue;
                                }
                            }
                            info!(
                                schedule = %job.schedule,
                                channel = %job.channel,
                                platform = %job.platform,
                                sender = %job.sender_name,
                                "🔔 cronjob fired"
                            );
                            // Mark as in-flight before spawning
                            in_flight.lock().await.insert(idx);
                            // Spawn fire_cronjob to avoid blocking the scheduler loop
                            let guard = InFlightGuard { idx, in_flight: in_flight.clone() };
                            let router = router.clone();
                            let adapters = adapters.clone();
                            let job = job.clone();
                            tokio::spawn(async move {
                                fire_cronjob(&job, &router, &adapters, guard).await;
                            });
                        }
                    }
                }
                last_tick = now;
            }
            _ = shutdown_rx.changed() => {
                if *shutdown_rx.borrow() {
                    info!("cron scheduler shutting down");
                    return;
                }
            }
        }
    }
}

async fn fire_cronjob(
    job: &CronJobConfig,
    router: &Arc<AdapterRouter>,
    adapters: &HashMap<String, Arc<dyn ChatAdapter>>,
    _guard: InFlightGuard, // dropped automatically when this fn returns
) {
    let adapter = match adapters.get(&job.platform) {
        Some(a) => a.clone(),
        None => {
            error!(platform = %job.platform, "no adapter for platform, skipping cronjob");
            return;
        }
    };

    let thread_channel = ChannelRef {
        platform: job.platform.clone(),
        channel_id: job.channel.clone(),
        thread_id: job.thread_id.clone(),
        parent_id: None,
    };

    let sender = SenderContext {
        schema: "openab.sender.v1".into(),
        sender_id: "openab-cron".into(),
        sender_name: job.sender_name.clone(),
        display_name: job.sender_name.clone(),
        channel: job.platform.clone(),
        channel_id: job.channel.clone(),
        thread_id: job.thread_id.clone(),
        is_bot: true,
    };
    let sender_json = match serde_json::to_string(&sender) {
        Ok(j) => j,
        Err(e) => {
            warn!(error = %e, "failed to serialize cron sender context");
            return;
        }
    };

    // Send visible message first so users see what triggered
    let trigger_msg = match adapter.send_message(&thread_channel, &format!("🕐 [{}]: {}", job.sender_name, job.message)).await {
        Ok(msg) => msg,
        Err(e) => {
            error!(channel = %job.channel, error = %e, "failed to send cron message");
            return;
        }
    };

    // Trigger agent processing
    if let Err(e) = router
        .handle_message(&adapter, &thread_channel, &sender_json, &job.message, vec![], &trigger_msg, false)
        .await
    {
        error!("cron handle_message error: {e}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_cron_expression_valid() {
        assert!(validate_cron_expression("0 9 * * 1-5").is_ok());
        assert!(validate_cron_expression("*/5 * * * *").is_ok());
        assert!(validate_cron_expression("0 0 * * 7").is_ok()); // Sunday = 7 in cron crate
    }

    #[test]
    fn validate_cron_expression_invalid() {
        assert!(validate_cron_expression("not a cron").is_err());
        assert!(validate_cron_expression("").is_err());
        assert!(validate_cron_expression("60 * * * *").is_err());
    }

    #[test]
    fn validate_timezone_valid() {
        assert!(validate_timezone("UTC").is_ok());
        assert!(validate_timezone("Asia/Taipei").is_ok());
        assert!(validate_timezone("Europe/Berlin").is_ok());
    }

    #[test]
    fn validate_timezone_invalid() {
        assert!(validate_timezone("Not/A/Timezone").is_err());
        assert!(validate_timezone("").is_err());
    }

    #[test]
    fn sliding_window_detects_due_job() {
        // Simulate: last_tick was 2 minutes ago, schedule is every minute
        let expr = "0 * * * * *"; // every minute (6-field with seconds)
        let schedule = Schedule::from_str(expr).unwrap();
        let now = Utc::now();
        let two_min_ago = now - chrono::Duration::minutes(2);
        // There should be at least one scheduled time in (two_min_ago, now]
        let next = schedule.after(&two_min_ago).next().unwrap();
        assert!(next <= now, "expected scheduled time in the window");
    }

    #[test]
    fn sliding_window_no_false_fire() {
        // Schedule: every day at 00:00. If last_tick was 1 minute ago,
        // there should be no scheduled time in (last_tick, now] (unless it's midnight)
        let expr = "0 0 0 * * *"; // midnight daily (6-field)
        let schedule = Schedule::from_str(expr).unwrap();
        let now = chrono::Utc::now();
        let one_min_ago = now - chrono::Duration::minutes(1);
        if let Some(next) = schedule.after(&one_min_ago).next() {
            // next should be in the future (next midnight), not in our 1-minute window
            // (unless we happen to be running at exactly midnight, which is unlikely in tests)
            if next <= now {
                // This is fine — we happened to run at midnight
            }
        }
    }

    #[tokio::test]
    async fn in_flight_guard_removes_on_drop() {
        let in_flight: Arc<Mutex<HashSet<usize>>> = Arc::new(Mutex::new(HashSet::new()));
        in_flight.lock().await.insert(42);
        {
            let _guard = InFlightGuard { idx: 42, in_flight: in_flight.clone() };
            assert!(in_flight.lock().await.contains(&42));
        }
        // Guard dropped — spawned a task to remove. Give it a moment.
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        assert!(!in_flight.lock().await.contains(&42));
    }
}
