FROM rust:1.92-slim AS builder

WORKDIR /app

# Install build dependencies
RUN apt-get update && apt-get install -y libssl-dev pkg-config curl

# Copy sources
COPY . .

# Build in release mode without swagger
RUN cargo build --release --no-default-features

# Build migration tool as binary
WORKDIR /app/migration
RUN cargo build --release -p migration

# Create runtime image
FROM debian:stable-slim

WORKDIR /app

RUN apt-get update && apt-get install -y libssl3 ca-certificates curl file \
    && rm -rf /var/lib/apt/lists/*

# Node.js runtime for the luneth Playwright browser bridge (requires >= 18)
COPY --from=node:22-slim /usr/local/bin/node /usr/local/bin/node
COPY --from=node:22-slim /usr/local/lib/node_modules /usr/local/lib/node_modules
RUN ln -s /usr/local/lib/node_modules/npm/bin/npm-cli.js /usr/local/bin/npm \
 && ln -s /usr/local/lib/node_modules/npm/bin/npx-cli.js /usr/local/bin/npx

# Pre-install Playwright + Chromium with ALL required system dependencies.
# install-deps knows exactly which shared libs Chromium needs for the current
# Playwright version — replacing a manual dep list that may become incomplete.
RUN npm install -g playwright \
 && npx playwright install chromium \
 && apt-get update \
 && npx playwright install-deps chromium \
 && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/lunirelust .
COPY --from=builder /app/target/release/migration .

# Copy and rename environment file
# COPY --from=builder /app/.env.test .env

# # Copy assets if needed
# COPY assets ./assets

ENV RUST_LOG=info,sqlx=warn

ENTRYPOINT ["/app/lunirelust"]
