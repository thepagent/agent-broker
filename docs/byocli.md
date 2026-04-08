# Bring Your Own CLI (BYOCLI)

openab works with any CLI that speaks ACP (Agent Client Protocol) over stdio. This guide explains how to build, configure, and test your own ACP-compatible CLI.

## How it works

```
Discord user ──▶ openab ──stdin/stdout──▶ Your CLI ──▶ Any AI backend
                          (JSON-RPC 2.0)
```

openab spawns your CLI as a child process and communicates via **newline-delimited JSON-RPC 2.0** over stdin/stdout. Your CLI needs to implement three methods and emit streaming notifications.

## ACP protocol requirements

### Message format

Every message is a single line of JSON followed by `\n`. No Content-Length framing — just newline-delimited JSON.

### Methods your CLI must handle

#### 1. `initialize`

Called once after spawn. Handshake to exchange capabilities.

**Request (stdin):**
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "initialize",
  "params": {
    "protocolVersion": 1,
    "clientCapabilities": {},
    "clientInfo": { "name": "openab", "version": "0.1.0" }
  }
}
```

**Response (stdout):**
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "agentInfo": { "name": "my-agent", "version": "1.0.0" },
    "capabilities": {}
  }
}
```

**Timeout:** 30 seconds.

#### 2. `session/new`

Creates a new conversation session.

**Request:**
```json
{
  "jsonrpc": "2.0",
  "id": 2,
  "method": "session/new",
  "params": {
    "cwd": "/path/to/working/directory",
    "mcpServers": []
  }
}
```

**Response:**
```json
{
  "jsonrpc": "2.0",
  "id": 2,
  "result": {
    "sessionId": "unique-session-id"
  }
}
```

**Timeout:** 120 seconds (longer due to potential model loading).

#### 3. `session/prompt`

Sends a user message and receives streaming responses.

**Request:**
```json
{
  "jsonrpc": "2.0",
  "id": 3,
  "method": "session/prompt",
  "params": {
    "sessionId": "unique-session-id",
    "prompt": [
      { "type": "text", "text": "Hello, help me with this code" }
    ]
  }
}
```

**Response:** Your CLI should emit **notifications** (see below) as it processes, then send a final response:
```json
{
  "jsonrpc": "2.0",
  "id": 3,
  "result": { "status": "completed" }
}
```

### Notifications your CLI should emit

During `session/prompt` processing, emit these notifications to stdout (no `id` field):

#### Text output (streaming)
```json
{
  "jsonrpc": "2.0",
  "method": "session/notify",
  "params": {
    "update": {
      "sessionUpdate": "agent_message_chunk",
      "content": { "text": "chunk of response text" }
    }
  }
}
```

openab collects these chunks and live-updates the Discord message every 1.5 seconds.

#### Thinking indicator
```json
{
  "jsonrpc": "2.0",
  "method": "session/notify",
  "params": {
    "update": { "sessionUpdate": "agent_thought_chunk" }
  }
}
```

Triggers the 🤔 reaction in Discord.

#### Tool call start
```json
{
  "jsonrpc": "2.0",
  "method": "session/notify",
  "params": {
    "update": {
      "sessionUpdate": "tool_call",
      "title": "tool_name"
    }
  }
}
```

Triggers tool-specific reactions (🔥 for general, 👨‍💻 for coding, ⚡ for web).

#### Tool call completion
```json
{
  "jsonrpc": "2.0",
  "method": "session/notify",
  "params": {
    "update": {
      "sessionUpdate": "tool_call_update",
      "title": "tool_name",
      "status": "completed"
    }
  }
}
```

`status` can be `"completed"` or `"failed"`.

### Permission requests (optional)

If your CLI needs to request permission for sensitive operations, send:

```json
{
  "jsonrpc": "2.0",
  "id": 100,
  "method": "session/request_permission",
  "params": {
    "toolCall": { "title": "tool_name" }
  }
}
```

openab auto-replies with:
```json
{
  "jsonrpc": "2.0",
  "id": 100,
  "result": { "optionId": "allow_always" }
}
```

## Configuring openab

Add your CLI to `config.toml`:

```toml
[agent]
command = "my-cli"          # executable name or full path
args = ["--some-flag"]      # command-line arguments
working_dir = "/tmp"        # working directory for the agent
env = { MY_KEY = "value" }  # extra environment variables
```

Environment variables support `${VAR}` expansion from the host environment.

