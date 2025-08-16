FROM rust:1.86-slim AS builder

WORKDIR /app

# Install build dependencies
RUN apt-get update && apt-get install -y libssl-dev pkg-config curl

# Copy sources
COPY . .

# Build in release mode
RUN cargo build --release

# Build migration tool as binary
WORKDIR /app/migration
RUN cargo build --release -p migration

# Create runtime image
FROM debian:stable-slim

WORKDIR /app

RUN apt-get update && apt-get install -y libssl3 ca-certificates curl file && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/lunirelust .
COPY --from=builder /app/target/release/migration .

# Copy and rename environment file
# COPY --from=builder /app/.env.test .env

# # Copy assets if needed
# COPY assets ./assets

ENV RUST_LOG=info

ENTRYPOINT ["/app/lunirelust"]
