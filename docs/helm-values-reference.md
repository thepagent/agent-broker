# Helm Values Reference

Complete reference for `charts/openab/values.yaml`.

## Top-level

| Key | Default | Description |
|-----|---------|-------------|
| `nameOverride` | `""` | Override the chart name used in resource names (truncated to 63 chars) |
| `fullnameOverride` | `""` | Override the full resource name prefix. Useful for multi-instance deployments (e.g., two agents on the same cluster) |
| `image.repository` | `ghcr.io/openabdev/openab` | Default image repository (used when no per-agent image is set) |
| `image.tag` | `""` | Image tag — defaults to `Chart.AppVersion` when empty |
| `image.pullPolicy` | `IfNotPresent` | Kubernetes image pull policy |
| `podSecurityContext` | see values.yaml | Pod-level security context (non-root user 1000) |
| `containerSecurityContext` | see values.yaml | Container-level security context (no privilege escalation) |

## Per-agent (`agents.<name>`)

Each key under `agents` defines one agent deployment. The default agent is `kiro`.

| Key | Default | Description |
|-----|---------|-------------|
| `enabled` | `true` | Set to `false` to skip creating resources for this agent |
| `image` | `""` | Per-agent image override (e.g., `ghcr.io/openabdev/openab-claude:latest`). Falls back to top-level `image` when empty |
| `command` | `kiro-cli` | Entrypoint command for the agent container |
| `args` | `["acp", "--trust-all-tools"]` | Arguments passed to `command` |
| `workingDir` | `/home/agent` | Working directory inside the container |
| `env` | `{}` | Extra environment variables as key/value pairs |
| `envFrom` | `[]` | Inject env vars from Secrets or ConfigMaps — preferred for credentials like `GH_TOKEN` |
| `agentsMd` | `""` | Agent identity/instructions file content. Use `--set-file` for large files (see below) |
| `resources` | `{}` | Kubernetes resource requests/limits |
| `nodeSelector` | `{}` | Node selector labels |
| `tolerations` | `[]` | Pod tolerations |
| `affinity` | `{}` | Pod affinity rules |

### `agents.<name>.discord`

| Key | Default | Description |
|-----|---------|-------------|
| `botToken` | `""` | Discord bot token |
| `allowedChannels` | `["YOUR_CHANNEL_ID"]` | ⚠️ Use `--set-string` to avoid float64 precision loss on large IDs |
| `allowedUsers` | `[]` | Restrict to specific user IDs. Empty = allow all users. ⚠️ Use `--set-string` |
| `allowBotMessages` | `"off"` | `"off"` \| `"mentions"` \| `"all"` — controls whether bot messages trigger the agent |
| `trustedBotIds` | `[]` | When `allowBotMessages` is not `"off"`, restrict to these bot IDs. Empty = any bot |

### `agents.<name>.pool`

| Key | Default | Description |
|-----|---------|-------------|
| `maxSessions` | `10` | Maximum concurrent ACP sessions |
| `sessionTtlHours` | `24` | Idle session TTL in hours |

### `agents.<name>.reactions`

| Key | Default | Description |
|-----|---------|-------------|
| `enabled` | `true` | Show emoji reactions while the agent is processing |
| `removeAfterReply` | `false` | Remove the reaction once the agent replies |

### `agents.<name>.stt`

Speech-to-text for Discord voice messages.

| Key | Default | Description |
|-----|---------|-------------|
| `enabled` | `false` | Enable STT transcription |
| `apiKey` | `""` | API key for the STT provider |
| `model` | `whisper-large-v3-turbo` | STT model name |
| `baseUrl` | `https://api.groq.com/openai/v1` | STT API base URL |

### `agents.<name>.persistence`

| Key | Default | Description |
|-----|---------|-------------|
| `enabled` | `true` | Mount a PersistentVolumeClaim for agent state |
| `storageClass` | `""` | Storage class name. Empty = cluster default |
| `size` | `1Gi` | PVC size |
| `existingClaim` | `""` | Use a pre-existing PVC instead of creating one |

---

## Common patterns

### Passing credentials via Secret (recommended)

Instead of putting tokens in Helm values, create a Secret and reference it with `envFrom`:

```bash
kubectl create secret generic gh-token --from-literal=GH_TOKEN=ghp_xxx
```

```yaml
agents:
  kiro:
    envFrom:
      - secretRef:
          name: gh-token
```

### Loading a large `agentsMd` file

```bash
helm install openab openab/openab \
  --set-file 'agents.kiro.agentsMd=./AGENTS.md'
```

### Multi-instance deployment (two agents on one cluster)

Use `fullnameOverride` to avoid resource name collisions:

```bash
helm install openab-kiro openab/openab --set fullnameOverride=openab-kiro ...
helm install openab-claude openab/openab --set fullnameOverride=openab-claude ...
```

### Channel and user IDs — always use `--set-string`

Discord IDs are large integers that Helm converts to float64, corrupting the value. Always use `--set-string`:

```bash
--set-string 'agents.kiro.discord.allowedChannels[0]=1234567890123456789'
--set-string 'agents.kiro.discord.allowedUsers[0]=9876543210987654321'
```
