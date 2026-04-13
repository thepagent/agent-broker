# Design: OpenAB Healthcheck Watchdog

## Technical Approach

外部 PowerShell script 定期檢查每個 bot 的 log 檔案最後修改時間，偵測殭屍進程並自動重啟。

## Architecture

```
┌──────────────────────────────────────────────────┐
│ Task Scheduler (every 2 min)                     │
│   → openab-healthcheck.ps1                       │
└──────────┬───────────────────────────────────────┘
           │
           ▼
┌──────────────────────────────────────────────────┐
│ Per-bot check loop:                              │
│                                                  │
│  ┌─────────┐  ┌─────────┐  ┌─────────┐  ┌─────┐│
│  │  CICX   │  │  GITX   │  │ GIMINIX │  │CODEX││
│  │logs/    │  │logs-cop/│  │logs-gem/ │  │logs-c│
│  └────┬────┘  └────┬────┘  └────┬────┘  └──┬──┘│
│       │            │            │           │    │
│       ▼            ▼            ▼           ▼    │
│  Check log LastWriteTime                         │
│       │                                          │
│  > 10 min stale?                                 │
│  ├─ No → skip                                   │
│  └─ Yes → Kill process tree                     │
│            → Ensure run-hidden.vbs exists        │
│            → wscript restart bat                 │
│            → Log to healthcheck.log              │
│            → Set cooldown marker                 │
└──────────────────────────────────────────────────┘
```

## Bot Configuration Map

```powershell
$bots = @(
  @{ Name="CICX";    Config="config.toml";         LogDir="logs";         Bat="run-openab-claude.bat"  },
  @{ Name="GITX";    Config="config-copilot.toml";  LogDir="logs-copilot"; Bat="run-openab-copilot.bat" },
  @{ Name="GIMINIX"; Config="config-gemini.toml";   LogDir="logs-gemini";  Bat="run-openab-gemini.bat"  },
  @{ Name="CODEX";   Config="config-codex.toml";    LogDir="logs-codex";   Bat="run-openab-codex.bat"   }
)
```

## Kill Logic

Per bot:
1. 用 `CommandLine -match <config>` 找到對應的 `openab.exe` PID
2. 殺 `openab.exe`
3. 用 `CommandLine -match` 找對應的 `cmd.exe` bat loop 和 `node.exe` ACP agent
4. 全部 Force Kill
5. 等 2 秒讓 file handles 釋放
6. wscript 啟動對應 bat

## VBS Self-Heal

```powershell
$vbsPath = "C:\Users\Administrator\openab\run-hidden.vbs"
if (-not (Test-Path $vbsPath)) {
    @'
Set WShell = CreateObject("WScript.Shell")
WShell.Run Chr(34) & WScript.Arguments(0) & Chr(34), 0, False
'@ | Set-Content $vbsPath -Encoding ASCII
    # Log the self-heal
}
```

## Cooldown Mechanism

防止剛重啟的 bot 被立刻再次判定為 stale：
- 重啟時建立 marker 檔案：`logs-*/healthcheck-restart.marker`
- 檔案內容：重啟時間戳
- 檢查時：如果 marker 存在且 < 3 分鐘前 → skip 該 bot
- Marker 自然過期（下次檢查時 > 3 分鐘就刪除）

## Log Format

```
logs/healthcheck.log (append)
2026-04-13 10:35:00 [RESTART] CICX stale=15min pid=35024
2026-04-13 10:35:02 [RESTART] GIMINIX stale=180min pid=32644
2026-04-13 10:35:05 [HEALED] run-hidden.vbs recreated
2026-04-13 10:37:00 [OK] all 4 bots healthy
```

正常時不寫 `[OK]` 行（完全靜默）。只在有 RESTART 或 HEALED 動作時才寫 log。

## Dependencies
- PowerShell 5.1+（Windows 內建）
- Task Scheduler（已有 OpenAB task 的先例）
- `run-hidden.vbs`（自癒）

## Security Considerations
- Script 以 Administrator 身份執行（Task Scheduler 設定）
- 不存取網路、不修改源碼
- 只殺特定 CommandLine pattern 的進程
