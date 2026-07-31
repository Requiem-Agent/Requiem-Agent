-- Migration 004: User Preferences & Settings Schema
-- S4-04: Comprehensive user settings with per-user agent configuration
-- Compatible with: PostgreSQL 14+ / Turso (libSQL)
-- Applied by: migrate::run() at startup

-- ─────────────────────────────────────────────────────────────────────────────
-- 1. user_preferences — core settings table
-- ─────────────────────────────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS user_preferences (
    id              TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
    user_id         TEXT NOT NULL UNIQUE,

    -- UI / UX
    theme           TEXT NOT NULL DEFAULT 'dark'
                        CHECK (theme IN ('dark', 'light', 'system')),
    language        TEXT NOT NULL DEFAULT 'en'
                        CHECK (length(language) BETWEEN 2 AND 10),
    timezone        TEXT NOT NULL DEFAULT 'UTC',
    date_format     TEXT NOT NULL DEFAULT 'YYYY-MM-DD',
    time_format     TEXT NOT NULL DEFAULT '24h'
                        CHECK (time_format IN ('12h', '24h')),

    -- Agent behaviour
    default_model   TEXT NOT NULL DEFAULT 'claude-3-5-sonnet-20241022',
    default_mode    TEXT NOT NULL DEFAULT 'chat'
                        CHECK (default_mode IN ('chat', 'orchestrator', 'auto')),
    max_steps       INTEGER NOT NULL DEFAULT 10
                        CHECK (max_steps BETWEEN 1 AND 50),
    temperature     REAL NOT NULL DEFAULT 0.7
                        CHECK (temperature BETWEEN 0.0 AND 2.0),
    stream_tokens   INTEGER NOT NULL DEFAULT 1   -- boolean: 1=true, 0=false
                        CHECK (stream_tokens IN (0, 1)),

    -- Notifications
    notify_on_done  INTEGER NOT NULL DEFAULT 1
                        CHECK (notify_on_done IN (0, 1)),
    notify_on_error INTEGER NOT NULL DEFAULT 1
                        CHECK (notify_on_error IN (0, 1)),
    notify_channel  TEXT NOT NULL DEFAULT 'in_app'
                        CHECK (notify_channel IN ('in_app', 'telegram', 'email', 'none')),

    -- Privacy & data
    save_history    INTEGER NOT NULL DEFAULT 1
                        CHECK (save_history IN (0, 1)),
    history_days    INTEGER NOT NULL DEFAULT 90
                        CHECK (history_days BETWEEN 1 AND 3650),
    allow_analytics INTEGER NOT NULL DEFAULT 0
                        CHECK (allow_analytics IN (0, 1)),

    -- Timestamps
    created_at      TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    updated_at      TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
);

CREATE INDEX IF NOT EXISTS idx_user_preferences_user_id
    ON user_preferences(user_id);

-- ─────────────────────────────────────────────────────────────────────────────
-- 2. user_api_keys — per-user LLM provider keys (encrypted at rest)
-- ─────────────────────────────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS user_api_keys (
    id              TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
    user_id         TEXT NOT NULL,
    provider        TEXT NOT NULL
                        CHECK (provider IN (
                            'openai', 'anthropic', 'google', 'mistral',
                            'cohere', 'groq', 'together', 'custom'
                        )),
    -- Key is stored AES-256-GCM encrypted; nonce prepended (base64)
    encrypted_key   TEXT NOT NULL,
    key_hint        TEXT,                          -- last 4 chars of the raw key, for display
    label           TEXT,                          -- user-defined label e.g. "Work OpenAI"
    is_active       INTEGER NOT NULL DEFAULT 1
                        CHECK (is_active IN (0, 1)),
    last_used_at    TEXT,
    created_at      TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    updated_at      TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),

    UNIQUE (user_id, provider, label)
);

CREATE INDEX IF NOT EXISTS idx_user_api_keys_user_id
    ON user_api_keys(user_id);
CREATE INDEX IF NOT EXISTS idx_user_api_keys_provider
    ON user_api_keys(user_id, provider);

