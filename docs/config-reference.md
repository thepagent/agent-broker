# Configuration Reference

OpenAB is configured via a TOML file (default: `config.toml`). Before parsing, the loader performs `${VAR}` expansion — any string value can reference environment variables using `${ENV_VAR_NAME}` syntax.

## Table of Contents

- [`[discord]`](#discord)
- [`[slack]`](#slack)
- [`[agent]`](#agent)
- [`[pool]`](#pool)
- [`[reactions]`](#reactions)
- [`[stt]`](#stt-speech-to-text)
- [Environment Variables](#environment-variables)
- [Helm Values](#helm-values)

---

## `[discord]`

Required if `[slack]` is not configured.

| Key | Type | Default | Description |
|---|---|---|---|
| `bot_token` | string | **required** | Discord bot token. Use `${DISCORD_BOT_TOKEN}` to inject from env. |
| `allow_all_channels` | bool | auto | `true` = allow all channels. Auto-detected: `true` when `allowed_channels` is empty, `false` otherwise. |
| `allowed_channels` | string[] | `[]` | Channel IDs to allow. Setting this implies `allow_all_channels = false`. |
| `allow_all_users` | bool | auto | `true` = allow all users. Auto-detected same as channels. |
| `allowed_users` | string[] | `[]` | User IDs to allow. Setting this implies `allow_all_users = false`. |
| `allow_bot_messages` | string | `"off"` | How to handle messages from other bots. See [Bot Message Modes](#bot-message-modes). |
| `trusted_bot_ids` | string[] | `[]` | When non-empty, only bot messages from these snowflake IDs pass the bot gate. Ignored when `allow_bot_messages = "off"`. |
| `allow_user_messages` | string | `"involved"` | Controls @mention requirement in threads. See [User Message Modes](#user-message-modes). |
| `max_bot_turns` | integer | `20` | Max consecutive bot turns per thread before throttling. Resets on a human message. |

---

## `[slack]`

Required if `[discord]` is not configured.

| Key | Type | Default | Description |
|---|---|---|---|
| `bot_token` | string | **required** | Slack Bot User OAuth Token (`xoxb-…`). Use `${SLACK_BOT_TOKEN}`. |
| `app_token` | string | **required** | Slack App-Level Token (`xapp-…`) for Socket Mode. Use `${SLACK_APP_TOKEN}`. |
| `allow_all_channels` | bool | auto | Same auto-detection logic as Discord. |
| `allowed_channels` | string[] | `[]` | Slack channel IDs (e.g. `C0123456789`). |
| `allow_all_users` | bool | auto | Same auto-detection logic as Discord. |
| `allowed_users` | string[] | `[]` | Slack user IDs (e.g. `U0123456789`). |
| `allow_bot_messages` | string | `"off"` | See [Bot Message Modes](#bot-message-modes). |
| `trusted_bot_ids` | string[] | `[]` | Slack Bot User IDs (`U…`). Find via Slack UI: click bot profile → Copy member ID. |
| `allow_user_messages` | string | `"involved"` | See [User Message Modes](#user-message-modes). |
| `max_bot_turns` | integer | `20` | Max consecutive bot turns per thread. |

---

## `[agent]`

| Key | Type | Default | Description |
|---|---|---|---|
| `command` | string | **required** | CLI command to spawn the agent (e.g. `"claude-agent-acp"`, `"kiro-cli"`, `"codex-acp"`, `"gemini"`, `"opencode"`, `"cursor-agent"`, `"copilot"`). |
| `args` | string[] | `[]` | Arguments passed to `command`. |
| `working_dir` | string | `"/tmp"` | Working directory for the agent process. |
| `env` | map | `{}` | Extra environment variables injected into the agent process. |

**Examples by agent type:**

```toml
# Claude Code
[agent]
command = "claude-agent-acp"
args = ["--acp"]
working_dir = "/home/user"
env = { ANTHROPIC_API_KEY = "${ANTHROPIC_API_KEY}" }

# Kiro
[agent]
command = "kiro-cli"
args = ["acp", "--trust-all-tools"]
working_dir = "/home/user"

# Codex
[agent]
command = "codex-acp"
args = ["--acp"]
working_dir = "/home/user"
env = { OPENAI_API_KEY = "${OPENAI_API_KEY}" }

# Gemini
[agent]
command = "gemini"
args = ["--acp"]
working_dir = "/home/user"
env = { GEMINI_API_KEY = "${GEMINI_API_KEY}" }

# OpenCode
[agent]
command = "opencode"
args = ["acp"]
working_dir = "/home/user"

# Copilot
[agent]
command = "copilot"
args = ["acp", "--stdio"]
working_dir = "/home/user"
```

---

## `[pool]`

| Key | Type | Default | Description |
|---|---|---|---|
| `max_sessions` | integer | `10` | Maximum number of concurrent agent sessions (one per thread). |
| `session_ttl_hours` | integer | `24` | Idle session TTL in hours. Sessions inactive longer than this are evicted. |

---

## `[reactions]`

Controls emoji status reactions on messages.

| Key | Type | Default | Description |
|---|---|---|---|
| `enabled` | bool | `true` | Enable emoji status reactions. |
| `remove_after_reply` | bool | `false` | Remove all status reactions after the bot replies. |

### `[reactions.emojis]`

| Key | Type | Default | Description |
|---|---|---|---|
| `queued` | string | `"👀"` | Reaction added when the message is queued. |
| `thinking` | string | `"🤔"` | Reaction while the agent is thinking (no tool calls yet). |
| `tool` | string | `"🔥"` | Reaction during generic tool execution. |
| `coding` | string | `"👨‍💻"` | Reaction during coding/file-edit tool calls. |
| `web` | string | `"⚡"` | Reaction during web/search tool calls. |
| `done` | string | `"🆗"` | Reaction on successful completion. |
| `error` | string | `"😱"` | Reaction on error. |

### `[reactions.timing]`

| Key | Type | Default | Description |
|---|---|---|---|
| `debounce_ms` | integer | `700` | Milliseconds to debounce reaction updates (prevents flicker). |
| `stall_soft_ms` | integer | `10000` | After this many ms without output, transition to the stall reaction. |
| `stall_hard_ms` | integer | `30000` | After this many ms, switch to hard-stall reaction. |
| `done_hold_ms` | integer | `1500` | How long (ms) to hold the "done" reaction before removing it (when `remove_after_reply = true`). |
| `error_hold_ms` | integer | `2500` | How long (ms) to hold the "error" reaction before removing it. |

---

## `[stt]` (Speech-to-Text)

Enables transcription of voice message attachments (Discord only). See also: [stt.md](stt.md).

| Key | Type | Default | Description |
|---|---|---|---|
| `enabled` | bool | `false` | Enable STT transcription of voice messages. |
| `api_key` | string | `""` | API key for the STT provider. Auto-detected from `GROQ_API_KEY` when using the default Groq endpoint. For local servers, set to any non-empty string (e.g. `"not-needed"`). |
| `model` | string | `"whisper-large-v3-turbo"` | Whisper model name. Use `"whisper-1"` for OpenAI, `"large-v3-turbo"` for a local Whisper server. |
| `base_url` | string | `"https://api.groq.com/openai/v1"` | OpenAI-compatible `/audio/transcriptions` endpoint base URL. Change to point to OpenAI, a local server, or a LAN endpoint. `GROQ_API_KEY` auto-detection is disabled when this is overridden. |

---

## Bot Message Modes

Applies to both `discord.allow_bot_messages` and `slack.allow_bot_messages`.

| Value | Aliases | Description |
|---|---|---|
| `"off"` | `"none"`, `"false"` | Ignore all messages from other bots. |
| `"mentions"` | | Only process bot messages that @mention this bot. |
| `"all"` | `"true"` | Process all bot messages (up to `max_bot_turns` per thread). |

When `trusted_bot_ids` is non-empty, only listed bot IDs pass the bot gate (any mode other than `"off"`).

---

## User Message Modes

Applies to both `discord.allow_user_messages` and `slack.allow_user_messages`.

| Value | Description |
|---|---|
| `"involved"` | Respond in threads the bot already participates in without requiring @mention. In new threads, @mention is required to start a session. |
| `"mentions"` | Always require @mention. |
| `"multibot-mentions"` | Like `"involved"`, but once another bot has posted in the thread, @mention is required again. Useful in multi-agent channels to prevent cross-bot loops. |

---

## Environment Variables

These are read directly by the process (not via `config.toml`):

| Variable | Description |
|---|---|
| `RUST_LOG` | Log verbosity filter (e.g. `openab=debug`). Falls back to `openab=info`. Uses `tracing_subscriber::EnvFilter` format. |
| `GROQ_API_KEY` | Auto-detected as `stt.api_key` when `stt.base_url` is the default Groq endpoint. |
| `DISCORD_BOT_TOKEN` | Typically injected into config via `bot_token = "${DISCORD_BOT_TOKEN}"`. |
| `SLACK_BOT_TOKEN` | Typically injected into config via `bot_token = "${SLACK_BOT_TOKEN}"`. |
| `SLACK_APP_TOKEN` | Typically injected into config via `app_token = "${SLACK_APP_TOKEN}"`. |
| `ANTHROPIC_API_KEY` | Passed to Claude Code agent via `agent.env`. |
| `OPENAI_API_KEY` | Passed to Codex agent via `agent.env`. |
| `GEMINI_API_KEY` | Passed to Gemini agent via `agent.env`. |

---

## Helm Values

When deploying with the Helm chart, each agent is defined under `agents.<name>`. The chart generates `config.toml` from these values and injects secrets as environment variables.

| Helm Path | config.toml equivalent | Notes |
|---|---|---|
| `agents.<name>.enabled` | — | Set `false` to skip resource creation for this agent. |
| `agents.<name>.command` | `agent.command` | |
| `agents.<name>.args` | `agent.args` | |
| `agents.<name>.workingDir` | `agent.working_dir` | Default: `/home/agent` |
| `agents.<name>.env` | `agent.env` | Key-value map of env vars. |
| `agents.<name>.envFrom` | Kubernetes `envFrom` | Array of `configMapRef`/`secretRef` for bulk env injection. |
| **Discord** | | |
| `agents.<name>.discord.enabled` | — | Toggle Discord adapter for this agent. |
| `agents.<name>.discord.botToken` | Secret → `DISCORD_BOT_TOKEN` | |
| `agents.<name>.discord.allowedChannels` | `discord.allowed_channels` | Use `--set-string` to avoid float64 precision loss on large snowflake IDs. |
| `agents.<name>.discord.allowedUsers` | `discord.allowed_users` | Use `--set-string`. |
| `agents.<name>.discord.allowAllChannels` | `discord.allow_all_channels` | |
| `agents.<name>.discord.allowAllUsers` | `discord.allow_all_users` | |
| `agents.<name>.discord.allowBotMessages` | `discord.allow_bot_messages` | Validated: must be `"off"`, `"mentions"`, or `"all"`. |
| `agents.<name>.discord.trustedBotIds` | `discord.trusted_bot_ids` | Use `--set-string`. Must be 17–20 digit snowflake IDs. |
| `agents.<name>.discord.allowUserMessages` | `discord.allow_user_messages` | Validated: `"involved"`, `"mentions"`, or `"multibot-mentions"`. |
| **Slack** | | |
| `agents.<name>.slack.enabled` | — | Toggle Slack adapter for this agent. |
| `agents.<name>.slack.botToken` | Secret → `SLACK_BOT_TOKEN` | |
| `agents.<name>.slack.appToken` | Secret → `SLACK_APP_TOKEN` | |
| `agents.<name>.slack.allowedChannels` | `slack.allowed_channels` | |
| `agents.<name>.slack.allowedUsers` | `slack.allowed_users` | |
| `agents.<name>.slack.allowBotMessages` | `slack.allow_bot_messages` | |
| `agents.<name>.slack.trustedBotIds` | `slack.trusted_bot_ids` | |
| `agents.<name>.slack.allowUserMessages` | `slack.allow_user_messages` | |
| **Pool** | | |
| `agents.<name>.pool.maxSessions` | `pool.max_sessions` | Default: `10` |
| `agents.<name>.pool.sessionTtlHours` | `pool.session_ttl_hours` | Default: `24` |
| **Reactions** | | |
| `agents.<name>.reactions.enabled` | `reactions.enabled` | Default: `true` |
| `agents.<name>.reactions.removeAfterReply` | `reactions.remove_after_reply` | Default: `false` |
| **STT** | | |
| `agents.<name>.stt.enabled` | `stt.enabled` | Default: `false` |
| `agents.<name>.stt.apiKey` | Secret → `STT_API_KEY` / `stt.api_key` | Required if `stt.enabled = true`. |
| `agents.<name>.stt.model` | `stt.model` | Default: `"whisper-large-v3-turbo"` |
| `agents.<name>.stt.baseUrl` | `stt.base_url` | Default: `"https://api.groq.com/openai/v1"` |
| **Persistence** | | |
| `agents.<name>.persistence.enabled` | — | Enable PVC for agent home directory. Default: `true` |
| `agents.<name>.persistence.storageClass` | — | Kubernetes StorageClass. Default: `""` (cluster default). |
| `agents.<name>.persistence.size` | — | PVC size. Default: `1Gi` |
| **Misc** | | |
| `agents.<name>.agentsMd` | Mounts as `AGENTS.md` | When set, shadows any `AGENTS.md` on the PVC with this string value. Remove from Helm values to restore PVC version. |
| `agents.<name>.image` | — | Override container image for this agent. |
| `agents.<name>.resources` | — | Kubernetes resource requests/limits. |
| `agents.<name>.nodeSelector` | — | Kubernetes nodeSelector. |
| `agents.<name>.tolerations` | — | Kubernetes tolerations. |
| `agents.<name>.affinity` | — | Kubernetes affinity. |
| `agents.<name>.extraInitContainers` | — | Additional init containers. |
| `agents.<name>.extraContainers` | — | Additional sidecar containers. |
| `agents.<name>.extraVolumeMounts` | — | Additional volume mounts. |
| `agents.<name>.extraVolumes` | — | Additional volumes. |
| **Global image** | | |
| `image.repository` | — | Default: `ghcr.io/openabdev/openab` |
| `image.tag` | — | Default: chart `AppVersion` |
| `image.pullPolicy` | — | Default: `IfNotPresent` |
