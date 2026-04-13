# Tasks: Copilot Backend Support

## Execution Model

```
┌─────────────────────────────────────────────────┐
│              VERIFY LOOP (Phase 1)              │
│  T1 → T2 → T3                                  │
│  任何一個 FAIL → 停止，重新調查，修正假設再重跑  │
│  全部 PASS → 進入 Phase 2                       │
└─────────────────────────────────────────────────┘
                    ↓
┌─────────────────────────────────────────────────┐
│          IMPLEMENT + VERIFY LOOP (Phase 2)      │
│  T4, T5, T6 (並行實作)                          │
│       ↓                                         │
│  T7 (/self-review 6-phase)                      │
│       ↓                                         │
│  有問題？→ 回到 T4/T5/T6 修 → 重跑 T7          │
│  零問題？→ 進入 Phase 3                         │
└─────────────────────────────────────────────────┘
                    ↓
┌─────────────────────────────────────────────────┐
│           SECURITY + SUBMIT LOOP (Phase 3)      │
│  T8 (security scan)                             │
│       ↓                                         │
│  有洩漏？→ 回到 T4/T5/T6 修 → 重跑 T7 + T8    │
│  零洩漏？→ T9 (建 PR)                           │
│       ↓                                         │
│  T10 (等 Copilot review)                        │
│       ↓                                         │
│  有 comment？→ 修 → 重跑 T7 + T8 → 更新 PR     │
│  零 comment？→ ✅ 完成                          │
└─────────────────────────────────────────────────┘
```

## Phase 1: Verification Loop

每個 task FAIL 就停止。不猜、不假設、不跳過。

- [x] **T1: 驗證 npm 套件安裝** ✅ @github/copilot v1.0.24, bin=copilot, --acp confirmed
  - 跑: `npm install -g @github/copilot@1` （或在 WSL 乾淨環境）
  - 驗證: `copilot --version` 回 1.x
  - 驗證: `copilot --help` 有 `--acp` flag
  - FAIL → 搜正確套件名，更新 design.md，重跑 T1

- [x] **T2: 驗證 ACP 完整流程** ✅ init+session+prompt all OK, 3 modes, 8 models, usage_update=false (safe fallback)
  - 跑: spawn `copilot --acp` → `initialize` → `session/new` → `session/prompt "reply OK"`
  - 驗證: 收到 `agent_message_chunk` + prompt response
  - 記錄: modes、models 數量、configOptions、usage_update 支援
  - FAIL → 調查 ACP 相容性問題，更新 design.md，重跑 T2

- [x] **T3: 驗證 headless auth** ✅ gh auth login --with-token works, token at ~/.config/gh/hosts.yml
  - 確認: `gh auth login --with-token` 可非互動認證
  - 記錄: auth 後 token 存放路徑
  - FAIL → 找替代 auth 方式，更新 design.md，重跑 T3

## Phase 2: Implementation + Verify Loop

實作後必須跑 T7。T7 有問題就回來改，改完重跑 T7。

- [x] **T4: 建 Dockerfile.copilot** ✅ diff with Gemini = 3 lines only (comment + npm pkg + comment)
  - 從 `Dockerfile.gemini` 複製
  - 改 npm 套件為 T1 驗證過的正確名稱
  - diff with Dockerfile.gemini 只有預期的差異
  - 無 hardcoded path、secrets、user-specific values

- [x] **T5: 更新 README.md** ✅ backend table + Helm + config added, Gemini example intact
  - 加 Copilot 到 backend table（格式完全一致）
  - 加 Helm install 範例（格式完全一致）
  - 加 manual config.toml 範例（插在正確位置）
  - 驗證: Gemini 範例的 `working_dir` + `env` 沒被破壞
  - 驗證: markdown 渲染正確

- [x] **T6: 加 copilot 到 CI build matrix** ✅ added to build.yml (⚠️ may need workflow scope to push)
  - 加 `{ suffix: "-copilot", dockerfile: "Dockerfile.copilot", artifact: "copilot" }`
  - 如果 `workflow` scope 不夠 → 在 PR comment 說明，請 maintainer 加
  - 格式完全 match 現有 entries

- [x] **T7: /self-review (完整 6 phase)** ✅ 0 issues, security=0, Gemini intact, workflow scope fallback handled
  - Phase 1: detect
  - Phase 2: static analysis
  - Phase 3: self-attack checklist — 每一項都回答
  - Phase 4: fix loop（有問題 → 回 T4/T5/T6 改 → 重跑 T7）
  - Phase 5: independent sub-agent review
  - Phase 6: final verdict
  - **T7 沒過 → 不進 Phase 3**

## Phase 3: Security + Submit Loop

- [x] **T8: Security scan** ✅ 0 secrets, 0 PII, 0 hardcoded paths
  - `git diff` 零: tokens、passwords、emails、hardcoded paths、PII
  - PR branch only — 沒有本地檔案洩漏
  - **T8 沒過 → 回 T4/T5/T6 改 → 重跑 T7 + T8**

- [x] **T9: 建 PR** ✅ PR #266 — 2 files, +50 lines, CI matrix note in body
  - Title: `feat: add GitHub Copilot CLI as supported ACP backend`
  - Body: Summary + Changes table + How it works + Configuration + Tested capabilities
  - 格式 match upstream #225
  - 包含 T2 的驗證結果

- [x] **T10: 回應 Copilot review** ✅ Fixed: pinned npm to 1.0.24, added "GitHub Copilot" to intro sentence
  - 等 auto-review
  - 有 comment → 修 code → 重跑 T7 + T8 → 更新 PR
  - 零 comment → ✅ 完成
