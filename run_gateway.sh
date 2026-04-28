#!/bin/bash
# Set listen port
export GATEWAY_LISTEN="0.0.0.0:9090"
export RUST_LOG=info

echo "Starting OpenAB Gateway on port 9090..."
# Use allexport to export all vars from the file
set -a
source secrets.env
set +a

./gateway/target/release/openab-gateway
