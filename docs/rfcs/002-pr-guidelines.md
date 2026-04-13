# RFC 002: Pull Request Contribution Guidelines

| Field | Value |
|-------|-------|
| **RFC** | 002 |
| **Title** | Pull Request Contribution Guidelines |
| **Author** | @chaodu-agent |
| **Status** | Draft |
| **Created** | 2026-04-13 |

---

## Summary

Establish a standard PR template requiring contributors to research prior art (at minimum OpenClaw and Hermes Agent) before proposing solutions, and to document the problem, approach, tradeoffs, and alternatives in every PR description.

## Motivation

OpenAB is growing and accepting external contributions. Without a clear PR standard, we see PRs that:

- Jump straight to implementation without explaining the problem
- Don't research how existing projects solve the same problem
- Don't justify why a particular approach was chosen over alternatives
- Make review harder because reviewers must do the research themselves

**Good example:** Issue #224 / PR #225 (voice message STT) included a thorough prior art investigation — comparing OpenClaw's `audio-transcription-runner.ts` preflight pipeline with Hermes Agent's `transcription_tools.py` local-first approach, producing a clear comparison table, and explaining why openab chose a simpler OpenAI-compatible endpoint design. This is the standard we want every PR to meet.

**What we want to avoid:** PRs that jump straight to implementation without documenting how existing projects solve the same problem — forcing reviewers to do the research themselves during review.

## Design

### Required PR Sections

Every PR must include these sections in its description:

| # | Section | Purpose |
|---|---------|---------|
| 1 | **What problem does this solve?** | Pain point or requirement in plain language. Link the related issue. |
| 2 | **At a Glance** | ASCII diagram showing the high-level flow, architecture, or where the change fits in the system. |
| 3 | **Prior Art & Industry Research** | How OpenClaw and Hermes Agent handle the same problem. Links to code/docs. |
| 4 | **Proposed Solution** | Technical approach, architecture decisions, key implementation details. |
| 5 | **Why This Approach** | Why this over the alternatives from research. Tradeoffs and limitations. |
| 6 | **Alternatives Considered** | Approaches evaluated but not chosen, and why. |
| 7 | **Validation** | How do you prove it works? Unit tests, integration tests, manual testing steps, screenshots, logs. |

### Mandatory Prior Art Research

Contributors must research at minimum these two projects:

| Project | Why it's mandatory |
|---|---|
| [OpenClaw](https://github.com/openclaw/openclaw) | Largest open-source AI agent gateway. Plugin architecture across 7+ messaging platforms. Mature patterns for media, security, session management. |
| [Hermes Agent](https://github.com/NousResearch/hermes-agent) | Nous Research's self-hosted agent. Gateway architecture across 17+ platforms. Strong prior art on messaging, tool integration, and service management. |

For each project, document:
- How they solve the same problem (with links to source code or docs)
- Key architectural decisions they made
- What we can learn from their approach

If neither project addresses the problem, state that explicitly with evidence.

### Research Flow

```
Contributor researches prior art
        │
        ▼
┌───────────────────────┐     ┌──────────────────────────────────┐
│ Finds better pattern  │────►│ Adopts it (we benefit)           │
└───────────┬───────────┘     └──────────────────────────────────┘
            │
            ▼
┌───────────────────────────┐ ┌──────────────────────────────────┐
│ Finds different pattern   │►│ Documents why we diverge         │
└───────────┬───────────────┘ │ (reviewers understand tradeoff)  │
            │                 └──────────────────────────────────┘
            ▼
┌───────────────────────────┐ ┌──────────────────────────────────┐
│ Finds nothing relevant    │►│ States so explicitly             │
└───────────────────────────┘ │ (saves reviewers from searching) │
                              └──────────────────────────────────┘
```

## Implementation

| Phase | Deliverable | Description |
|-------|-------------|-------------|
| **1** | `.github/pull_request_template.md` | Auto-populated PR form with all required sections |
| **2** | `CONTRIBUTING.md` | Contributor guide explaining the guidelines and linking to this RFC |
| **3** | Review process update | Reviewers check for prior art section completeness |

### PR Template

```markdown
## What problem does this solve?

<!-- Describe the pain point in plain language. Link the related issue. -->

Closes #

## At a Glance

<!-- ASCII diagram showing the high-level flow or where this change fits in the system. Example:

┌──────────────┐     ┌───────┐     ┌───────────┐
│ Discord User │────►│ openab│────►│ ACP Agent │
└──────────────┘     └───┬───┘     └───────────┘
                         │
                         ▼
                  ┌──────────────┐
                  │ your change  │
                  └──────────────┘
-->

```
(your diagram here)
```

## Prior Art & Industry Research

<!-- Research how at least OpenClaw and Hermes Agent handle this problem. -->

**OpenClaw:**
<!-- How does OpenClaw solve this? Link to relevant code/docs. -->

**Hermes Agent:**
<!-- How does Hermes Agent solve this? Link to relevant code/docs. -->

**Other references (optional):**

## Proposed Solution

<!-- Technical approach, architecture decisions, key implementation details. -->

## Why This Approach

<!-- Why this over the alternatives from your research? Tradeoffs? Limitations? -->

## Alternatives Considered

<!-- Approaches evaluated but not chosen, and why. -->

## Validation

<!-- How do you prove this works? Show, don't just tell. -->

- [ ] `cargo check` passes
- [ ] `cargo test` passes (including new tests)
- [ ] Manual testing — describe the steps you took and what you observed
- [ ] Screenshots, logs, or terminal output demonstrating the feature working end-to-end
```

## Open Questions

1. Should we enforce the prior art section via CI (e.g., a bot that checks for the section headers)?
2. Should we maintain a living doc of "how OpenClaw/Hermes do X" to reduce per-PR research burden?
3. Are there other mandatory reference projects beyond OpenClaw and Hermes?

---

## References

- Issue #224 / PR #225 — exemplary prior art research (STT: OpenClaw vs Hermes Agent comparison)
- [OpenClaw Channel & Messaging Deep Dive](https://avasdream.com/blog/openclaw-channels-messaging-deep-dive)
- [Hermes Agent Messaging Gateway](https://hermes-agent.nousresearch.com/docs/user-guide/messaging/)
- RFC 001 — [Session Management](./001-session-management.md)
