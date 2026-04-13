# Proposal: OpenAB Healthcheck Watchdog

## Goal
自動偵測 Discord gateway 靜默斷線的 bot 並重啟，消除殭屍進程問題。

## Background
2026-04-13 實際發生多次 CICX/GIMINIX 進程活著但 Discord gateway 斷線的情況。Serenity（Rust Discord library）的自動重連在某些情境下靜默失敗，進程變殭屍 — 活著但不回應 Discord 訊息。目前只能人工發現並手動重啟。

### 觸發事件
- CICX 斷線 3.5 小時無人發現（log 停在 23:04，直到 02:53 手動重啟）
- GIMINIX 同時斷線
- `run-hidden.vbs` 反覆被其他 session 的 git 操作刪除，導致 bot 全掛

## Scope

### In Scope
- 外部 PowerShell health check script（不碰 Rust 源碼）
- Per-bot 粒度偵測與重啟
- `run-hidden.vbs` 自癒（不在就重建）
- Task Scheduler 每 2 分鐘執行
- Health check log（`logs/healthcheck.log`）

### Out of Scope
- Rust 源碼改動（會被其他 session revert）
- Discord webhook 通知（未來可加）
- 根因修復（Serenity 重連失敗的底層原因）

## Success Criteria
1. 殭屍 bot 在 12 分鐘內自動恢復（2 分鐘檢查間隔 + 10 分鐘閾值）
2. 正常運作的 bot 不受影響（零誤觸發）
3. `run-hidden.vbs` 被刪後自動重建
4. 重啟事件記錄在 `logs/healthcheck.log`
5. 不碰任何現有 Rust/JS/config 檔案
