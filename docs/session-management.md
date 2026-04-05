# Session Management

agent-broker maintains a pool of persistent agent sessions — one per Telegram thread/topic. This doc covers how sessions are created, kept alive, evicted, and resumed.

## Session Lifecycle

```
User message → get_or_create session → stream prompt → update last_active
                                                              ↓
                                              cleanup_idle checks every 15 min
                                                              ↓
                                         idle > 2hr → evict → notify user
                                                              ↓
                                         next message → resume session
```

## TTL & Cleanup Settings

Configured in `src/telegram.rs`:

| Constant | Value | Description |
|---|---|---|
| `CLEANUP_INTERVAL_SECS` | 900 (15 min) | How often the idle cleanup loop runs |
| `SESSION_TTL_SECS` | 7200 (2 hr) | Inactivity time before a session is evicted |

A session is only eligible for eviction if it is **not actively streaming** a response.

## Session Keys

Each session is keyed by context:

| Chat type | Session key |
|---|---|
| Private DM | `<chat_id>` |
| Group chat | `<chat_id>:<user_id>` |
| Forum topic | `<chat_id>:<thread_id>` |

## Resume Mechanism

When a session is evicted, its ACP session ID is saved in `prev_session_ids`. On the next message:

1. kiro is spawned with `--resume` flag
2. `session/load` is attempted with the saved session ID (if kiro reports `loadSession` capability)
3. If `session/load` succeeds → full memory restored ✅
4. If `session/load` fails → falls back to `--resume` only (partial, kiro-dependent)
5. If no prior session ID exists → cold `session/new`

The user is notified when their session is evicted:
> ⏱ Your session was closed due to inactivity. Send any message to resume.

## Pool Limits

`max_sessions` in `config.toml` caps the number of concurrent live sessions. If the pool is full, new sessions are rejected with a warning. Tune this based on available RAM — each live kiro process consumes memory.

```toml
[pool]
max_sessions = 10
```

## Tuning for Different Environments

| Use case | TTL | Cleanup interval |
|---|---|---|
| Testing | 120s (2 min) | 60s (1 min) |
| Production (default) | 7200s (2 hr) | 900s (15 min) |
| High-traffic / low RAM | 1800s (30 min) | 300s (5 min) |

For testing, temporarily change the constants in `src/telegram.rs` and rebuild.
