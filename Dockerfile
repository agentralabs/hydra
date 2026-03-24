# Multi-stage build: Rust compilation → minimal runtime
FROM rust:latest AS builder

WORKDIR /app
COPY . .

# Build release binaries
RUN cargo build --release -p hydra-kernel -p hydra-tui 2>&1

# Minimal runtime image
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Copy binaries
COPY --from=builder /app/target/release/hydra /usr/local/bin/hydra
COPY --from=builder /app/target/release/hydra_tui /usr/local/bin/hydra-tui

# Copy skills (default genome)
COPY --from=builder /app/skills /opt/hydra/skills

# Data volume
VOLUME /root/.hydra

# Default: run in daemon mode
EXPOSE 3141
ENV HYDRA_LLM_PROVIDER=ollama
CMD ["hydra", "--daemon"]
