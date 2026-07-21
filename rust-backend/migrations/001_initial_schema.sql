-- ═══════════════════════════════════════════════════════════════════════════
-- Migration 001 — Initial Schema
-- Requiem Agent — S3-05
-- Compatible with: Turso (libSQL) + PostgreSQL
-- ═══════════════════════════════════════════════════════════════════════════

-- ─── Users ───────────────────────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS users (
    id              TEXT        PRIMARY KEY,
    telegram_id     INTEGER     UNIQUE NOT NULL,
    first_name      TEXT        NOT NULL,
    last_name       TEXT,
    username        TEXT,
    quota_read_used  INTEGER    NOT NULL DEFAULT 0,
    quota_write_used INTEGER    NOT NULL DEFAULT 0,
    quota_reset_at  TEXT        NOT NULL,
    created_at      TEXT        NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_users_telegram_id ON users(telegram_id);

-- ─── Sessions ────────────────────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS sessions (
    id              TEXT        PRIMARY KEY,
    user_id         TEXT        NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name            TEXT        NOT NULL,
    mode            TEXT        NOT NULL DEFAULT 'orchestrator',
    effort          TEXT        NOT NULL DEFAULT 'medium',
    active_model    TEXT,
    message_count   INTEGER     NOT NULL DEFAULT 0,
    last_message_at TEXT,
    created_at      TEXT        NOT NULL,
    updated_at      TEXT        NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_sessions_user_id ON sessions(user_id);
CREATE INDEX IF NOT EXISTS idx_sessions_updated_at ON sessions(updated_at);

-- ─── Messages ────────────────────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS messages (
    id              TEXT        PRIMARY KEY,
    session_id      TEXT        NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
    role            TEXT        NOT NULL CHECK (role IN ('user', 'assistant', 'system', 'tool')),
    content         TEXT        NOT NULL,
    model_used      TEXT,
    mode            TEXT,
    effort          TEXT,
    tool_calls      TEXT,       -- JSON array of tool calls
    code_changes    TEXT,       -- JSON array of file changes
    created_at      TEXT        NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_messages_session_id ON messages(session_id);
CREATE INDEX IF NOT EXISTS idx_messages_created_at ON messages(created_at);

-- ─── Bots ─────────────────────────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS bots (
    id              TEXT        PRIMARY KEY,
    user_id         TEXT        NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name            TEXT        NOT NULL,
    username        TEXT        NOT NULL,
    bot_token       TEXT,
    description     TEXT,
    status          TEXT        NOT NULL DEFAULT 'pending'
                                CHECK (status IN ('pending', 'running', 'stopped', 'error', 'deploying')),
    hf_space_url    TEXT,
    deployed_at     TEXT,
    created_at      TEXT        NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_bots_user_id ON bots(user_id);
CREATE INDEX IF NOT EXISTS idx_bots_status ON bots(status);
