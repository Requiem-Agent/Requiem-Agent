use anyhow::Result;
use libsql::{Builder, Connection};
use std::sync::Arc;
// S3-02: per-user rate limit state
use crate::rate_limit::MultiEndpointRateLimiter;
// S6-01: HasPreferencesDb trait implementation
use crate::routes::preferences::{HasPreferencesDb, UserPreferences};
// S6-02: HasApiKeysDb trait implementation
use crate::routes::user_api_keys::{HasApiKeysDb, StoredApiKey};

#[derive(Clone)]
pub struct AppState {
    pub conn: Arc<Connection>,
    pub bot_token: String,
    pub hf_token: String,
    pub hf_space_prdcn: String,
    pub session_secret: String,
    // S3-02: Rate limiter مُشترَك عبر جميع الـ handlers
    pub rate_limiter: Arc<MultiEndpointRateLimiter>,
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
                .unwrap_or_else(|_| {
                tracing::warn!("SESSION_SECRET not set — using ephemeral random secret. Sessions will not persist across restarts.");
                use std::collections::hash_map::DefaultHasher;
                use std::hash::{Hash, Hasher};
                use std::time::{SystemTime, UNIX_EPOCH};
                let mut hasher = DefaultHasher::new();
                SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_nanos().hash(&mut hasher);
                std::process::id().hash(&mut hasher);
                format!("ephemeral-{:x}-{:x}", hasher.finish(), rand_u64())
            }),
            // S3-02: تهيئة MultiEndpointRateLimiter من env variables
            rate_limiter: Arc::new(MultiEndpointRateLimiter::from_env()),
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

            -- ══════════════════════════════════════════════════════════════
            -- RAG MEMORY — persistent per-user store
            -- ══════════════════════════════════════════════════════════════
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

            -- ══════════════════════════════════════════════════════════════
            -- SESSION SUMMARIES — auto-generated on delete
            -- ══════════════════════════════════════════════════════════════
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

fn rand_u64() -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut h = DefaultHasher::new();
    std::time::SystemTime::now().hash(&mut h);
    h.finish()
}

// S4-03: WebSocket config — AppState implements HasWsConfig with production defaults
impl crate::routes::ws_agent::HasWsConfig for AppState {
    fn ws_max_message_size(&self) -> usize {
        std::env::var("WS_MAX_MESSAGE_SIZE")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(64 * 1024) // 64 KB
    }

