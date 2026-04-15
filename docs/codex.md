# Codex

Codex uses the [@zed-industries/codex-acp](https://github.com/zed-industries/codex-acp) adapter for ACP support.

## Docker Image

```bash
docker build -f Dockerfile.codex -t openab-codex:latest .
```

The image installs `@zed-industries/codex-acp` and `@openai/codex` globally via npm.

## Helm Install

```bash
helm install openab openab/openab \
  --set agents.kiro.enabled=false \
  --set agents.codex.discord.botToken="$DISCORD_BOT_TOKEN" \
  --set-string 'agents.codex.discord.allowedChannels[0]=YOUR_CHANNEL_ID' \
  --set agents.codex.image=ghcr.io/openabdev/openab-codex:latest \
  --set agents.codex.command=codex-acp \
  --set agents.codex.workingDir=/home/node
```

> Set `agents.kiro.enabled=false` to disable the default Kiro agent.

## Recommended Helm configuration

For real deployments, it is often useful to configure Codex runtime behavior explicitly instead of relying on the default `args = []`.

```yaml
agents:
  codex:
    image: ghcr.io/openabdev/openab-codex:latest
    command: codex-acp
    workingDir: /home/node
    args:
      - -c
      - approval_policy="never"
      - -c
      - sandbox_mode="workspace-write"
      - -c
      - sandbox_workspace_write.network_access=true
    agentsMd: |
      Always reply in Traditional Chinese.
      Always provide a clear completion message when a task finishes.
```

- `agents.codex.args` is passed through to Codex and becomes the `args` array in the generated `config.toml`.
- `agents.codex.agentsMd` is mounted into the container as `AGENTS.md`, which is the supported way to persist project- or agent-specific instructions across pod restarts.

If your agent needs to run networked shell commands such as `gh issue view`, `git fetch`, or API calls, make sure the selected Codex sandbox/approval settings actually allow that in your runtime.

The safest working configuration depends on your container runtime. `workspace-write` is usually a good starting point, but you should verify that your environment supports it correctly before assuming Codex itself is misconfigured.

For the underlying Codex settings, see the official documentation:

- [Codex config.toml reference](https://developers.openai.com/codex/config-reference#configtoml)
- [Codex sandbox and approvals](https://developers.openai.com/codex/agent-approvals-security#sandbox-and-approvals)

## Manual config.toml

A minimal configuration looks like this:

```toml
[agent]
command = "codex-acp"
args = []
working_dir = "/home/node"
```

A more explicit configuration for networked shell usage looks like this:

```toml
[agent]
command = "codex-acp"
args = [
  "-c", "approval_policy=\"never\"",
  "-c", "sandbox_mode=\"workspace-write\"",
  "-c", "sandbox_workspace_write.network_access=true",
]
working_dir = "/home/node"
```

## Authentication

Authenticate Codex itself with:

```bash
kubectl exec -it deployment/openab-codex -- codex login --device-auth
```
