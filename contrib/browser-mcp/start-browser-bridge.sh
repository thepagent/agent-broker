#!/bin/bash
# Start browser-bridge for agent-browser MCP
# Usage: ./start-browser-bridge.sh

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
BRIDGE="$SCRIPT_DIR/browser-bridge.js"
LOG="$SCRIPT_DIR/browser-bridge.log"
NODE="/opt/homebrew/bin/node"
PORT=3002

# Check if already running
if lsof -i :$PORT -sTCP:LISTEN >/dev/null 2>&1; then
  echo "browser-bridge already running on :$PORT"
  exit 0
fi

nohup "$NODE" "$BRIDGE" >> "$LOG" 2>&1 &
echo "browser-bridge started (PID $!, port $PORT)"
