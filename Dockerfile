# ── Builder stage ────────────────────────────────────────────────
FROM rust:1.75-bookworm AS builder

WORKDIR /app

# Cache dependencies by building with just the manifests first
COPY Cargo.toml Cargo.lock* ./
RUN mkdir src && echo "fn main() {}" > src/main.rs && \
    cargo build --release 2>/dev/null || true && \
    rm -rf src

# Copy source and build
COPY . .
RUN cargo build --release

# ── Runtime stage ────────────────────────────────────────────────
FROM debian:bookworm-slim AS runtime

RUN apt-get update && \
    apt-get install -y --no-install-recommends \
        ca-certificates \
        libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN groupadd -r tslink && useradd -r -g tslink tslink

WORKDIR /app

# Copy binary
COPY --from=builder /app/target/release/tslink /app/tslink

# Copy config files
COPY config/ /app/config/

# Set ownership
RUN chown -R tslink:tslink /app

USER tslink

# HTTP port
EXPOSE 8080
# Metrics port (if separate)
EXPOSE 9090

ENV RUST_LOG=info
ENV RUN_ENV=production

CMD ["/app/tslink"]
