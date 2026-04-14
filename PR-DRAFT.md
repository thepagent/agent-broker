## What problem does this solve?

Users can manage MCP servers via Discord slash commands (`/mcp-add`, `/mcp-remove`, `/mcp-list`), but those servers are never actually loaded into ACP sessions. The profile JSON is written but the backend (Claude/Copilot/Gemini/Codex) doesn't read it — making `/mcp-add` a dead-end UI with no runtime effect.

This PR completes the "UI management → runtime activation" loop.

Closes # <!-- link issue if one exists -->

## At a Glance

```
Discord User
  │
  ├── /mcp-add mempalace http://...
  │       ↓
  │   data/mcp-profiles/{bot}/{user_id}.json   ← existing (write-only)
  │       │
  │       ▼
  ├── sends message ──→ discord.rs ──→ pool.get_or_create(thread, mcp_servers)
  │                                         │
  │                            ┌─────────────┘
  │                            ▼
  │                     connection.session_new(cwd, mcp_servers)  ← NEW
  │                            │
  │                            ▼
  │                     ACP JSON-RPC: session/new
  │                     { "cwd": "...", "mcpServers": [{name, type, url, headers}] }
  │                            │
  │                            ▼
  │                     Backend (Claude/Copilot/Gemini/Codex)
  │                     loads MCP servers into session ✅
```

## Prior Art & Industry Research

**OpenClaw:**

OpenClaw uses a two-layer merge architecture for MCP:
- **Bundle layer**: plugin/skill `.mcp.json` defaults
- **Global config layer**: `openclaw.json` `mcp.servers` (overrides bundle)

Both layers are merged at session creation in `embedded-pi-mcp.ts`, producing a `SessionMcpRuntime` cached by `sessionId + configFingerprint(SHA1)`. When config changes, stale runtimes are auto-disposed and recreated.

Key differences from our approach:
- OpenClaw uses a **global single config** — no per-user/per-channel separation. Anyone with owner+admin scope writes to the same namespace.
- For CLI backends (Claude Code, Codex), OpenClaw injects MCP via `--mcp-config` CLI args (`injectClaudeMcpConfigArgs()`), not via ACP `session/new`.
- Source: `src/agents/embedded-pi-mcp.ts`, `src/agents/pi-bundle-mcp-runtime.ts`, `src/config/mcp-config.ts`

**Hermes Agent:**

Hermes Agent has first-class MCP client support with a dedicated daemon thread running a persistent asyncio event loop per server:
- **Global config**: `~/.hermes/config.yaml` under `mcp_servers` key, loaded at startup.
- **ACP session injection**: `new_session`/`load_session` accept `mcp_servers` parameter — but registered tools go into the **process-wide singleton `ToolRegistry`**, not per-session isolation.
- Tools are namespaced as `mcp_<server>_<tool>` and merged into umbrella toolsets.
- Supports `tools/list_changed` notifications for live refresh, and `/reload-mcp` slash command for hot-reload.
- Security: filtered subprocess env, credential stripping from errors, OSV malware check before spawn.

Key differences:
- Hermes accepts `mcp_servers` in ACP `session/new` (same approach as this PR), but merges into a global registry — no per-user isolation.
- Hermes has rich CLI management (`hermes mcp add` with interactive curses wizard), while OpenAB uses Discord slash commands.
- Source: `tools/mcp_tool.py`, `hermes_cli/mcp_config.py`, `acp_adapter/server.py`

**Comparison:**

| Aspect | OpenClaw | Hermes Agent | OpenAB (this PR) |
|--------|----------|-------------|-----------------|
| MCP config scope | Global single | Global + ACP injection | **Per-user per-bot** |
| Injection point | Config file merge + CLI args | ACP session/new params | ACP session/new params |
| User isolation | None | None (shared registry) | **Yes** (profile per Discord user) |
| Management UI | Chat `/mcp set` + CLI | CLI `hermes mcp add` | Discord `/mcp-*` slash commands |
| Hot-reload | Config fingerprint change | `/reload-mcp` + notifications | New session only |

## Proposed Solution

Add `mcp_servers` parameter threading through the session creation path:

1. **`config.rs`**: `McpServerEntry` struct + `read_mcp_profile()` reads `{mcp_profiles_dir}/{user_id}.json`
2. **`connection.rs`**: `session_new()` and `session_load()` accept `&[serde_json::Value]` for mcpServers
3. **`pool.rs`**: `get_or_create()` accepts and passes through `mcp_servers`
4. **`discord.rs`**: `mcp_servers_for_user()` helper reads profile and builds the JSON array; message handler + session-creating commands (`/native`, `/plan`, `/mcp`, `/compact`) pass user's MCP servers; diagnostic commands (`/doctor`, `/stats`, `/tokens`) pass `&[]`