-- ─────────────────────────────────────────────────────────────────────────────
-- 3. user_shortcuts — custom keyboard shortcuts / quick commands
-- ─────────────────────────────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS user_shortcuts (
    id          TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
    user_id     TEXT NOT NULL,
    trigger     TEXT NOT NULL,          -- e.g. "/summarize", "!fix"
    expansion   TEXT NOT NULL,          -- full prompt text to expand to
    description TEXT,
    sort_order  INTEGER NOT NULL DEFAULT 0,
    created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),

    UNIQUE (user_id, trigger)
);

CREATE INDEX IF NOT EXISTS idx_user_shortcuts_user_id
    ON user_shortcuts(user_id);

-- ─────────────────────────────────────────────────────────────────────────────
-- 4. user_workspace_settings — per-workspace overrides
-- ─────────────────────────────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS user_workspace_settings (
    id              TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
    user_id         TEXT NOT NULL,
    workspace_id    TEXT NOT NULL,

    -- Override global preferences for this workspace
    model_override  TEXT,               -- NULL = use global default
    mode_override   TEXT
                        CHECK (mode_override IS NULL OR
                               mode_override IN ('chat', 'orchestrator', 'auto')),
    system_prompt   TEXT,               -- custom system prompt for this workspace
    max_steps       INTEGER
                        CHECK (max_steps IS NULL OR max_steps BETWEEN 1 AND 50),
    temperature     REAL
                        CHECK (temperature IS NULL OR temperature BETWEEN 0.0 AND 2.0),

    -- Workspace-level rate limit overrides (NULL = use global)
    rate_limit_chat     INTEGER,
    rate_limit_sandbox  INTEGER,

    created_at      TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    updated_at      TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),

    UNIQUE (user_id, workspace_id)
);

CREATE INDEX IF NOT EXISTS idx_user_workspace_settings_user
    ON user_workspace_settings(user_id);
CREATE INDEX IF NOT EXISTS idx_user_workspace_settings_workspace
    ON user_workspace_settings(workspace_id);

-- ─────────────────────────────────────────────────────────────────────────────
-- 5. user_notification_log — audit trail for sent notifications
-- ─────────────────────────────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS user_notification_log (
    id          TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
    user_id     TEXT NOT NULL,
    channel     TEXT NOT NULL,          -- 'in_app' | 'telegram' | 'email'
    event_type  TEXT NOT NULL,          -- 'agent_done' | 'agent_error' | 'system'
    title       TEXT NOT NULL,
    body        TEXT,
    read_at     TEXT,                   -- NULL = unread
    sent_at     TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
);

CREATE INDEX IF NOT EXISTS idx_notification_log_user
    ON user_notification_log(user_id, sent_at DESC);
CREATE INDEX IF NOT EXISTS idx_notification_log_unread
    ON user_notification_log(user_id, read_at)
    WHERE read_at IS NULL;

-- ─────────────────────────────────────────────────────────────────────────────
-- 6. Trigger: auto-update updated_at on user_preferences
-- ─────────────────────────────────────────────────────────────────────────────

CREATE TRIGGER IF NOT EXISTS trg_user_preferences_updated_at
    AFTER UPDATE ON user_preferences
    FOR EACH ROW
BEGIN
    UPDATE user_preferences
    SET updated_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now')
    WHERE id = NEW.id;
END;

CREATE TRIGGER IF NOT EXISTS trg_user_api_keys_updated_at
    AFTER UPDATE ON user_api_keys
    FOR EACH ROW
BEGIN
    UPDATE user_api_keys
    SET updated_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now')
    WHERE id = NEW.id;
END;

CREATE TRIGGER IF NOT EXISTS trg_user_workspace_settings_updated_at
    AFTER UPDATE ON user_workspace_settings
    FOR EACH ROW
BEGIN
    UPDATE user_workspace_settings
    SET updated_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now')
    WHERE id = NEW.id;
END;

-- ─────────────────────────────────────────────────────────────────────────────
-- 7. Default preferences seed (optional — remove if handled in application)
-- ─────────────────────────────────────────────────────────────────────────────

-- No seed data: preferences are created on first login via INSERT OR IGNORE.
-- Application code should call:
--   INSERT OR IGNORE INTO user_preferences (user_id) VALUES (?);
-- to ensure a row exists before reading.
