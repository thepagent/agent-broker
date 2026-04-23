# ADR-001: Agent Cognitive Architecture Specification

- **Status**: Proposed
- **Date**: 2026-04-23
- **Author**: pahud.hsieh

## Context

OpenAB is a multi-bot, agent-agnostic, vendor-agnostic platform. It bridges multiple coding CLIs (Kiro, Claude Code, Codex, Gemini, Copilot, OpenCode, Cursor, etc.) into chat platforms like Discord and Slack, where multiple bots/agents coexist in the same chatroom.

When multiple agents share a chatroom, a critical challenge emerges: **how does each agent quickly establish its own cognitive system — identity, memory, and social relationships — so they can effectively communicate, coordinate, and collaborate?**

Today, each agent operates in isolation without a shared standard. This leads to:

1. No consistent persona — agents don't know "who they are" across sessions
2. No social awareness — agents don't know who else is in the room, what they're good at, or how to mention them
3. No persistent memory — knowledge is lost between sessions with no mechanism to accumulate and refine it over time

For OpenAB's multi-agent vision to work, we need a generic, platform-agnostic specification that any agent implementation can follow to bootstrap these cognitive capabilities — regardless of the underlying LLM or framework.

## Decision

We adopt a three-pillar cognitive architecture for agents:

1. **Self-Identity System** — defines who the agent is
2. **Social Awareness System** — defines who else exists and how to interact
3. **Knowledge System** — defines how the agent remembers, recalls, and refines knowledge

---

## 1. Self-Identity System

Every agent MUST maintain a self-identity definition that answers: **"Who am I?"**

### Required Identity Fields

```yaml
identity:
  name: ""            # Agent's name (how it refers to itself)
  uid: ""             # Unique identifier (platform-specific, e.g. Discord UID)
  persona: ""         # One-line self-description
  personality: []     # List of personality traits
  tone: ""            # Communication style (e.g. humorous, formal, blunt)
  language: []        # Preferred languages, in order
  origin: ""          # Backstory or origin (optional)
```

### Behavioral Guidelines

- **Consistency**: Respond in a manner consistent with defined personality across all interactions.
- **Self-reference**: Refer to itself by `name`, never by underlying model or framework name.
- **Boundaries**: Identity definition should include what the agent will NOT do.

---

## 2. Social Awareness System

In a multi-agent environment, each agent MUST be aware of its social context.

### Peer Registry

```yaml
peers:
  - name: "Agent B"
    uid: "9876543210"
    role: "Research assistant"
    mention_syntax: "<@9876543210>"   # Platform-specific mention format
    status: "active"                   # active | inactive | muted
    notes: "Specializes in summarization"
```

### Social Rules

- **Discovery**: Agents query the peer registry to find who can help with a given task.
- **Mention Protocol**: Use the platform's `mention_syntax` when referencing another agent.
- **Delegation**: An agent MAY delegate tasks to peers with user consent.
- **Mute/Ignore**: Agents MUST respect mute directives.

### Storage

The peer registry can be stored as a local file, shared database, or API endpoint.

---

## 3. Knowledge System

The knowledge system is the agent's long-term memory, designed around three layers:

```
┌─────────────────────────────────────┐
│           SQLite Index              │  ← Fast lookup, search, metadata
│  (paths, tags, timestamps, links)   │
├─────────────────────────────────────┤
│         Knowledge Files (.md)       │  ← Refined, structured knowledge
│  (topics, facts, how-tos, people)   │
├─────────────────────────────────────┤
│      Daily Logs (YYYY-MM-DD.md)     │  ← Raw observations, conversations
│  (unprocessed, timestamped entries) │
└─────────────────────────────────────┘
```

### Layer 1: Daily Logs (Raw Input)

**Format**: `logs/YYYY-MM-DD.md`

```markdown
# 2026-04-23

## 14:32 - Conversation with pahud
- Discussed agent cognitive architecture
- Key idea: knowledge should be layered (raw → refined → indexed)
```

Rules:
- Append-only during the day. Never edit past entries.
- Each entry has a timestamp and brief context.

### Layer 2: Knowledge Files (Refined)

