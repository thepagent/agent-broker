# Browser Automation via agent-browser

OpenAB agents can use [agent-browser](https://github.com/vercel-labs/agent-browser) for real browser automation — useful when `web_fetch` fails on JavaScript-heavy or Cloudflare-protected pages.

## How It Works

A lightweight HTTP bridge runs on the host (or as a sidecar), wrapping the `agent-browser` CLI. Containers connect via a thin MCP stdio proxy — no Chrome needed inside containers.

See [`contrib/browser-mcp/`](../contrib/browser-mcp/) for the bridge, proxy, and setup instructions.

## Quick Setup

1. Install `agent-browser` on the host: `brew install agent-browser`
2. Start the bridge: `node contrib/browser-mcp/browser-bridge.js`
3. Add `browser` to your agent's MCP config pointing at the bridge
4. Add `@browser` to the agent's tools list
5. Optionally add the fallback prompt so agents auto-switch from `web_fetch` to `@browser`

## Tools

- `browse_url(url)` — Open URL, return accessibility snapshot
- `browse_click(ref)` — Click element by @ref
- `browse_type(ref, text)` — Fill text field
- `browse_scroll(direction, pixels)` — Scroll page
- `browse_snapshot(selector?)` — Re-read current page
- `browse_get_text(ref)` — Get element text
- `browse_screenshot()` — Take screenshot
- `browse_close()` — Close browser session
