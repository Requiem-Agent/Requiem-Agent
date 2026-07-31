-- migrations/005_conversations.sql
-- S8-02: Conversation history persistence
-- يحفظ المحادثات والرسائل في PostgreSQL مع دعم البحث والتصفح

-- ─────────────────────────────────────────────────────────────────────────────
-- conversations: كل محادثة بين مستخدم والـ agent
-- ─────────────────────────────────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS conversations (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id         TEXT NOT NULL,
    title           TEXT,                           -- عنوان المحادثة (يُولَّد تلقائياً من أول رسالة)
    model           TEXT NOT NULL DEFAULT 'claude-sonnet-4-5',
    provider        TEXT NOT NULL DEFAULT 'anthropic',
    mode            TEXT NOT NULL DEFAULT 'chat'    -- chat | orchestrator | code
                    CHECK (mode IN ('chat', 'orchestrator', 'code')),
    message_count   INTEGER NOT NULL DEFAULT 0,
    total_tokens    INTEGER NOT NULL DEFAULT 0,
    is_archived     BOOLEAN NOT NULL DEFAULT FALSE,
    is_pinned       BOOLEAN NOT NULL DEFAULT FALSE,
    last_message_at TIMESTAMPTZ,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Index للبحث السريع بالمستخدم
CREATE INDEX IF NOT EXISTS idx_conversations_user_id
    ON conversations (user_id, created_at DESC);

-- Index للمحادثات المثبّتة
CREATE INDEX IF NOT EXISTS idx_conversations_pinned
    ON conversations (user_id, is_pinned, last_message_at DESC)
    WHERE is_pinned = TRUE;

-- ─────────────────────────────────────────────────────────────────────────────
-- messages: رسائل كل محادثة
-- ─────────────────────────────────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS messages (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    conversation_id UUID NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
    user_id         TEXT NOT NULL,
    role            TEXT NOT NULL CHECK (role IN ('user', 'assistant', 'system', 'tool')),
    content         TEXT NOT NULL,
    model           TEXT,                           -- الـ model الذي أنتج هذه الرسالة
    provider        TEXT,
    tokens_used     INTEGER,
    -- ReAct metadata
    react_step      INTEGER,                        -- رقم الخطوة في الـ ReAct loop
    tool_name       TEXT,                           -- اسم الـ tool إذا كانت tool message
    tool_args       JSONB,                          -- arguments الـ tool
    tool_result     TEXT,                           -- نتيجة الـ tool
    -- Timing
    latency_ms      INTEGER,                        -- وقت توليد الرسالة بالـ ms
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Index للجلب السريع لرسائل محادثة معينة
CREATE INDEX IF NOT EXISTS idx_messages_conversation_id
    ON messages (conversation_id, created_at ASC);

-- Index للبحث في محتوى الرسائل (full-text search)
CREATE INDEX IF NOT EXISTS idx_messages_content_fts
    ON messages USING gin(to_tsvector('arabic', content));

-- ─────────────────────────────────────────────────────────────────────────────
-- conversation_summaries: ملخصات المحادثات الطويلة (للـ RAG context)
-- ─────────────────────────────────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS conversation_summaries (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    conversation_id UUID NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
    summary         TEXT NOT NULL,
    messages_covered INTEGER NOT NULL,              -- عدد الرسائل التي يغطيها الملخص
    model_used      TEXT NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_summaries_conversation
    ON conversation_summaries (conversation_id, created_at DESC);

-- ─────────────────────────────────────────────────────────────────────────────
-- conversation_tags: تصنيف المحادثات
-- ─────────────────────────────────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS conversation_tags (
    conversation_id UUID NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
    tag             TEXT NOT NULL,
    PRIMARY KEY (conversation_id, tag)
);

CREATE INDEX IF NOT EXISTS idx_tags_user_tag
    ON conversation_tags (tag);

-- ─────────────────────────────────────────────────────────────────────────────
-- Triggers: تحديث تلقائي للـ metadata
-- ─────────────────────────────────────────────────────────────────────────────

-- تحديث message_count و last_message_at عند إضافة رسالة جديدة
CREATE OR REPLACE FUNCTION update_conversation_on_message()
RETURNS TRIGGER AS $$
BEGIN
    UPDATE conversations
    SET
        message_count   = message_count + 1,
        total_tokens    = total_tokens + COALESCE(NEW.tokens_used, 0),
        last_message_at = NEW.created_at,
        updated_at      = NOW()
    WHERE id = NEW.conversation_id;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trg_update_conversation_on_message ON messages;
CREATE TRIGGER trg_update_conversation_on_message
    AFTER INSERT ON messages
    FOR EACH ROW
    EXECUTE FUNCTION update_conversation_on_message();

-- تحديث updated_at تلقائياً
CREATE OR REPLACE FUNCTION update_conversations_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trg_conversations_updated_at ON conversations;
CREATE TRIGGER trg_conversations_updated_at
    BEFORE UPDATE ON conversations
    FOR EACH ROW
    EXECUTE FUNCTION update_conversations_updated_at();

-- ─────────────────────────────────────────────────────────────────────────────
-- Views: مساعدة للاستعلامات الشائعة
-- ─────────────────────────────────────────────────────────────────────────────

-- ملخص المحادثات مع آخر رسالة
CREATE OR REPLACE VIEW conversation_list AS
SELECT
    c.id,
    c.user_id,
    COALESCE(c.title, 'محادثة جديدة') AS title,
    c.model,
    c.provider,
    c.mode,
    c.message_count,
    c.total_tokens,
    c.is_archived,
    c.is_pinned,
    c.last_message_at,
    c.created_at,
    -- آخر رسالة للمستخدم (للـ preview)
    (
        SELECT LEFT(content, 100)
        FROM messages m
        WHERE m.conversation_id = c.id AND m.role = 'user'
        ORDER BY m.created_at DESC
        LIMIT 1
    ) AS last_user_message
FROM conversations c
WHERE c.is_archived = FALSE;

-- تسجيل الـ migration
INSERT INTO schema_migrations (version, description, applied_at)
VALUES ('005', 'conversation history persistence', NOW())
ON CONFLICT (version) DO NOTHING;
