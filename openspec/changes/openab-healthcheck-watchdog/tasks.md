# Tasks: OpenAB Healthcheck Watchdog

## Phase 1: Core Script

- [x] **T1: 建立 `openab-healthcheck.ps1`** [P]
  - 實作 4-bot log stale 偵測（10 分鐘閾值）
  - 實作 per-bot kill + restart 邏輯
  - 實作 cooldown marker（3 分鐘防抖）
  - 實作 healthcheck.log 寫入
  - 驗證：手動跑 script，確認正常 bot 不被重啟

- [x] **T2: 實作 VBS self-heal** [P]
  - 在 healthcheck script 開頭檢查 `run-hidden.vbs`
  - 不存在就自動重建 + log `[HEALED]`
  - 驗證：刪除 VBS → 跑 script → 確認 VBS 被重建

## Phase 2: 自動化

- [x] **T3: 建立 `install-healthcheck.ps1`**
  - 依賴 T1
  - 註冊 Task Scheduler：每 2 分鐘跑一次
  - Principal: Administrator, LogonType Interactive, RunLevel Highest
  - 驗證：`Get-ScheduledTask OpenAB-Healthcheck` 顯示 Running

## Phase 3: 整合測試

- [x] **T4: 模擬斷線測試**
  - 依賴 T1, T3
  - 手動殺一個 bot 的 openab.exe（不殺 bat loop）→ bat 會重啟
  - 手動殺一個 bot 的 openab.exe + cmd.exe + node.exe（完全死）→ 等 2 分鐘看 healthcheck 是否偵測並重啟
  - 驗證：`healthcheck.log` 出現 `[RESTART]` 記錄 + bot 恢復連線

- [x] **T5: VBS 被刪測試**
  - 依賴 T2, T3
  - 刪除 `run-hidden.vbs` → 等 2 分鐘
  - 驗證：VBS 被重建 + `healthcheck.log` 出現 `[HEALED]`

- [x] **T6: 零誤觸發驗證**
  - 依賴 T4
  - 4 個 bot 全部正常跑 10 分鐘
  - 驗證：`healthcheck.log` 沒有任何 `[RESTART]` 記錄
