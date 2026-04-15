# Ollama (Local AI via acp-bridge)

Use your local [Ollama](https://ollama.com) as an agent backend through [acp-bridge](https://github.com/BlakeHung/acp-bridge) — a lightweight Rust binary that bridges Ollama's native API to ACP.

**Zero API cost. Data never leaves your machine.**

```
┌──────────┐     ┌────────┐     ┌────────────┐     ┌────────┐
│ Discord  │ ──► │ openab │ ──► │ acp-bridge │ ──► │ Ollama │
│ User     │     │        │     │  (5MB Rust) │     │ (GPU)  │
└──────────┘     └────────┘     │            │     └────────┘
                                │  tools:    │
                                │  read_file │
                                │  list_dir  │
                                │  search    │
                                └────────────┘
```

## Prerequisites

1. [Ollama](https://ollama.com) installed and running
2. A model pulled (e.g. `ollama pull gemma4:26b`)
3. [acp-bridge](https://github.com/BlakeHung/acp-bridge) installed

```bash
# Install acp-bridge
cargo install acp-bridge

# Or build from source
git clone https://github.com/BlakeHung/acp-bridge
cd acp-bridge && cargo build --release
```

## Manual config.toml

```toml
[discord]
bot_token = "${DISCORD_BOT_TOKEN}"
allowed_channels = ["YOUR_CHANNEL_ID"]

[agent]
command = "acp-bridge"
args = []
working_dir = "/path/to/your/project"
env = { LLM_BASE_URL = "http://localhost:11434", LLM_MODEL = "gemma4:26b" }

[pool]
max_sessions = 3
session_ttl_hours = 24

[reactions]
enabled = true
```

> **Tip:** Use `LLM_BASE_URL=http://localhost:11434` (without `/v1`) to enable Ollama native mode. acp-bridge will use `/api/chat` with NDJSON streaming, query model context length via `/api/show`, and check VRAM status via `/api/ps`.

## Run

```bash
export DISCORD_BOT_TOKEN="your-token"
cargo run --release -- config.toml
```

## Built-in Tools

acp-bridge v0.5.0+ includes built-in tools that let the LLM interact with files in the working directory:

| Tool | Description | Limits |
|------|-------------|--------|
| `read_file` | Read file contents | Max 1MB, sandboxed |
| `list_dir` | List directory tree | Max depth 3, 200 entries |
| `search_code` | Grep for patterns | Max 50 matches |

All tools are **sandboxed** to the `working_dir` configured above — the LLM cannot access files outside it.

Ask in Discord: *"What's the structure of this project?"* — and the bot will actually read your source code and answer.

## Model Recommendations

| Hardware | RAM | Recommended Model | Command |
|----------|-----|-------------------|---------|
| Desktop GPU (RTX 3090/4090) | 24GB | `gemma4:26b` | `ollama pull gemma4:26b` |
| MacBook Air M2/M3 | 8-16GB | `llama3.2:7b` | `ollama pull llama3.2:7b` |
| MacBook Pro M3/M4 | 18-24GB | `gemma4:26b` | `ollama pull gemma4:26b` |
| MacBook Pro M4 Pro | 48GB | `qwen2.5:32b` | `ollama pull qwen2.5:32b` |
| Mac Studio M4 Ultra | 64-192GB | `llama3.1:70b` | `ollama pull llama3.1:70b` |

## Environment Variables

acp-bridge supports these environment variables (set in the `env` table of your config):

| Variable | Default | Description |
|----------|---------|-------------|
| `LLM_BASE_URL` | `http://localhost:11434/v1` | Ollama endpoint (use without `/v1` for native mode) |
| `LLM_MODEL` | `gemma4:26b` | Model name |
| `LLM_TEMPERATURE` | (model default) | Sampling temperature (0.0-2.0) |
| `LLM_MAX_TOKENS` | (model default) | Max tokens to generate |
| `LLM_TIMEOUT` | `300` | HTTP timeout in seconds |
| `LLM_MAX_HISTORY_TURNS` | `50` | Max conversation turns (0 = unlimited) |
| `LLM_MAX_SESSIONS` | `0` | Max concurrent sessions (0 = unlimited) |
| `LLM_SESSION_IDLE_TIMEOUT` | `0` | Auto-evict idle sessions after N seconds |

## Multi-Bot Setup

Run multiple Discord bots with different Ollama models:

```toml
# config-coder.toml — fast coding model
[agent]
command = "acp-bridge"
env = { LLM_BASE_URL = "http://localhost:11434", LLM_MODEL = "qwen2.5:32b" }

# config-reviewer.toml — analytical model
[agent]
command = "acp-bridge"
env = { LLM_BASE_URL = "http://localhost:11434", LLM_MODEL = "gemma4:26b" }
```

## Remote Ollama

If Ollama runs on a different machine (e.g. a GPU server):

```toml
[agent]
command = "acp-bridge"
env = { LLM_BASE_URL = "http://gpu-server:11434", LLM_MODEL = "llama3.1:70b" }
```

Make sure the Ollama server allows remote connections (`OLLAMA_HOST=0.0.0.0`).

## Links

- [acp-bridge GitHub](https://github.com/BlakeHung/acp-bridge)
- [acp-bridge on crates.io](https://crates.io/crates/acp-bridge)
- [Ollama](https://ollama.com)
