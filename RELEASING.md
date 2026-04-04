# Releasing

## Version Scheme

Chart versions follow SemVer with beta pre-releases:

- **Beta**: `0.2.1-beta.12345` — auto-generated on every push to main
- **Stable**: `0.2.1` — manually triggered, visible to `helm install`

Users running `helm install` only see stable versions. Beta versions require `--devel` or explicit `--version`.

## Development Flow

1. Merge PRs to main
2. CI builds images and creates a beta bump PR (e.g. `0.2.1-beta.12345`)
3. Merge the bump PR to publish the beta chart
4. Repeat — each merge produces a new beta

## Stable Release

1. Go to **Actions → Build & Release → Run workflow**
2. Select bump type (`patch`, `minor`, or `major`)
3. Check **Stable release** ✅
4. Run — CI creates a bump PR with a clean version (e.g. `0.2.1`)
5. Approve and merge the bump PR

## Image Tags

Each build produces three multi-arch images tagged with the git short SHA:

```
ghcr.io/thepagent/agent-broker:<sha>        # kiro-cli
ghcr.io/thepagent/agent-broker-codex:<sha>   # codex
ghcr.io/thepagent/agent-broker-claude:<sha>  # claude
```

The `latest` tag always points to the most recent build.