    fn ws_timeout_secs(&self) -> u64 {
        std::env::var("WS_TIMEOUT_SECS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(300) // 5 minutes
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// S6-01: HasPreferencesDb — real SQLx implementation for AppState
// ═══════════════════════════════════════════════════════════════════════════
#[async_trait::async_trait]
impl HasPreferencesDb for AppState {
    /// جلب تفضيلات المستخدم من قاعدة البيانات.
    /// إذا لم يكن للمستخدم سجل بعد، يُعيد القيم الافتراضية.
    async fn get_preferences(&self, user_id: &str) -> Result<UserPreferences, crate::error::AppError> {
        let mut rows = self.conn
            .query(
                "SELECT theme, language, compact_mode, show_timestamps, enable_animations,
                        default_model, default_mode, max_tokens, temperature, system_prompt,
                        stream_responses, show_thinking,
                        notify_on_complete, notify_on_error, notify_on_mention,
                        save_history, share_analytics
                 FROM user_preferences
                 WHERE user_id = ?1
                 LIMIT 1",
                libsql::params![user_id],
            )
            .await
            .map_err(|e| crate::error::AppError::Database(e.to_string()))?;

        if let Some(row) = rows.next().await.map_err(|e| crate::error::AppError::Database(e.to_string()))? {
            // بناء UserPreferences من الصف
            let prefs = UserPreferences {
                theme: row.get::<String>(0).unwrap_or_else(|_| "dark".to_string()),
                language: row.get::<String>(1).unwrap_or_else(|_| "en".to_string()),
                compact_mode: row.get::<bool>(2).unwrap_or(false),
                show_timestamps: row.get::<bool>(3).unwrap_or(true),
                enable_animations: row.get::<bool>(4).unwrap_or(true),
                default_model: row.get::<String>(5).unwrap_or_else(|_| "claude-3-5-sonnet-20241022".to_string()),
                default_mode: row.get::<String>(6).unwrap_or_else(|_| "chat".to_string()),
                max_tokens: row.get::<i64>(7).unwrap_or(4096) as i32,
                temperature: row.get::<f64>(8).unwrap_or(0.7),
                system_prompt: {
                    let s = row.get::<String>(9).unwrap_or_default();
                    if s.is_empty() { None } else { Some(s) }
                },
                stream_responses: row.get::<bool>(10).unwrap_or(true),
                show_thinking: row.get::<bool>(11).unwrap_or(false),
                notify_on_complete: row.get::<bool>(12).unwrap_or(true),
                notify_on_error: row.get::<bool>(13).unwrap_or(true),
                notify_on_mention: row.get::<bool>(14).unwrap_or(true),
                save_history: row.get::<bool>(15).unwrap_or(true),
                share_analytics: row.get::<bool>(16).unwrap_or(false),
            };
            Ok(prefs)
        } else {
            // لا يوجد سجل — أنشئ سجلاً بالقيم الافتراضية وأعده
            let defaults = UserPreferences::default();
            AppState::insert_default_preferences(&self.conn, user_id, &defaults).await?;
            Ok(defaults)
        }
    }

    /// Upsert preferences — matches the HasPreferencesDb trait signature.
    async fn upsert_preferences(
        &self,
        user_id: &str,
        req: &crate::routes::preferences::UpdatePreferencesRequest,
    ) -> Result<Vec<String>, crate::error::AppError> {
        use crate::routes::preferences::build_update_fields;
        let (set_clause, updated_fields) = build_update_fields(req);
        if updated_fields.is_empty() {
            return Ok(vec![]);
        }
        let now = chrono::Utc::now().to_rfc3339();
        self.conn
            .execute(
                &format!(
                    "INSERT OR IGNORE INTO user_preferences (user_id, created_at, updated_at)
                     VALUES (?1, ?2, ?2);
                     UPDATE user_preferences SET {set_clause}, updated_at = ?2 WHERE user_id = ?1",
                    set_clause = set_clause,
                ),
                libsql::params![user_id, now],
            )
            .await
            .map_err(|e| crate::error::AppError::Database(e.to_string()))?;
        Ok(updated_fields)
    }
}

impl AppState {
    /// Insert default preferences row (used when no row exists yet).
    async fn insert_default_preferences(
        conn: &libsql::Connection,
        user_id: &str,
        prefs: &UserPreferences,
    ) -> Result<(), crate::error::AppError> {
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "INSERT OR IGNORE INTO user_preferences (
                user_id, theme, language, compact_mode, show_timestamps, enable_animations,
                default_model, default_mode, max_tokens, temperature, system_prompt,
                stream_responses, show_thinking,
                notify_on_complete, notify_on_error, notify_on_mention,
                save_history, share_analytics, created_at, updated_at
             ) VALUES (
                ?1, ?2, ?3, ?4, ?5, ?6,
                ?7, ?8, ?9, ?10, ?11,
                ?12, ?13,
                ?14, ?15, ?16,
                ?17, ?18, ?19, ?19
             )",
            libsql::params![
                user_id,
                prefs.theme.as_str(), prefs.language.as_str(),
                prefs.compact_mode, prefs.show_timestamps, prefs.enable_animations,
                prefs.default_model.as_str(), prefs.default_mode.as_str(),
                prefs.max_tokens as i64, prefs.temperature,
                prefs.system_prompt.as_deref().unwrap_or(""),
                prefs.stream_responses, prefs.show_thinking,
                prefs.notify_on_complete, prefs.notify_on_error, prefs.notify_on_mention,
                prefs.save_history, prefs.share_analytics,
                now.as_str(),
            ],
        )
        .await
        .map_err(|e| crate::error::AppError::Database(e.to_string()))?;
        Ok(())
    }

    /// UPSERT داخلي لـ user_preferences (legacy — kept for compatibility)
    #[allow(dead_code)]
    async fn upsert_preferences_full(
        &self,
        user_id: &str,
        prefs: &UserPreferences,
    ) -> Result<(), crate::error::AppError> {
        let now = chrono::Utc::now().to_rfc3339();
        self.conn
            .execute(
                "INSERT INTO user_preferences (
                    user_id, theme, language, compact_mode, show_timestamps, enable_animations,
                    default_model, default_mode, max_tokens, temperature, system_prompt,
                    stream_responses, show_thinking,
                    notify_on_complete, notify_on_error, notify_on_mention,
                    save_history, share_analytics, created_at, updated_at
                 ) VALUES (
                    ?1, ?2, ?3, ?4, ?5, ?6,
                    ?7, ?8, ?9, ?10, ?11,
                    ?12, ?13,
                    ?14, ?15, ?16,
                    ?17, ?18, ?19, ?19
                 )
                 ON CONFLICT(user_id) DO UPDATE SET
                    theme = excluded.theme,
                    language = excluded.language,
                    compact_mode = excluded.compact_mode,
                    show_timestamps = excluded.show_timestamps,
                    enable_animations = excluded.enable_animations,
                    default_model = excluded.default_model,
                    default_mode = excluded.default_mode,
                    max_tokens = excluded.max_tokens,
                    temperature = excluded.temperature,
                    system_prompt = excluded.system_prompt,
                    stream_responses = excluded.stream_responses,
                    show_thinking = excluded.show_thinking,
                    notify_on_complete = excluded.notify_on_complete,
                    notify_on_error = excluded.notify_on_error,
                    notify_on_mention = excluded.notify_on_mention,
                    save_history = excluded.save_history,
                    share_analytics = excluded.share_analytics,
                    updated_at = excluded.updated_at",
                libsql::params![
                    user_id,
                    prefs.theme.as_str(),
                    prefs.language.as_str(),
                    prefs.compact_mode,
                    prefs.show_timestamps,
                    prefs.enable_animations,
                    prefs.default_model.as_str(),
                    prefs.default_mode.as_str(),
                    prefs.max_tokens as i64,
                    prefs.temperature,
                    prefs.system_prompt.as_deref().unwrap_or(""),
                    prefs.stream_responses,
                    prefs.show_thinking,
                    prefs.notify_on_complete,
                    prefs.notify_on_error,
                    prefs.notify_on_mention,
                    prefs.save_history,
                    prefs.share_analytics,
                    now.as_str(),
                ],
            )
            .await
            .map_err(|e| crate::error::AppError::Database(e.to_string()))?;
        Ok(())
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// S6-02: HasApiKeysDb — real SQLx implementation for AppState
// ═══════════════════════════════════════════════════════════════════════════
#[async_trait::async_trait]
impl HasApiKeysDb for AppState {
    /// جلب جميع مفاتيح API للمستخدم (مشفّرة — لا تُعاد plaintext)
    async fn list_api_keys(&self, user_id: &str) -> Result<Vec<StoredApiKey>, crate::error::AppError> {
        let mut rows = self.conn
            .query(
                "SELECT id, provider, key_hint, encrypted_key, created_at, updated_at
                 FROM user_api_keys
                 WHERE user_id = ?1
                 ORDER BY created_at DESC",
                libsql::params![user_id],
            )
            .await
            .map_err(|e| crate::error::AppError::Database(e.to_string()))?;

        let mut keys = Vec::new();
        while let Some(row) = rows.next().await.map_err(|e| crate::error::AppError::Database(e.to_string()))? {
            keys.push(StoredApiKey {
                id: row.get::<String>(0).unwrap_or_default(),
                provider: row.get::<String>(1).unwrap_or_default(),
                key_hint: row.get::<String>(2).unwrap_or_default(),
                encrypted_key: row.get::<String>(3).unwrap_or_default(),
                created_at: row.get::<String>(4).unwrap_or_default(),
                updated_at: row.get::<String>(5).unwrap_or_default(),
            });
        }
        Ok(keys)
    }

    /// حفظ مفتاح API مشفّر جديد (أو تحديث موجود لنفس الـ provider)
    async fn save_api_key(
        &self,
        user_id: &str,
        provider: &str,
        encrypted_key: &str,
        key_hint: &str,
    ) -> Result<StoredApiKey, crate::error::AppError> {
        let id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();

        self.conn
            .execute(
                "INSERT INTO user_api_keys (id, user_id, provider, key_hint, encrypted_key, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)
                 ON CONFLICT(user_id, provider) DO UPDATE SET
                    key_hint = excluded.key_hint,
                    encrypted_key = excluded.encrypted_key,
                    updated_at = excluded.updated_at",
                libsql::params![id.as_str(), user_id, provider, key_hint, encrypted_key, now.as_str()],
            )
            .await
            .map_err(|e| crate::error::AppError::Database(e.to_string()))?;

        Ok(StoredApiKey {
            id,
            provider: provider.to_string(),
            key_hint: key_hint.to_string(),
            encrypted_key: encrypted_key.to_string(),
            created_at: now.clone(),
            updated_at: now,
        })
    }

    /// حذف مفتاح API بالـ id
    async fn delete_api_key(&self, user_id: &str, key_id: &str) -> Result<(), crate::error::AppError> {
        self.conn
            .execute(
                "DELETE FROM user_api_keys WHERE id = ?1 AND user_id = ?2",
                libsql::params![key_id, user_id],
            )
            .await
            .map_err(|e| crate::error::AppError::Database(e.to_string()))?;
        Ok(())
    }

    /// جلب مفتاح API مشفّر واحد (للفك عند الاستخدام)
    async fn get_encrypted_key(
        &self,
        user_id: &str,
        provider: &str,
    ) -> Result<Option<String>, crate::error::AppError> {
        let mut rows = self.conn
            .query(
                "SELECT encrypted_key FROM user_api_keys WHERE user_id = ?1 AND provider = ?2 LIMIT 1",
                libsql::params![user_id, provider],
            )
            .await
            .map_err(|e| crate::error::AppError::Database(e.to_string()))?;

        if let Some(row) = rows.next().await.map_err(|e| crate::error::AppError::Database(e.to_string()))? {
            Ok(Some(row.get::<String>(0).unwrap_or_default()))
        } else {
            Ok(None)
        }
    }
}