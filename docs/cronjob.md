# Scheduled Messages (Config-Driven Cron)

Send recurring prompts to your agent on a schedule — daily summaries, weekly reports, periodic scans — without external infrastructure.

## How It Works

1. Define `[[cronjobs]]` entries in `config.toml`
2. OpenAB's internal scheduler evaluates cron expressions once per minute
3. When a schedule matches, the message is sent to the agent as if a user typed it
4. The agent processes the message and replies to the target channel

No external scheduler (K8s CronJob, GitHub Actions) is needed for simple use cases.

## Quick Start

Add to your `config.toml`:

```toml
[[cronjobs]]
schedule = "0 9 * * 1-5"
channel = "123456789012345678"
message = "summarize yesterday's merged PRs"
```

This sends `summarize yesterday's merged PRs` to the agent every weekday at 09:00 UTC in the specified Discord channel.

## Configuration

Each `[[cronjobs]]` entry supports these fields:

```toml
[[cronjobs]]
schedule = "0 9 * * 1-5"                    # required: cron expression
channel = "123456789012345678"               # required: target channel ID
message = "summarize yesterday's merged PRs" # required: prompt for the agent
platform = "discord"                         # optional, default: "discord"
sender_name = "DailyOps"                     # optional, default: "openab-cron"
timezone = "Asia/Taipei"                     # optional, default: "UTC"
thread_id = ""                               # optional: post to existing thread
```

| Field | Required | Default | Description |
|-------|----------|---------|-------------|
| `schedule` | ✅ | — | 5-field POSIX cron expression |
| `channel` | ✅ | — | Discord channel/thread ID or Slack channel ID |
| `message` | ✅ | — | Message sent to the agent as a prompt |
| `platform` | | `"discord"` | `"discord"` or `"slack"` |
| `sender_name` | | `"openab-cron"` | Attribution shown in prompt context |
| `timezone` | | `"UTC"` | IANA timezone (e.g. `"Asia/Taipei"`, `"Europe/Berlin"`) |
| `thread_id` | | — | Post into an existing thread instead of the channel |

## Cron Expression Format

Standard 5-field POSIX cron, same as Linux crontab, K8s CronJob, and GitHub Actions:

```
┌───────────── minute (0-59)
│ ┌───────────── hour (0-23)
│ │ ┌───────────── day of month (1-31)
│ │ │ ┌───────────── month (1-12)
│ │ │ │ ┌───────────── day of week (0-7, 0 and 7 = Sunday)
│ │ │ │ │
* * * * *
```

### Examples

| Expression | Meaning |
|---|---|
| `0 9 * * 1-5` | Weekdays at 09:00 |
| `0 0 * * 0` | Sundays at midnight |
| `*/30 * * * *` | Every 30 minutes |
| `0 18 * * 1-5` | Weekdays at 18:00 |
| `0 9 1 * *` | First day of every month at 09:00 |

## Timezone Support

By default, schedules are evaluated in UTC. Set `timezone` to any IANA timezone:

```toml
[[cronjobs]]
schedule = "0 9 * * 1-5"
channel = "123456789012345678"
message = "good morning team, here's today's agenda"
timezone = "Asia/Taipei"
```

This fires at 09:00 Taipei time (01:00 UTC).

## Multiple Jobs

Define as many `[[cronjobs]]` entries as you need:

```toml
[[cronjobs]]
schedule = "0 9 * * 1-5"
channel = "123456789012345678"
message = "summarize yesterday's merged PRs"
sender_name = "DailyOps"
timezone = "Asia/Taipei"

[[cronjobs]]
schedule = "0 0 * * 0"
channel = "123456789012345678"
message = "generate weekly status report"
sender_name = "WeeklyReport"

[[cronjobs]]
schedule = "0 18 * * 1-5"
channel = "C0123456789"
message = "check for any critical alerts in the last 8 hours"
platform = "slack"
sender_name = "OpsBot"
```

## Helm Deployment

When using the Helm chart, define cronjobs under each agent in `values.yaml`:

```yaml
agents:
  kiro:
    cronjobs:
      - schedule: "0 9 * * 1-5"
        channel: "123456789012345678"
        message: "summarize yesterday's merged PRs"
        platform: "discord"
        senderName: "DailyOps"
        timezone: "Asia/Taipei"
      - schedule: "0 0 * * 0"
        channel: "123456789012345678"
        message: "generate weekly status report"
```

> ⚠️ Use `--set-string` for channel IDs to avoid float64 precision loss:
> ```bash
> helm upgrade mybot charts/openab \
>   --set-string agents.kiro.cronjobs[0].channel="123456789012345678"
> ```

## Behaviors

- **Minute-aligned**: The scheduler aligns to minute boundaries (`:00`), so `0 9 * * *` fires at exactly 09:00:00, not at whatever second the process started.
- **Overlap protection**: If a previous execution of the same job is still running, the next tick is skipped.
- **Isolation**: Cron failures are logged but never block interactive chat traffic.
- **Stateless**: No persistence needed. Schedules are re-evaluated from config on restart.
- **Graceful shutdown**: In-flight cron tasks are waited on (up to 30 seconds) during shutdown.

## Sender Identity

When a cron job fires, the agent sees a sender context like:

```
🕐 [DailyOps]: summarize yesterday's merged PRs
```

Use `sender_name` to distinguish different scheduled tasks in logs and thread titles. The agent can use this to tailor its response (e.g. "DailyOps asked for a summary" vs "WeeklyReport asked for a report").

## When to Use External Schedulers Instead

Config-driven cron covers the 80% use case: "send this message at this time." For advanced needs, use external schedulers:

| Need | Recommendation |
|---|---|
| Simple recurring prompts | ✅ Config-driven cron (this feature) |
| Long-running jobs (>5 min) | K8s CronJob |
| Conditional logic / retries | GitHub Actions or Step Functions |
| Multi-step workflows / DAGs | GitHub Actions or Step Functions |
| Per-execution isolation | K8s CronJob (separate Pod per run) |

See [Kubernetes CronJob Reference Architecture](cronjob_k8s_refarch.md) for the external scheduler approach.

## Troubleshooting

| Symptom | Cause | Fix |
|---|---|---|
| Job never fires | Invalid cron expression | Check logs for `invalid cron expression, skipping` |
| Job fires but no reply | Agent error | Check logs for `cron handle_message error` |
| Wrong time | Timezone mismatch | Set `timezone` explicitly (default is UTC) |
| Job skipped | Previous execution still running | Check logs for `skipping cronjob, previous execution still running` |
| Channel not found | Bot not in channel | Invite the bot to the target channel |
