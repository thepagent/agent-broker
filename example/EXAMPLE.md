# OpenAB Docker Compose Usage Guide

## Overview

This docker-compose configuration starts two AI assistant services:
- **kiro**: OpenAB's Kiro assistant
- **claude**: OpenAB's Claude assistant

## Directory Structure

```
example/
├── data/
│   ├── home/
│   │   ├── agent/          # Kiro's home directory
│   │   └── node/           # Claude's home directory
│   │       └── .claude/
│   │           └── CLAUDE.md  # Claude character config (Sasuke)
│   └── config/
│       ├── kiro/
│       │   └── config.toml    # Kiro configuration
│       └── claude/
│           └── config.toml    # Claude configuration
└── docker-compose.yml
```

## Start Services

```bash
docker-compose up -d
```

## Authentication Login
### kiro
```bash
docker-compose exec -it kiro kiro-cli login --use-device-flow
```
### Claude
```bash
docker-compose exec -it claude setup-token
```

## Stop Services

```bash
docker-compose down
```

## View Logs

```bash
# View all services
docker-compose logs -f

# View specific service
docker-compose logs -f kiro
docker-compose logs -f claude
```

## Restart Services

```bash
docker-compose restart
```

## Configuration Setup

### Discord Bot Settings

Before starting, you must modify the following configuration files:

**Kiro Configuration** (`./example/data/config/kiro/config.toml`):
```toml
[discord]
bot_token = "YOUR_KIRO_BOT_TOKEN"           # Replace with your Kiro Discord Bot Token
allowed_channels = ["YOUR_CHANNEL_ID"]      # Replace with allowed channel ID
```

**Claude Configuration** (`./example/data/config/claude/config.toml`):
```toml
[discord]
bot_token = "YOUR_CLAUDE_BOT_TOKEN"         # Replace with your Claude Discord Bot Token
allowed_channels = ["YOUR_CHANNEL_ID"]      # Replace with allowed channel ID
```

### Character Settings

The Claude service uses Uchiha Sasuke character settings, located at:
`./example/data/home/node/.claude/CLAUDE.md`

After modifying configurations, restart the corresponding service:
```bash
docker-compose restart kiro    # Restart Kiro
docker-compose restart claude  # Restart Claude
```

## Environment Variables

Both services are set with `RUST_LOG=debug` for detailed logging.

## Data Persistence

All data is mounted to the local `./example/data/` directory, so data persists after container restarts.
