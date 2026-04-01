# ==============================================================================
# Multi-stage Dockerfile for monobase-api (Rust)
# ==============================================================================

# Stage 1: Build
FROM rust:1.91-bookworm AS builder

WORKDIR /build

# Cache dependencies: copy manifests first, build a dummy to cache deps
COPY Cargo.toml Cargo.lock* ./
RUN mkdir src && echo "fn main() {}" > src/main.rs && echo "" > src/lib.rs
RUN cargo build --release 2>/dev/null || true
RUN rm -rf src

# Copy full source and build
COPY src/ src/
COPY migrations/ migrations/
RUN cargo build --release

# Stage 2: Runtime
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy the binary
COPY --from=builder /build/target/release/monobase-api /app/monobase-api

# Copy migrations (for runtime migration support)
COPY migrations/ /app/migrations/

# Default port
ENV SERVER_PORT=7213
ENV SERVER_HOST=0.0.0.0

EXPOSE 7213

ENTRYPOINT ["/app/monobase-api"]
