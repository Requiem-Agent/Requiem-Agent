# ─── Stage 1: Build Rust backend ───────────────────────────────────────────
FROM rust:1.85-slim AS rust-builder

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

COPY rust-backend/Cargo.toml rust-backend/Cargo.lock ./Cargo.toml ./Cargo.lock

RUN mkdir -p src && echo 'fn main() {}' > src/main.rs && \
    cargo build --release --locked 2>/dev/null || cargo build --release && \
    rm -f src/main.rs target/release/deps/requiem_server*

COPY rust-backend/src ./src
COPY rust-backend/migrations ./migrations
RUN cargo build --release --locked

# ─── Stage 2: Runtime ──────────────────────────────────────────────────
FROM debian:bookworm-slim

WORKDIR /app

RUN apt-get update && apt-get install -y \
    ca-certificates tini curl wget nginx procps bash \
    && rm -rf /var/lib/apt/lists/*

RUN wget -qO /usr/local/bin/ttyd https://github.com/tsl0922/ttyd/releases/download/1.7.7/ttyd.x86_64 \
    && chmod +x /usr/local/bin/ttyd

COPY --from=rust-builder /app/rust-backend/target/release/requiem-server ./requiem-server

COPY nginx.conf /etc/nginx/nginx.conf
COPY entrypoint.sh /app/entrypoint.sh
RUN chmod +x /app/entrypoint.sh

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
