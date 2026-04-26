# Browser MCP — agent-browser bridge for OpenAB

Give your OpenAB agents real browser automation via [agent-browser](https://github.com/vercel-labs/agent-browser), without bloating every container with Chrome.

## Architecture

```
Container (openab-*)              Host / Sidecar
┌───────────────────┐            ┌─────────────────────┐
│ browser-mcp-      │── HTTP ──▶ │ browser-bridge.js    │
│ stdio.py          │            │   └─▶ agent-browser  │
│ (MCP stdio proxy) │            │       CLI + Chrome   │
└───────────────────┘            └─────────────────────┘
```

- **browser-bridge.js** — Lightweight Node.js HTTP server that wraps `agent-browser` CLI commands.
- **browser-mcp-stdio.py** — Thin MCP stdio proxy (runs inside the container) that forwards tool calls to the bridge over HTTP.

Containers stay slim. Chrome lives in one place.

## Prerequisites

Install `agent-browser` on the host (or a dedicated sidecar):

```bash
brew install agent-browser        # macOS
# or
npm install -g agent-browser      # any platform
agent-browser install              # download Chrome
```

## Quick Start

### 1. Start the bridge on the host

```bash
./start-browser-bridge.sh
# or manually:
node browser-bridge.js             # listens on :3002
```

### 2. Add to your agent's MCP config

In `~/.kiro/settings/mcp.json` (or the agent JSON's `mcpServers`):

```json
{
  "mcpServers": {
    "browser": {
      "command": "python3",
      "args": ["/path/to/browser-mcp-stdio.py"],
      "env": {
        "BROWSER_BRIDGE_URL": "http://host.docker.internal:3002/message"
      }
    }
  }
}
```

### 3. Add `@browser` to the agent's tools list

```json
{
  "tools": ["read", "write", "shell", "web_search", "web_fetch", "@browser"]
}
```

## Available Tools

| Tool | Description |
|------|-------------|
| `browse_url` | Open a URL, wait for load, return accessibility snapshot |
| `browse_snapshot` | Get current page snapshot (optionally scoped by CSS selector) |
| `browse_click` | Click an element by `@ref` from the snapshot |
| `browse_type` | Clear and fill a text field by `@ref` |
| `browse_scroll` | Scroll the page (up/down/left/right) |
| `browse_screenshot` | Take a screenshot |
| `browse_get_text` | Get text content of an element |
| `browse_close` | Close the browser session |

## Recommended Agent Prompt Addition

Add this to your agent's system prompt so it automatically falls back to `@browser` when `web_fetch` fails:

```
## Browser Automation (@browser MCP)
When web_fetch fails (JS-rendered pages, Cloudflare blocks, empty content),
use @browser tools instead:
- browse_url(url) → opens page, returns accessibility snapshot with @refs
- browse_click(ref) → click element
- browse_type(ref, text) → fill text field
- browse_scroll(direction, pixels) → scroll page
- browse_close() → close when done
Order: web_fetch → fails → @browser/browse_url → interact → browse_close
```

## Configuration

| Environment Variable | Default | Description |
|---------------------|---------|-------------|
| `BROWSER_BRIDGE_PORT` | `3002` | Port for the HTTP bridge |
| `AGENT_BROWSER_PATH` | `/opt/homebrew/bin/agent-browser` | Path to agent-browser binary |
| `BROWSER_BRIDGE_URL` | `http://127.0.0.1:3002/message` | Bridge URL (for the stdio proxy) |

## K8s Deployment

Run the bridge as a sidecar or a shared service. The bridge is stateless — one instance can serve multiple agents. Use a `ClusterIP` Service and point `BROWSER_BRIDGE_URL` at it.
