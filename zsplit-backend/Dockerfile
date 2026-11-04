# Multi-stage Dockerfile for Z-Split Backend
# Produces a secure, minimal container running the combined Axum API + Wallet Worker.

# ==============================================================================
# Builder Stage
# ==============================================================================
FROM rust:1.94-bookworm AS builder

RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config \
    libssl-dev \
    protobuf-compiler \
    libsqlite3-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy dependency manifests
COPY Cargo.toml Cargo.lock ./
COPY crates/domain/Cargo.toml crates/domain/
COPY crates/zcash/Cargo.toml crates/zcash/
COPY crates/claims/Cargo.toml crates/claims/
COPY crates/store/Cargo.toml crates/store/
COPY apps/worker/Cargo.toml apps/worker/
COPY apps/api/Cargo.toml apps/api/
COPY tests/Cargo.toml tests/

# Copy all source trees and migrations
COPY crates/ crates/
COPY apps/ apps/
COPY tests/ tests/
COPY migrations/ migrations/
COPY benches/ benches/

# Build release binaries
RUN cargo build --release --bin zsplit-api --bin zsplit-bench

# ==============================================================================
# Runtime Stage
# ==============================================================================
FROM debian:bookworm-slim AS runtime

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libssl3 \
    libsqlite3-0 \
    curl \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Create non-root user
RUN groupadd -g 1000 zsplit && \
    useradd -u 1000 -g zsplit -m -s /bin/bash zsplit && \
    mkdir -p /app/data/wallet && \
    chown -R zsplit:zsplit /app

# Copy compiled binaries from builder
COPY --from=builder /app/target/release/zsplit-api /usr/local/bin/zsplit-api
COPY --from=builder /app/target/release/zsplit-bench /usr/local/bin/zsplit-bench
COPY --from=builder /app/migrations /app/migrations

USER zsplit

ENV PORT=8080 \
    ZCASH_DATA_DIR=/app/data/wallet \
    ZCASH_NETWORK=testnet \
    ZCASH_CONFIRMATIONS=3

EXPOSE 8080

HEALTHCHECK --interval=10s --timeout=5s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8080/healthz || exit 1

ENTRYPOINT ["/usr/local/bin/zsplit-api"]