**Format**: `knowledge/<topic>.md`

```markdown
# SQLite for Agent Memory

## Summary
SQLite with FTS5 is effective for indexing markdown-based knowledge files.

## Key Points
- Use FTS5 for full-text search across knowledge base
- Store file paths, tags, and last-updated timestamps

## Changelog
- 2026-04-23: Created from daily log observation
```

Rules:
- Each file covers ONE topic or entity.
- Files are living documents — refined over time.
- Include a `Changelog` section to track evolution.

### Layer 3: SQLite Index (Fast Lookup)

```sql
CREATE TABLE knowledge_files (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    path TEXT NOT NULL UNIQUE,
    title TEXT NOT NULL,
    tags TEXT,
    summary TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    last_reflected_from TEXT
);

CREATE VIRTUAL TABLE knowledge_fts USING fts5(
    title, tags, summary, content,
    content='knowledge_files',
    content_rowid='id'
);

CREATE TABLE daily_logs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    date TEXT NOT NULL UNIQUE,
    path TEXT NOT NULL,
    processed BOOLEAN DEFAULT 0
);
```

---

## 4. Knowledge Tools

Three commands power the knowledge lifecycle:

### `/recall` — Retrieve Knowledge

1. Parse query into search terms.
2. Query `knowledge_fts` for matching files.
3. Read top-N matching `.md` files.
4. Synthesize and return relevant information.

### `/remember` — Store New Knowledge

1. Append raw info to today's daily log.
2. Determine if this fits an existing knowledge file or needs a new one.
3. Update or create the knowledge `.md` file.
4. Update the SQLite index.

### `/reflect` — Extract & Refine Knowledge

1. Find all daily logs where `processed = 0`.
2. For each unprocessed log:
   - Read raw entries and identify discrete knowledge points.
   - Update existing or create new knowledge files.
   - Update SQLite index.
   - Mark log as `processed = 1`.
3. Return a summary of changes.

---

## 5. Lifecycle

```
User interaction / Events
        │
        ▼
  ┌──────────┐    /remember
  │ Daily Log │ ◄──────────── Immediate capture
  │ (raw)     │
  └────┬─────┘
       │
       │  /reflect (batch or scheduled)
       ▼
  ┌──────────────┐
  │ Knowledge    │ ◄── Extract, merge, refine
  │ Files (.md)  │
  └──────┬───────┘
         │
         │  Index on change
         ▼
  ┌──────────────┐
  │ SQLite Index │ ◄── Fast search & retrieval
  │ (FTS5)       │
  └──────┬───────┘
         │
         │  /recall
         ▼
    Agent Response
```

---

## 6. Implementation Notes

1. **File-first**: Knowledge lives in `.md` files. SQLite is an index, not the source of truth. If the DB is lost, rebuild from files.
2. **Idempotent reflect**: Running `/reflect` multiple times on the same log produces the same result.
3. **Conflict resolution**: For shared knowledge bases, use last-write-wins with changelog entries.
4. **Embedding-ready**: Add a `knowledge_embeddings` table for semantic search when needed.
5. **Platform-agnostic**: No assumption on LLM, framework, or platform.
6. **Privacy**: Implement access controls for knowledge files containing personal information.

## 7. File Structure Reference

```
~/
├── identity.yaml              # Self-identity definition (§1)
├── peers.yaml                 # Social peer registry (§2)
├── knowledge/                 # Refined knowledge files (§3.2)
│   ├── sqlite-memory.md
│   └── agent-architecture.md
├── logs/                      # Daily raw logs (§3.1)
│   ├── 2026-04-22.md
│   └── 2026-04-23.md
└── memory.db                  # SQLite index (§3.3)
```

## Consequences

- All OpenAB-compatible agents gain a standard way to define identity, discover peers, and manage knowledge.
- Agents can be swapped or upgraded without losing accumulated knowledge (file-first design).
- The spec is intentionally minimal — implementors can extend it (e.g. vector embeddings, shared knowledge) without breaking compatibility.
- Daily log + reflect pattern enables both real-time capture and batch knowledge refinement.
