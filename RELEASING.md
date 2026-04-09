# Releasing

## Version Scheme

Chart versions follow SemVer with beta pre-releases:

- **Beta**: `0.2.1-beta.12345` — auto-generated on every push to main
- **Stable**: `0.2.1` — manually triggered, visible to `helm install`

Users running `helm install` only see stable versions. Beta versions require `--devel` or explicit `--version`.

## Development Flow

```
  PR merged to main
        │
        ▼
  ┌─────────────┐     ┌──────────────────┐     ┌─────────────────────┐
  │ CI: Build   │────>│ CI: Bump PR      │────>│ Merge bump PR       │
  │ 3 images    │     │ 0.2.1-beta.12345 │     │ → publishes beta    │
  └─────────────┘     └──────────────────┘     └─────────────────────┘
                                                        │
        ┌───────────────────────────────────────────────┘
        ▼
  helm install ... --version 0.2.1-beta.12345   (explicit only)
  helm install ...                               (still gets 0.2.0 stable)
```

## Stable Release

```
  Actions → Build & Release → Run workflow
  [bump: patch] [✅ Stable release]
        │
        ▼
  ┌─────────────┐     ┌──────────────────┐     ┌─────────────────────┐
  │ CI: Build   │────>│ CI: Bump PR      │────>│ Merge bump PR       │
  │ 3 images    │     │ 0.2.1            │     │ → publishes stable  │
  └─────────────┘     └──────────────────┘     └─────────────────────┘
                                                        │
        ┌───────────────────────────────────────────────┘
        ▼
  helm install ...                               (gets 0.2.1 🎉)
```

## Release Flow (Tag-Driven)

```
  1. PR merged to main
        │
        ▼
  ┌──────────────────────────────────────────────────────┐
  │ tagpr 自動累積 commits，開 Release PR                │
  │ (chore: release vX.Y.Z)                              │
  └──────────────────────────────────────────────────────┘
        │
        ▼
  2. Maintainer review Release PR，確認版本號與 changelog
     → merge Release PR
        │
        ▼
  ┌──────────────────────────────────────────────────────┐
  │ tagpr 自動打 tag (e.g. v0.7.0-beta.1)               │
  │ → build.yml 觸發 (tag-driven)                       │
  └──────────────────────────────────────────────────────┘
        │
        ▼
  3. Beta 測試通過後，在同一個 commit 打 stable tag：
     git tag v0.7.0
     git push origin v0.7.0
        │
        ▼
  ┌──────────────────────────────────────────────────────┐
  │ build.yml promote-stable 觸發                       │
  └──────────────────────────────────────────────────────┘
```

> ⚠️ **重要約束**：stable tag 必須打在跟 beta 完全相同的 commit 上，
> 否則 promote-stable 會驗證失敗。

## Image Tags

Each build produces three multi-arch images tagged with the git short SHA:

```
ghcr.io/openabdev/openab:<sha>        # kiro-cli
ghcr.io/openabdev/openab-codex:<sha>   # codex
ghcr.io/openabdev/openab-claude:<sha>  # claude
```

The `latest` tag always points to the most recent build.
