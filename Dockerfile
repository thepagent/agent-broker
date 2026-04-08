# --- Build stage ---
FROM rust:1-bookworm AS builder
WORKDIR /build

# Pre-download deps to cache them
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo 'fn main() {}' > src/main.rs && cargo build --release && rm -rf src

# Copy source
COPY src/ src/
COPY src/acp/ src/acp/
RUN touch src/main.rs && cargo build --release

# --- Runtime stage ---
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates curl gnupg && rm -rf /var/lib/apt/lists/*

# Install Node.js 20
RUN apt-get update && apt-get install -y ca-certificates curl gnupg && \
    mkdir -p /etc/apt/keyrings && \
    curl -fsSL https://deb.nodesource.com/gpgkey/nodesource-repo.gpg.key | gpg --dearmor -o /etc/apt/keyrings/nodesource.gpg && \
    echo "deb [signed-by=/etc/apt/keyrings/nodesource.gpg] https://deb.nodesource.com/node_20.x nodistro main" > /etc/apt/sources.list.d/nodesource.list && \
    apt-get update && apt-get install -y nodejs && rm -rf /var/lib/apt/lists/*

# Install claude-agent-acp globally
RUN npm install -g @agentclientprotocol/claude-agent-acp

RUN useradd -m -s /bin/bash -u 1000 agent
RUN mkdir -p /home/agent/.local/share/kiro-cli /home/agent/.kiro && \
    chown -R agent:agent /home/agent
ENV HOME=/home/agent
WORKDIR /home/agent

COPY --from=builder --chown=agent:agent /build/target/release/openab /usr/local/bin/openab

USER agent
HEALTHCHECK --interval=30s --timeout=5s --retries=3 \
  CMD pgrep -x openab || exit 1
ENTRYPOINT ["openab"]
CMD ["/etc/openab/config.toml"]
