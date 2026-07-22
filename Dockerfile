# ─── Stage 1: Build Rust backend ───────────────────────────────────────────
FROM rust:1.88-slim AS rust-builder

WORKDIR /app/rust-backend

RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    cmake \
    libsqlite3-dev \
    zlib1g-dev \
    clang \
    libclang-dev \
    libseccomp-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy manifests first for cached dependency layer
COPY rust-backend/Cargo.toml ./Cargo.toml
COPY rust-backend/Cargo.lock ./Cargo.lock

# Dummy build to cache dependencies only
RUN mkdir -p src && echo 'fn main() {}' > src/main.rs
RUN rm -f target/release/requiem-server
RUN cargo build --release --locked 2>/dev/null || cargo build --release
RUN rm -f src/main.rs target/release/deps/requiem_server*

# Real build — copy src + migrations (needed by include_str! at compile time)
COPY rust-backend/src ./src
COPY rust-backend/migrations ./migrations
RUN cargo build --release --locked

# ─── Stage 2: Runtime (Sprint 2: +ttyd +nginx terminal support) ────────
FROM debian:bookworm-slim

WORKDIR /app

RUN apt-get update && apt-get install -y \
    ca-certificates tini curl wget nginx procps bash \
    python3 python3-pip nodejs npm \
    && rm -rf /var/lib/apt/lists/*

# Install ttyd — terminal over WebSocket (proven on HF Spaces)
RUN wget -qO /usr/local/bin/ttyd https://github.com/tsl0922/ttyd/releases/download/1.7.7/ttyd.x86_64 \
    && chmod +x /usr/local/bin/ttyd

# Copy Rust binary only — no frontend
COPY --from=rust-builder /app/rust-backend/target/release/requiem-server ./requiem-server

# Copy nginx reverse proxy config and entrypoint while still ROOT
COPY nginx.conf /etc/nginx/nginx.conf
COPY entrypoint.sh /app/entrypoint.sh
RUN chmod +x /app/entrypoint.sh

# Create non-root user with UID 1000 (matches HF bucket mount ownership)
# /data is mounted by HF with UID:GID = 1000:1000
RUN useradd -u 1000 -m -d /home/appuser -s /bin/false appuser \
    && mkdir -p /data \
    && chown -R appuser:appuser /data /app \
    && chown -R appuser:appuser /var/log/nginx /var/lib/nginx /etc/nginx /run

USER appuser

ENV PORT=7860
ENV RUST_LOG=info

EXPOSE 7860

HEALTHCHECK --interval=30s --timeout=10s --start-period=90s --retries=3 \
    CMD curl -sf http://localhost:7860/api/healthz || exit 1

ENTRYPOINT ["/usr/bin/tini", "--"]
CMD ["/app/entrypoint.sh"]
