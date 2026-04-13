# Design: Copilot Backend Support

## Technical Approach

Follow the exact pattern established by the Gemini integration:

```
Dockerfile.gemini  → Dockerfile.copilot   (same structure, different npm package)
build.yml matrix   → add copilot variant  (one line)
README.md          → add copilot entries  (backend table + Helm + config)
```

No Rust code changes. No config schema changes. Pure docs + Docker + CI.

## Architecture

```
Discord user
       │
       ▼
  OpenAB (Rust binary, same for all backends)
       │ stdio JSON-RPC (ACP protocol)
       ▼
  copilot --acp (Copilot CLI native ACP server)
       │ Copilot SDK internal
       ▼
  GitHub Copilot service (cloud)
```

## Key Decisions

### npm Package: `@github/copilot@1`
- NOT `@githubnext/github-copilot-cli` (deprecated v0.1.x — the old explain/suggest CLI)
- `@github/copilot` is the new Copilot CLI v1.x with `--acp` support
- `@1` pins to major version for reproducible builds (matches upstream pattern)
- Verification: `npm view @github/copilot version` → 1.0.24

### Auth: `gh auth login`
- Copilot CLI uses GitHub OAuth via `gh` CLI
- Requires Copilot subscription (Pro/Enterprise)
- In Kubernetes: mount `~/.config/gh/` via PVC (same pattern as other backends)

### CI Matrix Entry
- `.github/workflows/build.yml` has a variant matrix
- Add: `{ suffix: "-copilot", dockerfile: "Dockerfile.copilot", artifact: "copilot" }`
- This produces `ghcr.io/openabdev/openab-copilot:latest`
- ⚠️ Workflow files need `workflow` scope to push — may need maintainer to add

## Dependencies

- `@github/copilot` npm package (v1.x)
- `gh` CLI (for auth)
- Node.js 22 runtime (same as Gemini/Claude/Codex images)
- GitHub Copilot subscription (Pro or Enterprise)

## Security Considerations

- No secrets in Dockerfile or README
- Auth tokens stored in `~/.config/gh/` — not baked into image
- Copilot CLI runs as non-root user (`node:node`)
- Same HEALTHCHECK pattern as existing Dockerfiles

## Risks

1. `@github/copilot` npm package may change install behavior in v2 — pinned to `@1`
2. Workflow file push requires `workflow` scope — may need maintainer assistance
3. Copilot CLI cold-start time unknown in Docker — may need timeout tuning
