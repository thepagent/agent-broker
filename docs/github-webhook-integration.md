# GitHub Webhook to Discord — Agent Trigger Pattern

> **Note:** This documents a v1 workaround using GitHub Actions + Discord webhooks.
> The target architecture (v2+) is the [Custom Gateway](adr/custom-gateway.md) with a
> native GitHub adapter, which provides direct webhook reception, HMAC validation,
> and richer event context. See [ADR: Custom Gateway — Section 5](adr/custom-gateway.md#5-what-this-enables-beyond-chat) for the GitHub integration example.

## Overview

OpenAB only listens to Discord events. It does not accept external webhooks directly. To trigger agents from GitHub events (PR, Issue, etc.), we route through Discord as the single entry point.

## Architecture

```
GitHub (PR/Issue event)
  → GitHub Actions workflow
    → Discord Webhook (formatted message to channel)
      → OpenAB detects message
        → Routes to target agent
          → Agent performs action (review, comment, notify)
```

## Setup

### 1. Discord Webhook

Create a webhook in your Discord server for the target channel/topic:
- Server Settings → Integrations → Webhooks → New Webhook
- Copy the webhook URL

### 2. GitHub Secret

Add the webhook URL as a repository secret:
- Repo → Settings → Secrets and variables → Actions
- Name: `DISCORD_WEBHOOK_URL`
- Value: the webhook URL from step 1

### 3. GitHub Actions Workflow

Add `.github/workflows/notify-discord.yml` to your repo:

```yaml
name: Notify Discord

on:
  pull_request:
    types: [opened, reopened]
  issues:
    types: [opened, reopened]

jobs:
  notify:
    runs-on: ubuntu-latest
    steps:
      - name: Send to Discord
        env:
          DISCORD_WEBHOOK_URL: ${{ secrets.DISCORD_WEBHOOK_URL }}
          EVENT_NAME: ${{ github.event_name }}
          EVENT_ACTION: ${{ github.event.action }}
          PR_TITLE: ${{ github.event.pull_request.title }}
          PR_URL: ${{ github.event.pull_request.html_url }}
          PR_AUTHOR: ${{ github.event.pull_request.user.login }}
          PR_NUMBER: ${{ github.event.pull_request.number }}
          ISSUE_TITLE: ${{ github.event.issue.title }}
          ISSUE_URL: ${{ github.event.issue.html_url }}
          ISSUE_AUTHOR: ${{ github.event.issue.user.login }}
          ISSUE_NUMBER: ${{ github.event.issue.number }}
          REPO: ${{ github.repository }}
        run: |
          [ -z "$DISCORD_WEBHOOK_URL" ] && { echo "::warning::DISCORD_WEBHOOK_URL not set"; exit 0; }

          if [ "$EVENT_NAME" = "pull_request" ]; then
            TITLE="$PR_TITLE"
            URL="$PR_URL"
            AUTHOR="$PR_AUTHOR"
            NUM="$PR_NUMBER"
            TYPE="pr_${EVENT_ACTION}"
            LABEL="PR #${NUM}"
          else
            TITLE="$ISSUE_TITLE"
            URL="$ISSUE_URL"
            AUTHOR="$ISSUE_AUTHOR"
            NUM="$ISSUE_NUMBER"
            TYPE="issue_${EVENT_ACTION}"
            LABEL="Issue #${NUM}"
          fi

          PAYLOAD=$(jq -n \
            --arg content "[GH-EVENT] repo:${REPO} action:${TYPE} ${LABEL}
**${TITLE}**
by ${AUTHOR}
${URL}" \
            '{"content": $content}')
          curl --fail-with-body -s -H "Content-Type: application/json" -d "$PAYLOAD" "$DISCORD_WEBHOOK_URL"
```

## Message Format Convention

Messages use a structured prefix so OpenAB can identify GitHub events:

```
[GH-EVENT] repo:{owner/repo} action:{event_type} {PR/Issue} #{number}
**{title}**
by {author}
{url}
```

Supported `event_type` values: `pr_opened`, `pr_reopened`, `issue_opened`, `issue_reopened`

Example:
```
[GH-EVENT] repo:openabdev/openab action:pr_opened PR #42
**Add webhook integration docs**
by obrutjack
https://github.com/openabdev/openab/pull/42
```

## Open Questions

- **Bot message handling**: Does OpenAB currently ignore messages from bots/webhooks? If so, webhook sources need to be allowlisted. Note: OpenAB uses `allowed_channels` and `allowed_users` in `config.toml` for filtering — webhook messages come from a bot user, so the bot's user ID may need to be added to `allowed_users`, or the filtering logic would need a `[GH-EVENT]` prefix check.
- **Routing**: How does OpenAB determine which agent handles a `[GH-EVENT]` message?
- **Loop prevention**: If an agent replies in the same channel, could it re-trigger events? Recommend using a dedicated channel and filtering by `[GH-EVENT]` prefix only.

## Best Practices

- Use a dedicated channel or thread for webhook events
- Stick to the `[GH-EVENT]` prefix convention for all GitHub-sourced messages
- Validate webhook sources on the Discord side (restrict channel permissions)
- Avoid agents posting back to the same webhook channel to prevent loops
- Start minimal (PR + Issue notifications), expand as needed

## Future Considerations

- Extend pattern to other sources: Jira, Slack, PagerDuty, etc.
- Agent-to-agent invocation during review workflows
- Event filtering and deduplication at the OpenAB level
- Richer payloads using Discord embeds instead of plain text
