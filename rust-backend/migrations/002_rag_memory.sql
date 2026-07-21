-- ═══════════════════════════════════════════════════════════════════════════
-- Migration 002 — RAG Memory System
-- Requiem Agent — S3-05
-- ═══════════════════════════════════════════════════════════════════════════

-- ─── Memories (RAG persistent store) ─────────────────────────────────────────
CREATE TABLE IF NOT EXISTS memories (
    id              TEXT        PRIMARY KEY,
    user_id         TEXT        NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    session_id      TEXT,
    content         TEXT        NOT NULL,
    memory_type     TEXT        NOT NULL DEFAULT 'context'
                                CHECK (memory_type IN ('context', 'fact', 'preference', 'summary', 'code')),
    priority        TEXT        NOT NULL DEFAULT 'medium'
                                CHECK (priority IN ('low', 'medium', 'high', 'critical')),
    embedding       TEXT,       -- JSON array of floats (256-dim)
    embedding_dim   INTEGER     DEFAULT 256,
    access_count    INTEGER     NOT NULL DEFAULT 0,
    last_accessed   TEXT,
    is_summary      INTEGER     NOT NULL DEFAULT 0 CHECK (is_summary IN (0, 1)),
    created_at      TEXT        NOT NULL,
    updated_at      TEXT        NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_memories_user_id        ON memories(user_id);
CREATE INDEX IF NOT EXISTS idx_memories_user_session   ON memories(user_id, session_id);
CREATE INDEX IF NOT EXISTS idx_memories_type           ON memories(user_id, memory_type);
CREATE INDEX IF NOT EXISTS idx_memories_priority       ON memories(user_id, priority);
CREATE INDEX IF NOT EXISTS idx_memories_access_count   ON memories(user_id, access_count DESC);

-- ─── Session Summaries ────────────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS session_summaries (
    id              TEXT        PRIMARY KEY,
    user_id         TEXT        NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    session_id      TEXT        NOT NULL UNIQUE,
    session_name    TEXT        NOT NULL,
    summary         TEXT        NOT NULL,
    key_facts       TEXT,       -- JSON array of key facts
    message_count   INTEGER     DEFAULT 0,
    created_at      TEXT        NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_session_summaries_user ON session_summaries(user_id);
CREATE INDEX IF NOT EXISTS idx_session_summaries_session ON session_summaries(session_id);
