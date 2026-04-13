# Proposal: Add GitHub Copilot CLI as First-Class ACP Backend

## Goal

Enable OpenAB users to connect GitHub Copilot CLI to Discord via its native `--acp` mode, with the same level of support as the existing Gemini integration (Dockerfile, CI, docs, Helm example).

## Background

- Community demand: Pahud (maintainer) and multiple users in #general confirmed their companies use Copilot as primary AI coding tool
- Copilot CLI v1.0.24+ ships native `--acp` flag (`copilot --help` → `--acp: Start as Agent Client Protocol server`)
- OpenAB already supports Copilot via config (`command = "copilot"`, `args = ["--acp"]`) but it's undocumented and has no Dockerfile/CI
- Previous PR #264 was closed due to: wrong npm package name, broken Gemini README example, missing CI matrix entry

## Scope

### In
- `Dockerfile.copilot` — runtime image with Copilot CLI
- `.github/workflows/build.yml` — add copilot to CI build matrix
- `README.md` — backend table, Helm example, manual config example
- End-to-end verification that `copilot --acp` works (initialize → session/new → prompt → response)
- `/self-review` full 6-phase before PR

### Out
- Custom `copilot-agent-acp.js` bridge (that's a local enhancement, not for upstream)
- Copilot-specific slash commands (e.g. /agent, /skill-on — those depend on the custom bridge)
- Helm chart `values.yaml` changes (dynamic agent keys already work via the template)
- Auth persistence docs for Kubernetes (can be a follow-up)

## Success Criteria

1. `npm install -g @github/copilot@1` installs correctly in `node:22-bookworm-slim`
2. `copilot --acp` responds to ACP `initialize` + `session/new` + `session/prompt`
3. CI build matrix includes `copilot` variant and would produce `ghcr.io/openabdev/openab-copilot`
4. README backend table has Copilot row matching existing format
5. Gemini README example is NOT broken by the change
6. `/self-review` passes all 6 phases with zero issues
7. PR passes Copilot code review with zero actionable comments
