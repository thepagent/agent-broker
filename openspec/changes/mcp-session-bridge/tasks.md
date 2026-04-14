# MCP Session Bridge — Tasks

## Phase 1: 驗證 ACP mcpServers 支援

- [x] T1: 手動測試 Claude ACP — `[{name, type:"http", url, headers:[]}]` 格式通過 ✅
- [x] T2: 手動測試 Gemini ACP — 同格式通過 ✅
- [x] T3: 手動測試 Codex ACP — 同格式通過 ✅
- [x] T4: 記錄格式差異 — `name` 必填、必須 array、不能省略 mcpServers 欄位

## Phase 2: Rust 實作

- [x] T5: `src/config.rs` — McpServerEntry struct + read_mcp_profile()
- [x] T6: `src/acp/connection.rs` — session_new/session_load 加 mcp_servers 參數
- [x] T7: `src/acp/pool.rs` — get_or_create 加 mcp_servers 參數，透傳到 connection
- [x] T8: `src/discord.rs` — mcp_servers_for_user() helper + 主 handler 及 /native /plan /mcp /compact 帶 user MCP
- [x] T9: 診斷型指令（/doctor /stats /tokens /usage /permissions）傳 &[] — 不需 MCP 注入

## Phase 3: 測試與 UX

- [ ] T10: `/mcp-add` 回覆加提示「下次新 session 生效，用 /new-session 立即套用」
- [x] T11: Claude E2E — profile → session/new(mcpServers) → ToolSearch → mempalace_search → GPU data ✅
- [x] T12: `cargo build --release` 通過（0 errors, 3 pre-existing warnings）
- [x] T13: Bot 重啟驗證

## Phase 4: PR

- [ ] T14: 開 feature branch `feat/mcp-session-bridge`
- [ ] T15: Cherry-pick src/ 改動（不含個人 config/bat/scripts）
- [ ] T16: 按 PR #302 guidelines 格式推 PR（含 Prior Art 研究 + ASCII 圖 + 對比表）
