# MCP Session Bridge — Tasks

## Phase 1: 驗證 ACP mcpServers 支援

- [ ] T1: 手動測試 Claude ACP — 送 `session/new` 帶 `mcpServers: [{name:"test", url:"http://127.0.0.1:18793/mcp"}]`，確認 backend 行為
- [ ] T2: 手動測試 Gemini ACP — 同上
- [ ] T3: 手動測試 Codex ACP — 同上
- [ ] T4: 記錄各 backend 支援狀態和 mcpServers 格式差異

## Phase 2: Rust 實作

- [ ] T5: `src/config.rs` — 新增 `McpServerEntry` struct
- [ ] T6: `src/acp/connection.rs` — `session_new()` 和 `session_load()` 改接受 `mcp_servers` 參數
- [ ] T7: `src/acp/pool.rs` — `get_or_create()` 加 `user_id` 參數，讀 profile JSON，組 McpServerEntry Vec
- [ ] T8: `src/discord.rs` — message handler 傳 user_id 到 get_or_create()
- [ ] T9: 不支援 mcpServers 的 backend graceful fallback（空陣列或跳過）

## Phase 3: 測試與 UX

- [ ] T10: `/mcp-add` 回覆加提示「下次新 session 生效，用 /new-session 立即套用」
- [ ] T11: 端到端測試：/mcp-add → /new-session → 確認 MCP 工具可用
- [ ] T12: `cargo build --release` 編譯通過
- [ ] T13: 4 個 bot 重啟驗證

## Phase 4: PR

- [ ] T14: 開 feature branch `feat/mcp-session-bridge`
- [ ] T15: Cherry-pick src/ 改動（不含個人 config/bat/scripts）
- [ ] T16: 推 PR 到 upstream openabdev/openab
