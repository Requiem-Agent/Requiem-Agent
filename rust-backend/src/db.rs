use anyhow::Result;
use libsql::{Builder, Connection};
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub conn: Arc<Connection>,
    pub bot_token: String,
    pub hf_token: String,
    pub hf_space_prdcn: String,
    pub session_secret: String,
}

impl AppState {
    pub async fn new(url: &str, auth_token: Option<String>) -> Result<Self> {
        let db = match auth_token {
            Some(token) => Builder::new_remote(url.to_string(), token).build().await?,
            None => Builder::new_local(url).build().await?,
        };
        let conn = db.connect()?;

        Ok(Self {
            conn: Arc::new(conn),
            bot_token: std::env::var("TELEGRAM_BOT_TOKEN")
                .unwrap_or_else(|_| "8335891917:AAGPVYHTtPAx3vcd-iIcVRw8H5lfOTwnA04".to_string()),
            hf_token: std::env::var("HF_TOKEN").unwrap_or_default(),
            hf_space_prdcn: std::env::var("HF_SPACE_PRDCN")
                .unwrap_or_else(|_| "rayig/Prdcn".to_string()),
            session_secret: std::env::var("SESSION_SECRET")
                .unwrap_or_else(|_| "fallback-secret".to_string()),
        })
    }

    pub async fn init_schema(&self) -> Result<()> {
        self.conn.execute_batch("
            CREATE TABLE IF NOT EXISTS users (
                id TEXT PRIMARY KEY,
                telegram_id INTEGER UNIQUE NOT NULL,
                first_name TEXT NOT NULL,
                last_name TEXT,
                username TEXT,
                quota_read_used INTEGER NOT NULL DEFAULT 0,
                quota_write_used INTEGER NOT NULL DEFAULT 0,
                quota_reset_at TEXT NOT NULL,
                created_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS sessions (
                id TEXT PRIMARY KEY,
                user_id TEXT NOT NULL,
                name TEXT NOT NULL,
                mode TEXT NOT NULL DEFAULT 'orchestrator',
                effort TEXT NOT NULL DEFAULT 'medium',
                active_model TEXT,
                message_count INTEGER NOT NULL DEFAULT 0,
                last_message_at TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS messages (
                id TEXT PRIMARY KEY,
                session_id TEXT NOT NULL,
                role TEXT NOT NULL,
                content TEXT NOT NULL,
                model_used TEXT,
                mode TEXT,
                effort TEXT,
                tool_calls TEXT,
                code_changes TEXT,
                created_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS bots (
                id TEXT PRIMARY KEY,
                user_id TEXT NOT NULL,
                name TEXT NOT NULL,
                username TEXT NOT NULL,
                bot_token TEXT,
                description TEXT,
                status TEXT NOT NULL DEFAULT 'pending',
                hf_space_url TEXT,
                deployed_at TEXT,
                created_at TEXT NOT NULL
            );

            -- ═══════════════════════════════════════════
            -- RAG MEMORY — persistent per-user store
            -- ═══════════════════════════════════════════
            CREATE TABLE IF NOT EXISTS memories (
                id TEXT PRIMARY KEY,
                user_id TEXT NOT NULL,
                session_id TEXT,
                content TEXT NOT NULL,
                memory_type TEXT NOT NULL DEFAULT 'context',
                priority TEXT NOT NULL DEFAULT 'medium',
                embedding TEXT,
                embedding_dim INTEGER DEFAULT 256,
                access_count INTEGER NOT NULL DEFAULT 0,
                last_accessed TEXT,
                is_summary INTEGER NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_memories_user_id
                ON memories(user_id);
            CREATE INDEX IF NOT EXISTS idx_memories_user_session
                ON memories(user_id, session_id);
            CREATE INDEX IF NOT EXISTS idx_memories_type
                ON memories(user_id, memory_type);

            -- ═══════════════════════════════════════════
            -- SESSION SUMMARIES — auto-generated on delete
            -- ═══════════════════════════════════════════
            CREATE TABLE IF NOT EXISTS session_summaries (
                id TEXT PRIMARY KEY,
                user_id TEXT NOT NULL,
                session_id TEXT NOT NULL UNIQUE,
                session_name TEXT NOT NULL,
                summary TEXT NOT NULL,
                key_facts TEXT,
                message_count INTEGER DEFAULT 0,
                created_at TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_session_summaries_user
                ON session_summaries(user_id);
        ").await?;
        Ok(())
    }
}
