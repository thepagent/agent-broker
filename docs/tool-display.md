# Tool Display Configuration

Control how tool calls are rendered in chat messages during agent responses.

## Configuration

```toml
[reactions]
tool_display = "compact"   # full | compact | none
```

### Helm

```yaml
agents:
  kiro:
    reactions:
      toolDisplay: "compact"   # full | compact | none
```

## Modes

### `compact` (default)

Shows a single-line count summary. No tool names, commands, or arguments are displayed.

```
✅ 3 · 🔧 1 tool(s)

Agent response text here...
```

Best for: everyday use, public channels, mobile.

### `full`

Shows each tool call with its complete title. When more than 3 tools finish, they collapse into a count summary automatically.

```
✅ `curl -s "https://ghcr.io/v2/openabdev/charts/openab/tags/list"`
✅ `grep -r "pattern" src/`
🔧 `npm install`...

Agent response text here...
```

Best for: debugging, understanding what the agent is doing step by step.

### `none`

Hides tool lines entirely. Only the final agent response is shown. Reaction emojis (🔧→✅) still work, so you can tell the agent is busy.

```
Agent response text here...
```

Best for: clean output when you only care about the final answer.

## Icons

| Icon | Meaning |
|------|---------|
| 🔧 | Tool is running |
| ✅ | Tool completed successfully |
| ❌ | Tool failed |

## Notes

- **Default changed**: `compact` is the new default. Previously, tool calls were always shown in full. If you want the old behavior, set `tool_display = "full"`.
- **Reaction emojis are independent**: The emoji reactions on messages (👀→🤔→🔧→🆗) work regardless of `tool_display` setting.
- **Streaming behavior**: In `compact` mode, the count updates in real-time as tools start and finish. In `full` mode, individual tool lines appear and update during streaming.
