# GitHub Copilot CLI — Agent Backend Guide

How to run OpenAB with [GitHub Copilot CLI](https://github.com/github/copilot-cli) as the agent backend.

## Prerequisites

- An active [GitHub Copilot](https://github.com/features/copilot/plans) subscription (Free, Pro, Pro+, Business, or Enterprise)
- Copilot CLI v1.0.24+ (`npm install -g @github/copilot`)
- ACP support is in [public preview](https://github.blog/changelog/2026-01-28-acp-support-in-copilot-cli-is-now-in-public-preview/) since Jan 28, 2026

## Architecture

```
┌──────────────┐  Gateway WS   ┌──────────────┐  ACP stdio    ┌──────────────────────┐
│   Discord    │◄─────────────►│ openab       │──────────────►│ copilot --acp         │
│   User       │               │   (Rust)     │◄── JSON-RPC ──│ (Copilot CLI)         │
└──────────────┘               └──────────────┘               └──────────────────────┘
```

OpenAB spawns `copilot --acp` as a child process and communicates via stdio JSON-RPC. No intermediate adapter needed.

## Configuration

### config.toml

```toml
[agent]
command = "copilot"
args = ["--acp"]
working_dir = "/home/agent"
```

## Docker

Build with the Copilot-specific Dockerfile:

```bash
docker build -f Dockerfile.copilot -t openab-copilot .
```

## Authentication

Copilot CLI uses GitHub OAuth (same as `gh` CLI). In a headless container, use device flow:

```bash
# 1. Exec into the running pod/container
kubectl exec -it deployment/openab-copilot -- bash

# 2. Authenticate via device flow
gh auth login --hostname github.com --git-protocol https -p https -w

# 3. Follow the device code flow in your browser

# 4. Verify
gh auth status

# 5. Restart the pod (token is persisted via PVC)
kubectl rollout restart deployment/openab-copilot
```

The OAuth token is stored under `~/.config/gh/` and persisted across pod restarts via PVC.

> **Note**: See [gh-auth-device-flow.md](gh-auth-device-flow.md) for details on device flow in headless environments.

## Helm Install

```bash
helm install openab openab/openab \
  --set agents.kiro.enabled=false \
  --set agents.copilot.discord.botToken="$DISCORD_BOT_TOKEN" \
  --set-string 'agents.copilot.discord.allowedChannels[0]=YOUR_CHANNEL_ID' \
  --set agents.copilot.image=ghcr.io/openabdev/openab-copilot:latest \
  --set agents.copilot.command=copilot \
  --set 'agents.copilot.args={--acp}' \
  --set agents.copilot.workingDir=/home/node
```

## Verified Capabilities

Tested end-to-end with `copilot --acp` v1.0.24:

| Feature | Status | Details |
|---|---|---|
| `initialize` | ✅ | Returns agentInfo |
| `session/new` | ✅ | 3 modes, 8 models, configOptions=true |
| `session/prompt` | ✅ | Streaming via `agent_message_chunk` |
| `session/set_model` | ✅ | 8 available models |
| `session/set_mode` | ✅ | agent / plan / autopilot |
| `session/request_permission` | ✅ | Auto-approved by OpenAB |
| `usage_update` notification | ✅ | Emits `used` (tokens) and `size` (context window) |
| `session/list` | ✅ | Lists past sessions |
| `session/load` | ✅ | Loads a previous session |

### Available Models (v1.0.24)

Models available depend on your Copilot plan and may change. The list below is from a Pro plan tested on 2026-04-13. Use `/model` in Discord to see your actual available models.

| Model | Provider |
|---|---|
| Claude Opus 4.6 | Anthropic |
| Claude Sonnet 4.6 | Anthropic |
| Claude Haiku 4.5 | Anthropic |
| GPT-5.3-Codex | OpenAI |
| GPT-5-mini | OpenAI |
| Gemini 3 Pro | Google |
| Gemini 3 Flash | Google |
| o3-mini | OpenAI |

### Available Modes

| Mode | Description |
|---|---|
| `agent` | Default — full tool access |
| `plan` | Read-only planning mode |
| `autopilot` | Auto-approve all tool calls |

## Known Limitations

- ⚠️ ACP support is in **public preview** — behavior may change
- `usage_update` notifications are not emitted — context window tracking shows 0/0
- Headless auth with `GITHUB_TOKEN` env var is not fully validated; device flow via `gh auth login` is recommended
- Copilot CLI requires an active Copilot subscription per user/org
- For Copilot Business/Enterprise, an admin must enable Copilot CLI from the Policies page
