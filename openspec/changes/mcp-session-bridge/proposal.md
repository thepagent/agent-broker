# MCP Session Bridge

## 目標

讓 Discord `/mcp-add` 新增的 MCP server 在下次 ACP session 建立時自動注入 backend，
完成從「UI 管理 → 實際生效」的閉環。目前 `/mcp-add` 只寫 profile JSON，backend 不會讀取。

## 現況

- `connection.rs:329` — `session/new` 已有 `mcpServers: []` 欄位，傳空陣列
- `connection.rs:526` — `session/load` 同上
- `discord.rs` — `/mcp-add` `/mcp-remove` `/mcp-list` 等 6 指令已實作，讀寫 `{mcp_profiles_dir}/{user_id}.json`
- Profile JSON 格式：`{ "discord_user_id": "...", "mcpServers": { "name": { "url": "..." } }, "enabled": true }`

## 方案

### 資料流

```
用戶 /mcp-add mempalace http://127.0.0.1:18793/mcp
  → data/mcp-profiles/{bot}/{user_id}.json 寫入
  → 下次 get_or_create() 建 session
  → pool.rs 讀 profile → 組 mcpServers Vec
  → connection.session_new(cwd, mcp_servers)
  → ACP session/new { cwd, mcpServers: [{name, url}] }
  → backend 自動載入 MCP ✅
```

### 改動範圍

| 檔案 | 改動 | 預估行數 |
|------|------|---------|
| `src/acp/connection.rs` | `session_new()` 和 `session_load()` 接受 `mcp_servers: Vec<McpServerEntry>` 參數，替換硬編碼 `[]` | ~20 |
| `src/acp/pool.rs` | `get_or_create()` 讀 profile JSON，組 McpServerEntry，傳給 connection | ~30 |
| `src/config.rs` | 新增 `McpServerEntry` struct（name + url）| ~10 |
| `src/discord.rs` | 不需改（已有 /mcp-* handler）| 0 |

### McpServerEntry 格式

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerEntry {
    pub name: String,
    #[serde(flatten)]
    pub config: serde_json::Value, // 保留彈性：url / command+args 都支援
}
```

傳入 ACP 時序列化為：
```json
{
  "mcpServers": [
    { "name": "mempalace", "url": "http://127.0.0.1:18793/mcp" },
    { "name": "notion", "command": "npx", "args": ["notion-mcp"] }
  ]
}
```

## 風險與待確認

1. **各 backend ACP 是否支援 mcpServers**
   - Copilot CLI：✅ 已確認（copilot-agent-acp bridge 處理）
   - Claude Code：需驗證 `claude-agent-acp` 是否透傳 mcpServers
   - Gemini CLI：需驗證 `gemini --acp` 是否支援
   - Codex CLI：需驗證 `codex-acp` 是否支援
   - **驗證方法**：手動送 ACP JSON-RPC `session/new` 帶 mcpServers，觀察 backend 行為

2. **HTTP vs stdio MCP 格式差異**
   - HTTP：`{ "url": "http://..." }`
   - stdio：`{ "command": "python3", "args": ["-m", "mempalace.mcp_server"] }`
   - 用 `serde_json::Value` flatten 保留彈性，不硬編碼格式

3. **多用戶場景**
   - Profile 按 Discord user ID 分檔，但 OpenAB pool 按 thread_id 分 session
   - `get_or_create()` 需要 user_id 參數來讀對應 profile
   - 目前 `get_or_create()` 簽名沒有 user_id → 需要從 discord handler 傳入

4. **Profile 變更後的生效時機**
   - `/mcp-add` 後不會立即對現有 session 生效
   - 需要 `/new-session` 或下個 thread 才會載入新 profile
   - 可接受，在 `/mcp-add` 回覆中提示用戶

## 不做的事

- 不修改 backend 本機 config 檔（`~/.claude.json` 等）
- 不做 session 中途 hot-reload MCP
- 不做 MCP server 健康檢查（已有 `/mcp-status`）
- 不處理 MCP server 認證（留給各 MCP server 自己處理）

## 成功標準

1. `/mcp-add mempalace http://127.0.0.1:18793/mcp` → `/new-session` → backend 可用 mempalace 工具
2. `/mcp-remove mempalace` → `/new-session` → backend 不再有 mempalace
3. 對不支援 mcpServers 的 backend，graceful fallback（忽略，不 crash）
