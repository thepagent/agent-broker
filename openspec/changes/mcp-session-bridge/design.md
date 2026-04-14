# MCP Session Bridge — Design

## 架構圖

```
Discord User
  │
  ├── /mcp-add name url ──→ profile JSON (data/mcp-profiles/{bot}/{user_id}.json)
  │                                │
  │                                ▼
  ├── 發訊息 ──→ discord.rs ──→ pool.get_or_create(thread_id, user_id)
  │                                │
  │                                ├── 讀 profile JSON
  │                                ├── 組 Vec<McpServerEntry>
  │                                ▼
  │                          connection.session_new(cwd, mcp_servers)
  │                                │
  │                                ▼
  │                          ACP JSON-RPC: session/new
  │                          { "cwd": "...", "mcpServers": [...] }
  │                                │
  │                                ▼
  │                          Backend (Claude/Copilot/Gemini/Codex)
  │                          載入 MCP servers ✅
```

## 關鍵設計決策

### 1. McpServerEntry 用 serde_json::Value flatten

不硬編碼 url/command/args，用 `#[serde(flatten)]` 保留彈性：
- HTTP MCP: `{ "name": "x", "url": "http://..." }`
- stdio MCP: `{ "name": "x", "command": "python3", "args": [...] }`
- 未來新格式不需改 struct

### 2. user_id 傳遞路徑

```
discord.rs message() → extract cmd.user.id
  → pool.get_or_create(thread_id, user_id)
  → read profile(mcp_profiles_dir, user_id)
  → connection.session_new(cwd, mcp_servers)
```

pool.get_or_create() 需要新增 user_id 參數。已有 session 不重新注入（避免重複）。

### 3. Fallback 策略

```rust
// 讀 profile 失敗 → 傳空陣列（不 crash）
let mcp_servers = match read_mcp_profile(&profiles_dir, &user_id) {
    Ok(servers) => servers,
    Err(e) => {
        warn!(user_id, error = %e, "failed to read MCP profile, using empty");
        vec![]
    }
};
```

### 4. Profile JSON 格式（已定義）

```json
{
  "discord_user_id": "844236700611379200",
  "mcpServers": {
    "mempalace": { "url": "http://127.0.0.1:18793/mcp" },
    "notion": { "command": "npx", "args": ["notion-mcp"] }
  },
  "enabled": true
}
```

轉換為 ACP mcpServers 陣列：
```json
[
  { "name": "mempalace", "url": "http://127.0.0.1:18793/mcp" },
  { "name": "notion", "command": "npx", "args": ["notion-mcp"] }
]
```

### 5. 生效時機

- `/mcp-add` → 寫 JSON，**不影響現有 session**
- `/new-session` 或新 thread → 新 session 載入最新 profile
- 不做 hot-reload（複雜度高，收益低）
