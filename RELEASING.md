# Releasing

## Version Scheme

Versions follow SemVer (e.g. `0.7.0`)。tagpr 根據 PR label 自動決定版本號：

| Label | 效果 | 範例 |
|---|---|---|
| （無） | patch bump | `0.6.0 → 0.6.1` |
| `tagpr:minor` | minor bump | `0.6.0 → 0.7.0` |
| `tagpr:major` | major bump | `0.6.0 → 1.0.0` |

## Release Flow (Tag-Driven)

```
  ┌─────────────────────────────────────────────────────────────────┐
  │ 1. 貢獻者 PR merge to main                                       │
  │    → tagpr.yml 觸發                                              │
  │    → tagpr 累積 commits，自動開 Release PR                        │
  │      (更新 Cargo.toml + Chart.yaml version/appVersion            │
  │       + CHANGELOG.md)                                            │
  └─────────────────────────────────────────────────────────────────┘
        │
        ▼
  ┌─────────────────────────────────────────────────────────────────┐
  │ 2. Maintainer review Release PR                                  │
  │    確認版本號 / changelog 後 merge                                │
  │    → tagpr 自動打 tag (e.g. v0.7.0) + 建立 GitHub Release        │
  └─────────────────────────────────────────────────────────────────┘
        │
        ▼
  ┌─────────────────────────────────────────────────────────────────┐
  │ 3. build.yml 被 tag push 觸發                                    │
  │    → build-image:    4 variants × 2 platforms (amd64 + arm64)   │
  │    → merge-manifests: 建立 multi-arch manifest                   │
  │    → release-chart:  打包 helm chart → OCI registry              │
  └─────────────────────────────────────────────────────────────────┘
        │
        ▼
  ┌─────────────────────────────────────────────────────────────────┐
  │ 4. release.yml 偵測到 Chart.yaml 變更 push to main                │
  │    → chart-releaser 更新 GitHub Pages helm repo index            │
  │    → 附加 install instructions 到 chart release notes            │
  └─────────────────────────────────────────────────────────────────┘
```

## GitHub Releases

每次 release 會產生兩個 GitHub Release：

| Release | Tag 格式 | 內容 |
|---|---|---|
| tagpr | `v0.7.0` | CHANGELOG（自動從 commits 產生） |
| chart-releaser | `openab-0.7.0` | Version Info + Installation instructions |

## Workflow 對應表

| Workflow | 觸發條件 | 用途 |
|---|---|---|
| `tagpr.yml` | push to main | 自動開 Release PR、打 tag、建立 GitHub Release |
| `build.yml` | tag push `v*` | build image + push helm chart to OCI |
| `release.yml` | Chart.yaml 變更 push to main | chart-releaser 更新 GitHub Pages index + install instructions |

## Version 同步 (tagpr)

tagpr 在 Release PR 中自動更新以下檔案的版本：

| 檔案 | 欄位 | 更新方式 |
|---|---|---|
| `Cargo.toml` | `version` | tagpr 內建 (`versionFile`) |
| `charts/openab/Chart.yaml` | `version` | tagpr 內建 (`versionFile`) |
| `charts/openab/Chart.yaml` | `appVersion` | `postVersionCommand` |

三者統一為同一個 semver（e.g. `0.7.0`）。

## Image Variants

每次 build 產出 4 個 multi-arch image (linux/amd64 + linux/arm64)：

```
ghcr.io/openabdev/openab          # default (kiro-cli)
ghcr.io/openabdev/openab-codex    # codex
ghcr.io/openabdev/openab-claude   # claude
ghcr.io/openabdev/openab-gemini   # gemini
```

Image tags 依 release 類型不同：

| Tag | Stable (`v0.7.0`) | Pre-release (`v0.7.0-rc.1`) |
|---|---|---|
| `<sha>` | v | v |
| `0.7.0` / `0.7.0-rc.1` | v | v |
| `0.7` | v | x |
| `latest` | v | x |

## Installation

##### Helm Repository (GitHub Pages)

```bash
helm repo add openab https://openabdev.github.io/openab
helm repo update
helm install openab openab/openab --version 0.7.0
```

##### OCI Registry

```bash
helm install openab oci://ghcr.io/openabdev/charts/openab --version 0.7.0
```

## Pre-release

tagpr 不支援 pre-release tag。需要手動操作：

```bash
git tag v0.7.0-rc.1
git push origin v0.7.0-rc.1
```

build.yml 會觸發完整 build，但 **不會** 覆蓋 `latest` 和 `major.minor` tag。
Pre-release image 只有 `<sha>` 和 `0.7.0-rc.1` 兩個 tag。

安裝 pre-release chart：

```bash
helm install openab oci://ghcr.io/openabdev/charts/openab --version 0.7.0-rc.1
```

## 手動操作

| 時機 | 做什麼 |
|---|---|
| tagpr 開 Release PR 後 | Review 版本號 / CHANGELOG |
| 需要調整版本升級幅度 | 在 Release PR 加 `tagpr:minor` 或 `tagpr:major` label |
| 決定 release | Merge Release PR（之後全自動） |
| 需要 pre-release | 手動打 tag（e.g. `git tag v0.7.0-rc.1 && git push origin v0.7.0-rc.1`） |
| build 失敗或需重跑 | Actions → Build & Release → Run workflow（填入 tag） |
