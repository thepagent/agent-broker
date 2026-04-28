#!/bin/bash
# Set listen port
export GATEWAY_LISTEN="0.0.0.0:9090"
export RUST_LOG=info

echo "Starting OpenAB Gateway on port 9090..."
# Use allexport to export all vars from the file
if [ -f secrets.env ]; then
  set -a
  source secrets.env
  set +a
else
  echo "Warning: secrets.env not found — skipping. Create it with LINE_CHANNEL_SECRET, LINE_CHANNEL_ACCESS_TOKEN, etc."
fi

./gateway/target/release/openab-gateway
