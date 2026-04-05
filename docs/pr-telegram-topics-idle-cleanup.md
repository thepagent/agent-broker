# PR: Telegram adapter + session lifecycle with memory compaction

## Summary

Ports agent-broker from Discord to Telegram and adds full session lifecycle management: idle eviction, resume, and memory compaction so the agent remembers context across session boundaries.

## Changes

### 1. Telegram Adapter (`src/telegram.rs`) — replaces `discord.rs`

- Uses `teloxide` instead of `serenity`
- Telegram forum supergroup topics map 1:1 to sessions (one topic = one conversation)
- General topic always spawns a new topic to keep the main channel clean
- Auto-renames new topics with a Kiro-generated title after first response
- Emoji reactions as live status on the user's message: `👀 → 🤔 → 🔥/👨‍💻/⚡ → 👍 / 😱`
- Auth via `allowed_users` (Telegram user IDs) in `config.toml`
- Bot commands: `!stop`, `!restart`, `!status`

Session key strategy:
```
DM chat      → <chat_id>
Group chat   → <chat_id>:<user_id>
Forum topic  → <chat_id>:<thread_id>
```

### 2. Session Lifecycle (`src/acp/pool.rs`)

```
User message arrives
        │
        ▼
get_or_create(thread_id)
        │
        ├─ alive session? ──────────────────────────────► use it
        │
        └─ no session / dead
                │
                ▼
           spawn kiro-cli acp
                │
                ├─ prev session_id exists?
                │       │
                │       ├─ YES → session/load ──► ✅ full resume
                │       │
                │       └─ NO  → session/new  ──► fresh start
                │
                └─ pending_context? ──► prepend to first prompt
                                              (memory compaction injection)
        │
        ▼
stream_prompt ──► is_streaming = true, update last_active
        │
        ▼
prompt_done ──► is_streaming = false, update last_active
        │
        ▼
cleanup_idle (every 15 min)
        │
        ├─ is_streaming? ──► skip (never evict mid-response)
        │
        └─ idle > 2hr? ──► compact → evict → notify user
```

### 3. Memory Compaction (`src/acp/pool.rs` + `src/acp/connection.rs`)

Kiro ACP does not replay history on `session/load` — every resume is a cold start. Compaction bridges this gap:

```
Session idle > TTL
        │
        ▼
cleanup_idle sends compaction prompt (lock released — no deadlock):
  "Summarize this conversation in 3rd person, capturing all key
   facts about the user and topics discussed. Be concise."
        │
        ▼
Summary stored in SessionPool.summaries[thread_id]
        │
        ▼
Session evicted, acp_session_id saved in prev_session_ids
        │
Next message to same thread
        │
        ▼
get_or_create → session/new → conn.pending_context = summary
        │
        ▼
session_prompt prepends context to first real user prompt:
  "[Context from previous session]: <summary>\n\n<user message>"
        │
        ▼
Agent answers with full context ✅
```

No extra round-trip — context is injected inline with the first prompt.

### 4. Config

`[discord]` → `[telegram]`:

```toml
[telegram]
bot_token = "${TELEGRAM_BOT_TOKEN}"
allowed_users = [123456789]          # Telegram user ID allowlist

[pool]
max_sessions = 10
```

TTL constants in `src/telegram.rs`:
```rust
const CLEANUP_INTERVAL_SECS: u64 = 900;   // run cleanup every 15 min
const SESSION_TTL_SECS: u64 = 7200;       // evict after 2hr idle
```

## Testing

**Alice test** (memory compaction verification):
1. Send: *"Hi I'm Alice. I love blue flowers."*
2. Wait for eviction notification (⏱ ~2hr, or lower TTL for testing)
3. Send: *"Who am I?"* and *"What color flower do I love?"*
4. Expected: agent answers correctly from compacted summary ✅

**Tested with:** kiro-cli 1.29.3, `session/load` always fails with `-32603` (kiro does not persist history) → compaction path is the active resume mechanism.

## What's Not In This PR

- Discord support is fully removed (replacement, not addition)
- `docs/telegram-bot-howto.md` not yet written (see setup guide in README)