## Testing your CLI

### Step 1: Manual JSON-RPC test

Run your CLI directly and paste JSON-RPC messages:

```bash
./my-cli
```

Paste line by line:

```json
{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":1,"clientCapabilities":{},"clientInfo":{"name":"test","version":"0.1.0"}}}
```

Expected: a response with `agentInfo`.

```json
{"jsonrpc":"2.0","id":2,"method":"session/new","params":{"cwd":"/tmp","mcpServers":[]}}
```

Expected: a response with `sessionId`.

Then use the session ID:

```json
{"jsonrpc":"2.0","id":3,"method":"session/prompt","params":{"sessionId":"YOUR_SESSION_ID","prompt":[{"type":"text","text":"Say hello"}]}}
```

Expected: streaming `agent_message_chunk` notifications, followed by a final response with `id: 3`.

### Step 2: Test with openab

```bash
export DISCORD_BOT_TOKEN="your-token"

cat > config.toml <<EOF
[discord]
bot_token = "${DISCORD_BOT_TOKEN}"
allowed_channels = ["YOUR_CHANNEL_ID"]

[agent]
command = "/path/to/my-cli"
args = []
working_dir = "/tmp"
EOF

cargo run -- config.toml
```

Then `@mention` the bot in Discord and verify the response appears.

## Session lifecycle

```
openab spawns your CLI
    │
    ▼
initialize ──▶ response with agentInfo
    │
    ▼
session/new ──▶ response with sessionId
    │
    ▼
session/prompt ──▶ notifications... ──▶ final response
    │                    (streaming)
    ▼
session/prompt ──▶ ... (multi-turn, same sessionId)
    │
    ▼
(idle timeout or shutdown) ──▶ process killed
```

- One CLI process per Discord thread
- Sessions persist for multi-turn conversations
- Process is killed on idle timeout (`session_ttl_hours`, default 24h)
- `kill_on_drop` is enabled — process dies when openab shuts down

## Example: Ollama via local-ai-acp

[local-ai-acp](https://github.com/BlakeHung/local-ai-acp) is a community BYOCLI that bridges any OpenAI-compatible local AI service to ACP. Written in Rust, single binary, no runtime dependencies.

### Supported backends

Any service exposing `/v1/chat/completions` with SSE streaming:

- [Ollama](https://ollama.com) (default, `http://localhost:11434/v1`)
- [LocalAI](https://localai.io) (`http://localhost:8080/v1`)
- [vLLM](https://docs.vllm.ai) (`http://localhost:8000/v1`)
- [llama.cpp server](https://github.com/ggml-org/llama.cpp) (`http://localhost:8080/v1`)
- [LM Studio](https://lmstudio.ai) (`http://localhost:1234/v1`)

### Install

```bash
cargo install --git https://github.com/BlakeHung/local-ai-acp
```

### Configure

```toml
[agent]
command = "local-ai-acp"
args = []
working_dir = "/tmp"
env = { LLM_BASE_URL = "http://localhost:11434/v1", LLM_MODEL = "gemma4:26b" }
```

### Resource requirements

| Model | RAM required | Use case |
|-------|-------------|----------|
| `qwen2.5:7b` | ~4 GB | Lightweight, fast responses |
| `gemma4:26b` | ~16 GB | General purpose, good quality |
| `qwen2.5:32b` | ~20 GB | Code generation, detailed analysis |
| `llama3.1:70b` | ~48 GB | Maximum quality |

### Run

```bash
# 1. Start Ollama
ollama serve
ollama pull gemma4:26b

# 2. Start openab with local-ai-acp
export DISCORD_BOT_TOKEN="your-token"
./target/release/openab config.toml
```

Anyone in your Discord server can now `@mention` the bot and get AI responses powered by your local GPU — zero API cost, fully offline.

## Building your own BYOCLI

A minimal BYOCLI implementation needs to:

1. Read newline-delimited JSON-RPC from **stdin**
2. Write newline-delimited JSON-RPC to **stdout**
3. Handle `initialize`, `session/new`, `session/prompt`
4. Emit `agent_message_chunk` notifications during prompt processing
5. Send a final response (with matching `id`) when done
6. Support multi-turn conversations (same `sessionId`)
7. Exit cleanly on stdin EOF

For a complete reference implementation, see [local-ai-acp source code](https://github.com/BlakeHung/local-ai-acp/tree/main/src).
