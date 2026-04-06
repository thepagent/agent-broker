# agent-broker

A Rust bridge service between Discord and any ACP-compatible coding CLI (Kiro CLI, Claude Code, Codex, Gemini, etc.) using the [Agent Client Protocol](https://github.com/anthropics/agent-protocol) over stdio JSON-RPC.

```
┌──────────────┐  Gateway WS   ┌──────────────┐  ACP stdio    ┌──────────────┐
│   Discord    │◄─────────────►│ agent-broker │──────────────►│  coding CLI  │
│   User       │               │   (Rust)     │◄── JSON-RPC ──│  (acp mode)  │
└──────────────┘               └──────────────┘               └──────────────┘
```

## Demo

![agent-broker demo](images/demo.png)

## Features

- **Pluggable agent backend** — swap between Kiro CLI, Claude Code, Codex, Gemini via config
- **@mention trigger** — mention the bot in an allowed channel to start a conversation
- **Thread-based multi-turn** — auto-creates threads; no @mention needed for follow-ups
- **Edit-streaming** — live-updates the Discord message every 1.5s as tokens arrive
- **Emoji status reactions** — 👀→🤔→🔥/👨‍💻/⚡→👍+random mood face
- **Session pool** — one CLI process per thread, auto-managed lifecycle
- **ACP protocol** — JSON-RPC over stdio with tool call, thinking, and permission auto-reply support
- **Kubernetes-ready** — Dockerfile + k8s manifests with PVC for auth persistence

## Quick Start

### 1. Create a Discord Bot

See [docs/discord-bot-howto.md](docs/discord-bot-howto.md) for a detailed step-by-step guide.

In short:

1. Go to https://discord.com/developers/applications and create an application
2. Bot tab → enable **Message Content Intent**
3. OAuth2 → URL Generator → scope: `bot` → permissions: Send Messages, Send Messages in Threads, Create Public Threads, Read Message History, Add Reactions, Manage Messages
4. Invite the bot to your server using the generated URL

### 2. Configure

```bash
cp config.toml.example config.toml
```

Edit `config.toml`:
```toml
[discord]
bot_token = "${DISCORD_BOT_TOKEN}"
allowed_channels = ["YOUR_CHANNEL_ID"]

[agent]
command = "kiro-cli"
args = ["acp", "--trust-all-tools"]
working_dir = "/tmp"
```

### 3. Build & Run

```bash
export DISCORD_BOT_TOKEN="your-token"

# Development
cargo run

# Production
cargo build --release
./target/release/agent-broker config.toml
```

If no config path is given, it defaults to `config.toml` in the current directory.

### 4. Use

In your Discord channel:
```
@AgentBroker explain this code
```

The bot creates a thread. After that, just type in the thread — no @mention needed.

## Pluggable Agent Backends

Swap backends using the `agent.preset` Helm value or manual config. Tested backends:

| Preset | CLI | ACP Adapter | Auth |
|--------|-----|-------------|------|
| (default) | Kiro CLI | Native `kiro-cli acp` | `kiro-cli login --use-device-flow` |
| `codex` | Codex | [@zed-industries/codex-acp](https://github.com/zed-industries/codex-acp) | `codex login --device-auth` |
| `claude` | Claude Code | [@agentclientprotocol/claude-agent-acp](https://github.com/agentclientprotocol/claude-agent-acp) | `claude setup-token` |
| `gemini` | Gemini CLI | Native `gemini --acp` | Google OAuth or `GEMINI_API_KEY` |

### Helm Install (recommended)

```bash
helm repo add agent-broker https://thepagent.github.io/agent-broker
helm repo update

# Kiro CLI (default)
helm install agent-broker agent-broker/agent-broker \
  --set discord.botToken="$DISCORD_BOT_TOKEN" \
  --set-string discord.allowedChannels[0]="YOUR_CHANNEL_ID"

# Codex
helm install agent-broker agent-broker/agent-broker \
  --set discord.botToken="$DISCORD_BOT_TOKEN" \
  --set-string discord.allowedChannels[0]="YOUR_CHANNEL_ID" \
  --set agent.preset=codex

# Claude Code
helm install agent-broker agent-broker/agent-broker \
  --set discord.botToken="$DISCORD_BOT_TOKEN" \
  --set-string discord.allowedChannels[0]="YOUR_CHANNEL_ID" \
  --set agent.preset=claude

# Gemini
helm install agent-broker agent-broker/agent-broker \
  --set discord.botToken="$DISCORD_BOT_TOKEN" \
  --set-string discord.allowedChannels[0]="YOUR_CHANNEL_ID" \
  --set agent.preset=gemini
```

Then authenticate inside the pod (first time only):

```bash
# Kiro CLI
kubectl exec -it deployment/agent-broker -- kiro-cli login --use-device-flow

# Codex
kubectl exec -it deployment/agent-broker -- codex login --device-auth

# Claude Code
kubectl exec -it deployment/agent-broker -- claude setup-token
# Then: helm upgrade agent-broker agent-broker/agent-broker --set env.CLAUDE_CODE_OAUTH_TOKEN="<token>"

# Gemini (Google OAuth — open URL in browser, curl callback from pod)
kubectl exec -it deployment/agent-broker -- gemini
# Or use API key: helm upgrade agent-broker agent-broker/agent-broker --set env.GEMINI_API_KEY="<key>"
```

Restart after auth: `kubectl rollout restart deployment agent-broker`

### Manual config.toml

For non-Helm deployments, swap the `[agent]` block:

```toml
# Kiro CLI (default)
[agent]
command = "kiro-cli"
args = ["acp", "--trust-all-tools"]
working_dir = "/tmp"

# Codex (requires codex-acp in PATH)
[agent]
command = "codex-acp"
args = []
working_dir = "/tmp"

# Claude Code (requires claude-agent-acp in PATH)
[agent]
command = "claude-agent-acp"
args = []
working_dir = "/tmp"

# Gemini
[agent]
command = "gemini"
args = ["--acp"]
working_dir = "/tmp"
env = { GEMINI_API_KEY = "${GEMINI_API_KEY}" }
```

## Configuration Reference

```toml
[discord]
bot_token = "${DISCORD_BOT_TOKEN}"   # supports env var expansion
allowed_channels = ["123456789"]      # channel ID allowlist

[agent]
command = "kiro-cli"                  # CLI command
args = ["acp", "--trust-all-tools"]   # ACP mode args
working_dir = "/tmp"                  # agent working directory
env = {}                              # extra env vars passed to the agent

[pool]
max_sessions = 10                     # max concurrent sessions
session_ttl_hours = 24                # idle session TTL

[reactions]
enabled = true                        # enable emoji status reactions
remove_after_reply = false            # remove reactions after reply

[reactions.emojis]
queued = "👀"
thinking = "🤔"
tool = "🔥"
coding = "👨‍💻"
web = "⚡"
done = "🆗"
error = "😱"

[reactions.timing]
debounce_ms = 700                     # intermediate state debounce
stall_soft_ms = 10000                 # 10s idle → 🥱
stall_hard_ms = 30000                 # 30s idle → 😨
done_hold_ms = 1500                   # keep done emoji for 1.5s
error_hold_ms = 2500                  # keep error emoji for 2.5s
```

## Kubernetes Deployment

The Docker image bundles both `agent-broker` and `kiro-cli` in a single container (agent-broker spawns kiro-cli as a child process).

### Pod Architecture

```
┌─ Kubernetes Pod ─────────────────────────────────────────────────┐
│                                                                  │
│  ┌─────────────────────────────────────────────────────────┐     │
│  │  agent-broker (main process, PID 1)                     │     │
│  │                                                         │     │
│  │  ┌──────────────┐   ┌──────────────┐   ┌───────────┐    │     │
│  │  │ Discord      │   │ Session Pool │   │ Reaction  │    │     │
│  │  │ Gateway WS   │   │ (per thread) │   │ Controller│    │     │
│  │  └──────┬───────┘   └──────┬───────┘   └───────────┘    │     │
│  │         │                  │                            │     │
│  └─────────┼──────────────────┼────────────────────────────┘     │
│            │                  │                                  │
│            │ @mention /       │ spawn + stdio                    │
│            │ thread msg       │ JSON-RPC (ACP)                   │
│            │                  │                                  │
│            ▼                  ▼                                  │
│  ┌──────────────────────────────────────────────────────────┐    │
│  │  kiro-cli acp --trust-all-tools  (child process)         │    │
│  │                                                          │    │
│  │  stdin  ◄── JSON-RPC requests  (session/new, prompt)     │    │
│  │  stdout ──► JSON-RPC responses (text, tool_call, done)   │    │
│  │  stderr ──► (ignored)                                    │    │
│  └──────────────────────────────────────────────────────────┘    │
│                                                                  │
│  ┌─ PVC Mount (/data) ──────────────────────────────────────┐    │
│  │  ~/.kiro/              ← settings, skills, sessions      │    │
│  │  ~/.local/share/kiro-cli/ ← OAuth tokens (data.sqlite3)  │    │
│  └──────────────────────────────────────────────────────────┘    │
│                                                                  │
└──────────────────────────────────────────────────────────────────┘
         │
         │ WebSocket (wss://gateway.discord.gg)
         ▼
┌──────────────────┐         ┌──────────────┐
│  Discord API     │ ◄─────► │  Discord     │
│  Gateway         │         │  Users       │
└──────────────────┘         └──────────────┘
```

- **Single container** — agent-broker is PID 1, spawns kiro-cli as a child process
- **stdio JSON-RPC** — ACP communication over stdin/stdout, no network ports needed
- **Session pool** — one kiro-cli process per Discord thread, up to `max_sessions`
- **PVC** — persists OAuth tokens and settings across pod restarts

### Install with Your Coding CLI

Use one of these prompts with any coding CLI (Kiro CLI, Claude Code, Codex, Gemini, etc.) on the host that has `helm` and `kubectl` access to your cluster:

**Kiro CLI (default):**
> Install agent-broker on my local k8s cluster using the Helm chart from https://thepagent.github.io/agent-broker. My Discord bot token is in the environment variable DISCORD_BOT_TOKEN and my channel ID is <REPLACE_WITH_YOUR_CHANNEL_ID>. After install, follow the NOTES output to authenticate, then restart the deployment.

**Codex:**
> Install agent-broker on my local k8s cluster using the Helm chart from https://thepagent.github.io/agent-broker with `--set agent.preset=codex`. My Discord bot token is in the environment variable DISCORD_BOT_TOKEN and my channel ID is <REPLACE_WITH_YOUR_CHANNEL_ID>. After install, follow the NOTES output to authenticate, then restart the deployment.

**Claude Code:**
> Install agent-broker on my local k8s cluster using the Helm chart from https://thepagent.github.io/agent-broker with `--set agent.preset=claude`. My Discord bot token is in the environment variable DISCORD_BOT_TOKEN and my channel ID is <REPLACE_WITH_YOUR_CHANNEL_ID>. After install, follow the NOTES output to authenticate, then restart the deployment.

**Gemini:**
> Install agent-broker on my local k8s cluster using the Helm chart from https://thepagent.github.io/agent-broker with `--set agent.preset=gemini`. My Discord bot token is in the environment variable DISCORD_BOT_TOKEN and my channel ID is <REPLACE_WITH_YOUR_CHANNEL_ID>. After install, follow the NOTES output to authenticate, then restart the deployment.

### Build & Push

```bash
docker build -t agent-broker:latest .
docker tag agent-broker:latest <your-registry>/agent-broker:latest
docker push <your-registry>/agent-broker:latest
```

### Deploy

```bash
# Create the secret with your bot token
kubectl create secret generic agent-broker-secret \
  --from-literal=discord-bot-token="your-token"

# Edit k8s/configmap.yaml with your channel IDs
kubectl apply -f k8s/configmap.yaml
kubectl apply -f k8s/pvc.yaml
kubectl apply -f k8s/deployment.yaml
```

### Authenticate kiro-cli (first time only)

kiro-cli requires a one-time OAuth login. The PVC persists the tokens across pod restarts.

```bash
kubectl exec -it deployment/agent-broker -- kiro-cli login --use-device-flow
```

Follow the device code flow in your browser, then restart the pod:

```bash
kubectl rollout restart deployment agent-broker
```

### Manifests

| File | Purpose |
|------|---------|
| `k8s/deployment.yaml` | Single-container pod with config + data volume mounts |
| `k8s/configmap.yaml` | `config.toml` mounted at `/etc/agent-broker/` |
| `k8s/secret.yaml` | `DISCORD_BOT_TOKEN` injected as env var |
| `k8s/pvc.yaml` | Persistent storage for auth + settings |

The PVC persists two paths via `subPath`:
- `~/.kiro` — settings, skills, sessions
- `~/.local/share/kiro-cli` — OAuth tokens (`data.sqlite3` → `auth_kv` table), conversation history

## Project Structure

```
├── Dockerfile          # multi-stage: rust build + debian-slim runtime with kiro-cli
├── config.toml.example # example config with all agent backends
├── k8s/                # Kubernetes manifests
│   ├── deployment.yaml
│   ├── configmap.yaml
│   ├── secret.yaml
│   └── pvc.yaml
└── src/
    ├── main.rs         # entrypoint: tokio + serenity + cleanup + shutdown
    ├── config.rs       # TOML config + ${ENV_VAR} expansion
    ├── discord.rs      # Discord bot: mention, threads, edit-streaming
    ├── format.rs       # message splitting (2000 char limit)
    ├── reactions.rs    # status reaction controller (debounce, stall detection)
    └── acp/
        ├── protocol.rs # JSON-RPC types + ACP event classification
        ├── connection.rs # spawn CLI, stdio JSON-RPC communication
        └── pool.rs     # thread_id → AcpConnection map
```

## Inspired By

- [sample-acp-bridge](https://github.com/aws-samples/sample-acp-bridge) — ACP protocol + process pool architecture
- [OpenClaw](https://github.com/openclaw/openclaw) — StatusReactionController emoji pattern

## License

MIT
