-- ═══════════════════════════════════════════════════════════════════════════
-- Migration 003 — Rate Limits Audit + Metrics Snapshots
-- Requiem Agent — S3-05
-- ═══════════════════════════════════════════════════════════════════════════

-- ─── Rate Limit Events (audit trail) ─────────────────────────────────────────
-- يُسجِّل كل مرة يُرفض فيها طلب بسبب rate limiting
CREATE TABLE IF NOT EXISTS rate_limit_events (
    id              TEXT        PRIMARY KEY,
    user_id         TEXT,       -- NULL إذا كان الرفض قبل المصادقة
    ip_address      TEXT,
    endpoint        TEXT        NOT NULL,
    rejected_at     TEXT        NOT NULL,
    window_start    TEXT        NOT NULL,
    request_count   INTEGER     NOT NULL DEFAULT 1
);

CREATE INDEX IF NOT EXISTS idx_rate_limit_user    ON rate_limit_events(user_id);
CREATE INDEX IF NOT EXISTS idx_rate_limit_endpoint ON rate_limit_events(endpoint);
CREATE INDEX IF NOT EXISTS idx_rate_limit_time    ON rate_limit_events(rejected_at);

-- ─── Metrics Snapshots (hourly aggregates) ────────────────────────────────────
-- يُخزِّن لقطات ساعية من مقاييس Prometheus للتحليل التاريخي
CREATE TABLE IF NOT EXISTS metrics_snapshots (
    id                      TEXT    PRIMARY KEY,
    snapshot_at             TEXT    NOT NULL,
    http_requests_total     INTEGER NOT NULL DEFAULT 0,
    agent_steps_total       INTEGER NOT NULL DEFAULT 0,
    llm_calls_success       INTEGER NOT NULL DEFAULT 0,
    llm_calls_failure       INTEGER NOT NULL DEFAULT 0,
    rate_limit_hits_total   INTEGER NOT NULL DEFAULT 0,
    avg_response_ms         REAL    NOT NULL DEFAULT 0.0,
    p95_response_ms         REAL    NOT NULL DEFAULT 0.0,
    active_users            INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX IF NOT EXISTS idx_metrics_snapshot_at ON metrics_snapshots(snapshot_at);

-- ─── User Quotas (per-user rate limit state) ──────────────────────────────────
-- يُتتبَّع استخدام كل مستخدم لحساب per-user rate limits
CREATE TABLE IF NOT EXISTS user_quotas (
    user_id         TEXT        PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    -- Chat endpoint
    chat_count      INTEGER     NOT NULL DEFAULT 0,
    chat_window_start TEXT      NOT NULL DEFAULT (datetime('now')),
    -- Sandbox endpoint
    sandbox_count   INTEGER     NOT NULL DEFAULT 0,
    sandbox_window_start TEXT   NOT NULL DEFAULT (datetime('now')),
    -- Auth endpoint
    auth_count      INTEGER     NOT NULL DEFAULT 0,
    auth_window_start TEXT      NOT NULL DEFAULT (datetime('now')),
    -- General API
    api_count       INTEGER     NOT NULL DEFAULT 0,
    api_window_start TEXT       NOT NULL DEFAULT (datetime('now')),
    -- Metadata
    updated_at      TEXT        NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_user_quotas_updated ON user_quotas(updated_at);