Profile JSON format (written by existing `/mcp-add`):
```json
{
  "discord_user_id": "844236700611379200",
  "mcpServers": {
    "mempalace": { "type": "http", "url": "http://...", "headers": [] }
  },
  "enabled": true
}
```

Converted to ACP format:
```json
[{ "name": "mempalace", "type": "http", "url": "http://...", "headers": [] }]
```

## Why this approach?

1. **ACP-native injection** — all 4 backends already accept `mcpServers` in `session/new` (verified via testing). No config file manipulation needed.
2. **Per-user isolation** — unlike OpenClaw (global) and Hermes (global registry), our profiles are keyed by Discord user ID, so different users get different MCP servers.
3. **Per-bot isolation** — each bot has its own `mcp_profiles_dir`, so CICX and GITX can have different MCP configurations.
4. **Graceful fallback** — `read_mcp_profile()` returns empty vec on any error. Backends that don't support `mcpServers` simply ignore the parameter (empty array is always valid).
5. **Comprehensive review** — 7 rounds of Codex CLI review with all P1 findings fixed. Security (allowlists), correctness (permission protocol), and portability (no hardcoded paths) all addressed.

## Alternatives Considered

1. **Config file manipulation** (like OpenClaw's `injectClaudeMcpConfigArgs`): Rejected — modifying `~/.claude.json` or `~/.copilot/mcp-config.json` is fragile, backend-specific, and risks breaking user's existing config.

2. **Global singleton registry** (like Hermes): Rejected — OpenAB runs as a Discord bot where multiple users share the same process. Global MCP would leak one user's tools to another.

3. **Hot-reload within session**: Deferred — would require `session/update` or custom RPC. Current approach (effective on next session) is simpler and matches both OpenClaw and Hermes behavior.

## Validation

- [x] `cargo check` passes
- [x] `cargo build --release` — 0 errors, 2 pre-existing warnings
- [x] `cargo test` — 31 passed, 0 failed
- [x] `cargo fmt` — all files formatted
- [x] `cargo clippy` — 0 new warnings
- [x] Format verification: all 4 backends (Claude, Copilot, Gemini, Codex) accept `[{name, type:"http", url, headers:[]}]` in `session/new`
- [x] E2E test: Claude ACP — profile → session/new(mcpServers) → ToolSearch → mempalace_search → GPU data ✅
- [x] Codex CLI review — 7 rounds, 14 P1 + 13 P2 findings. Fixed 14 P1 + 10 P2. Remaining 3 P2 are architectural known-limitations (not regressions)
- [x] Test scripts: `scripts/test-mcp-acp-v3.js` (format matrix), `scripts/test-e2e-final.js` (end-to-end)

### Additional fixes from Codex review (not in original scope)

| Fix | Impact |
|-----|--------|
| Restored upstream `pick_best_option` + `build_permission_response` for ACP permissions | Correct tool approval for all backends |
| `kill_on_drop(true)` on ACP child processes | Windows subprocess cleanup |
| Copilot SDK/CLI paths dynamically resolved | Works on any machine, any Copilot version |
| `copilot_rpc_script_path()` resolves exe-relative | Removes all hardcoded local paths from Rust |
| Copilot bridge merges ACP request mcpServers with file-based config | Per-user MCP works for CopilotBridge |
| All slash commands gated with `copilot_guard_ok()` | Channel/user allowlist enforced everywhere |
| `cleanup_idle` uses `saturating_duration_since` | Prevents Instant underflow panic on Windows reboot |
| Native command caching via `tokio::spawn` | No 2s latency on first message |
| Copilot model refresh gated on `has_copilot_rpc()` | Non-Copilot bots keep correct model cache |
| `session_load()` parses model metadata | `/model` works in resumed sessions |

<details>
<summary>E2E test output (Claude ACP)</summary>

```
=== E2E: claude | 1 MCP server(s) ===

1. Init OK
2. Session: f58571ab-9c90-47
3. Waited 8s for MCP
4. Prompting: "search mempalace for GPU"...
5. Prompt accepted (response id=3)

6. Analysis:
   Notifications received: 23
   Contains 'mempalace': true
   Contains GPU/VRAM: true
   [tool_call] ToolSearch → mcp__plugin_mempalace_mempalace__mempalace_search
   [tool_call] mempalace_search(query: "GPU")
   [agent_message] GPU: NVIDIA RTX 2000 Ada Laptop... irisx-gpu-switch...

>>> MCP BRIDGE FULLY WORKING ✅ <<<
```

</details>

<details>
<summary>Format compatibility matrix (all 4 backends)</summary>

```
=== claude ===
array-named-http:  OK ✅
array-named-cmd:   OK ✅

=== copilot ===
http:              OK ✅  (all formats accepted)
sse:               OK ✅
stdio:             OK ✅

=== gemini ===
array-named-http:  OK ✅
array-named-cmd:   OK ✅

=== codex ===
array-named-http:  OK ✅
array-named-cmd:   OK ✅
```

</details>
