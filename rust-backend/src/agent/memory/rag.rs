//! # RAG Engine — Turso-persisted, per-user isolated
//!
//! كل instance مرتبط بـ user_id واحد.
//! جميع الاستعلامات تشمل WHERE user_id = ? — لا تسرب بين المستخدمين.

use std::sync::Arc;
use anyhow::Result;
use uuid::Uuid;
use chrono::Utc;
use libsql::Connection;
use tracing::{debug, warn};

use super::embeddings::{generate_embedding, cosine_similarity, priority_weight, recency_weight_from_iso};

pub const MIN_SIMILARITY: f32 = 0.25;
pub const MAX_FETCH: usize = 500;
pub const DEFAULT_LIMIT: usize = 10;

// ── Types ─────────────────────────────────────────────────────────────────
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct StoredMemory {
    pub id: String,
    pub user_id: String,
    pub session_id: Option<String>,
    pub content: String,
    pub memory_type: String,
    pub priority: String,
    pub access_count: i64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct RetrievedMemory {
    #[serde(flatten)]
    pub memory: StoredMemory,
    pub score: f32,
}

#[derive(Debug, serde::Serialize)]
pub struct RagBuildResult {
    pub system_context: String,
    pub memories_used: usize,
    pub token_count: usize,
    pub sources: Vec<MemorySource>,
}

#[derive(Debug, serde::Serialize)]
pub struct MemorySource {
    pub id: String,
    #[serde(rename = "type")]
    pub memory_type: String,
    pub score: f32,
}

#[derive(Debug, serde::Serialize)]
pub struct RagStats {
    pub total: i64,
    pub by_type: std::collections::HashMap<String, i64>,
    pub by_priority: std::collections::HashMap<String, i64>,
}

// ── Auto-extract memories from conversation ────────────────────────────────
fn auto_extract(user_msg: &str, assistant_msg: &str) -> Vec<(String, &'static str, &'static str)> {
    let mut results = Vec::new();

    // Extract code blocks
    let mut rest = assistant_msg;
    while let Some(start) = rest.find("```") {
        rest = &rest[start + 3..];
        if let Some(end) = rest.find("```") {
            let code = rest[..end].trim();
            // Skip language tag line
            let code = code.lines().skip(1).collect::<Vec<_>>().join("\n");
            if code.len() > 20 && code.len() < 2000 {
                results.push((code.to_string(), "code", "high"));
            }
            rest = &rest[end + 3..];
        } else { break; }
    }

    // Extract facts (Arabic + English patterns)
    let fact_patterns = [
        "يستخدم ", "يعتمد على ", "المشروع ", "the project uses ", "depends on ",
    ];
    for pattern in &fact_patterns {
        let lower = assistant_msg.to_lowercase();
        if let Some(pos) = lower.find(pattern) {
            let snippet = &assistant_msg[pos..];
            let end = snippet.find('\n').unwrap_or(snippet.len().min(120));
            let fact = snippet[..end].trim();
            if fact.len() > 10 {
                results.push((fact.to_string(), "fact", "medium"));
            }
        }
    }

    // User preference detection
    let pref_signals = ["أفضّل", "أحب", "دائماً", "prefer", "always use", "i like"];
    if pref_signals.iter().any(|s| user_msg.to_lowercase().contains(s)) && user_msg.len() < 300 {
        results.push((user_msg.to_string(), "preference", "high"));
    }

    results.truncate(5);
    results
}

// ── RAG Engine ────────────────────────────────────────────────────────────
pub struct RagEngine {
    conn: Arc<Connection>,
    user_id: String,
}

impl RagEngine {
    /// Create a per-user RAG engine — every query scoped to user_id
    pub fn new(conn: Arc<Connection>, user_id: impl Into<String>) -> Self {
        Self { conn, user_id: user_id.into() }
    }

    /// Store one memory, returns id
    pub async fn store(&self, content: &str, memory_type: &str, priority: &str, session_id: Option<&str>) -> Result<String> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        let embedding = serde_json::to_string(&generate_embedding(content))?;

        self.conn.execute(
            "INSERT INTO memories
             (id,user_id,session_id,content,memory_type,priority,embedding,embedding_dim,access_count,is_summary,created_at,updated_at)
             VALUES (?1,?2,?3,?4,?5,?6,?7,256,0,0,?8,?9)",
            libsql::params![
                id.clone(),
                self.user_id.clone(),
                session_id.map(String::from),
                content.to_string(),
                memory_type.to_string(),
                priority.to_string(),
                embedding,
                now.clone(),
                now,
            ],
        ).await?;

        debug!("RAG store: user={} type={} id={}", self.user_id, memory_type, &id[..8]);
        Ok(id)
    }

    /// Semantic search — hybrid score = similarity * priority_w * recency_w
    pub async fn retrieve(&self, query: &str, limit: usize, max_tokens: usize, session_id: Option<&str>) -> Result<Vec<RetrievedMemory>> {
        let limit = limit.min(50);

        let mut rows = self.conn.query(
            "SELECT id,user_id,session_id,content,memory_type,priority,embedding,access_count,created_at,updated_at
             FROM memories WHERE user_id=?1
             ORDER BY created_at DESC LIMIT ?2",
            libsql::params![self.user_id.clone(), MAX_FETCH as i64],
        ).await?;

        let query_emb = generate_embedding(query);
        let mut scored: Vec<(f32, StoredMemory)> = Vec::new();

        while let Ok(Some(row)) = rows.next().await {
            let emb_json: String = row.get(6).unwrap_or_default();
            let emb: Vec<f32> = serde_json::from_str(&emb_json).unwrap_or_default();
            let created_at: String = row.get(8).unwrap_or_default();
            let priority: String = row.get(5).unwrap_or_default();

            let sim = cosine_similarity(&query_emb, &emb);
            if sim < MIN_SIMILARITY { continue; }

            let score = sim * priority_weight(&priority) * recency_weight_from_iso(&created_at);

            let mem = StoredMemory {
                id: row.get(0).unwrap_or_default(),
                user_id: row.get(1).unwrap_or_default(),
                session_id: row.get(2).ok(),
                content: row.get(3).unwrap_or_default(),
                memory_type: row.get(4).unwrap_or_default(),
                priority,
                access_count: row.get(7).unwrap_or(0),
                created_at,
                updated_at: row.get(9).unwrap_or_default(),
            };

            // If session_id filter is set, prefer session-specific but include global
            if let Some(sid) = session_id {
                if mem.session_id.as_deref() != Some(sid) && mem.session_id.is_some() {
                    continue;
                }
            }

            scored.push((score, mem));
        }

        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

        // Apply token budget
        let mut results = Vec::new();
        let mut tokens_used = 0usize;
        for (score, mem) in scored.into_iter().take(limit * 3) {
            let est_tokens = mem.content.len() / 4;
            if tokens_used + est_tokens > max_tokens { continue; }
            tokens_used += est_tokens;
            results.push(RetrievedMemory { score, memory: mem });
            if results.len() >= limit { break; }
        }

        // Update access counts (fire and forget)
        if !results.is_empty() {
            let ids: Vec<String> = results.iter().map(|r| format!("'{}'", r.memory.id)).collect();
            let sql = format!(
                "UPDATE memories SET access_count = access_count + 1, last_accessed = '{}' WHERE id IN ({})",
                Utc::now().to_rfc3339(), ids.join(",")
            );
            if let Err(e) = self.conn.execute(&sql, ()).await {
                warn!("RAG access_count update failed: {}", e);
            }
        }

        Ok(results)
    }

    /// Build `<memory>...</memory>` system context block
    pub async fn build_context(&self, query: &str, session_id: Option<&str>, max_tokens: usize) -> Result<RagBuildResult> {
        let memories = self.retrieve(query, DEFAULT_LIMIT, max_tokens, session_id).await?;

        if memories.is_empty() {
            return Ok(RagBuildResult {
                system_context: String::new(),
                memories_used: 0,
                token_count: 0,
                sources: Vec::new(),
            });
        }

        let mut lines = vec!["<memory>".to_string()];
        let mut token_count = 0usize;
        let mut sources = Vec::new();

        for m in &memories {
            let label = m.memory.memory_type.to_uppercase();
            let line = format!("[{}] {}", label, m.memory.content);
            token_count += line.len() / 4;
            lines.push(line);
            sources.push(MemorySource {
                id: m.memory.id.clone(),
                memory_type: m.memory.memory_type.clone(),
                score: m.score,
            });
        }
        lines.push("</memory>".to_string());

        Ok(RagBuildResult {
            system_context: lines.join("\n"),
            memories_used: memories.len(),
            token_count,
            sources,
        })
    }

    /// Auto-extract + store memories from a conversation turn
    pub async fn auto_store(&self, user_msg: &str, assistant_msg: &str, session_id: &str) -> Vec<String> {
        let extracted = auto_extract(user_msg, assistant_msg);
        let mut ids = Vec::new();
        for (content, mtype, priority) in extracted {
            match self.store(&content, mtype, priority, Some(session_id)).await {
                Ok(id) => ids.push(id),
                Err(e) => warn!("RAG auto_store failed: {}", e),
            }
        }
        ids
    }

    /// Delete a memory (only if it belongs to this user)
    pub async fn forget(&self, memory_id: &str) -> Result<bool> {
        let affected = self.conn.execute(
            "DELETE FROM memories WHERE id=?1 AND user_id=?2",
            libsql::params![memory_id.to_string(), self.user_id.clone()],
        ).await?;
        Ok(affected > 0)
    }

    /// Clear all memories for this user (optionally scoped to session)
    pub async fn clear(&self, session_id: Option<&str>) -> Result<u64> {
        let affected = if let Some(sid) = session_id {
            self.conn.execute(
                "DELETE FROM memories WHERE user_id=?1 AND session_id=?2",
                libsql::params![self.user_id.clone(), sid.to_string()],
            ).await?
        } else {
            self.conn.execute(
                "DELETE FROM memories WHERE user_id=?1",
                libsql::params![self.user_id.clone()],
            ).await?
        };
        Ok(affected)
    }

    /// Stats for this user
    pub async fn stats(&self) -> Result<RagStats> {
        let mut rows = self.conn.query(
            "SELECT memory_type, priority, COUNT(*) FROM memories WHERE user_id=?1 GROUP BY memory_type, priority",
            libsql::params![self.user_id.clone()],
        ).await?;

        let mut by_type = std::collections::HashMap::new();
        let mut by_priority = std::collections::HashMap::new();
        let mut total = 0i64;

        while let Ok(Some(row)) = rows.next().await {
            let t: String = row.get(0).unwrap_or_default();
            let p: String = row.get(1).unwrap_or_default();
            let c: i64 = row.get(2).unwrap_or(0);
            *by_type.entry(t).or_insert(0i64) += c;
            *by_priority.entry(p).or_insert(0i64) += c;
            total += c;
        }

        Ok(RagStats { total, by_type, by_priority })
    }

    /// List memories (paginated)
    pub async fn list(&self, limit: usize, offset: usize, session_id: Option<&str>, memory_type: Option<&str>) -> Result<Vec<StoredMemory>> {
        let mut memories = Vec::new();

        let (sql, _params_str): (String, Option<String>) = match (session_id, memory_type) {
            (Some(s), Some(t)) => (
                format!("SELECT id,user_id,session_id,content,memory_type,priority,access_count,created_at,updated_at FROM memories WHERE user_id=?1 AND session_id='{}' AND memory_type='{}' ORDER BY created_at DESC LIMIT ?2 OFFSET ?3", s, t),
                None,
            ),
            (Some(s), None) => (
                format!("SELECT id,user_id,session_id,content,memory_type,priority,access_count,created_at,updated_at FROM memories WHERE user_id=?1 AND session_id='{}' ORDER BY created_at DESC LIMIT ?2 OFFSET ?3", s),
                None,
            ),
            (None, Some(t)) => (
                format!("SELECT id,user_id,session_id,content,memory_type,priority,access_count,created_at,updated_at FROM memories WHERE user_id=?1 AND memory_type='{}' ORDER BY created_at DESC LIMIT ?2 OFFSET ?3", t),
                None,
            ),
            (None, None) => (
                "SELECT id,user_id,session_id,content,memory_type,priority,access_count,created_at,updated_at FROM memories WHERE user_id=?1 ORDER BY created_at DESC LIMIT ?2 OFFSET ?3".to_string(),
                None,
            ),
        };

        let mut rows = self.conn.query(
            &sql,
            libsql::params![self.user_id.clone(), limit as i64, offset as i64],
        ).await?;

        while let Ok(Some(row)) = rows.next().await {
            memories.push(StoredMemory {
                id: row.get(0).unwrap_or_default(),
                user_id: row.get(1).unwrap_or_default(),
                session_id: row.get(2).ok(),
                content: row.get(3).unwrap_or_default(),
                memory_type: row.get(4).unwrap_or_default(),
                priority: row.get(5).unwrap_or_default(),
                access_count: row.get(6).unwrap_or(0),
                created_at: row.get(7).unwrap_or_default(),
                updated_at: row.get(8).unwrap_or_default(),
            });
        }

        Ok(memories)
    }
}

// Keep old type aliases so existing code compiles
pub use super::context_window::ContextManagerStats;
use super::{MemoryEntry, MemoryType, MemoryPriority, RetrievalContext};

/// Legacy RagEngine wrapper — keeps routes/rag.rs compiling during migration
pub struct LegacyRagEngine;
impl LegacyRagEngine {
    pub fn new(_dim: usize) -> Self { Self }
}

#[derive(Debug, thiserror::Error)]
pub enum RagError {
    #[error("Storage error: {0}")] StorageError(String),
    #[error("Search error: {0}")] SearchError(String),
    #[error("Embedding error: {0}")] EmbeddingError(String),
    #[error("Context error: {0}")] ContextError(String),
    #[error("DB error: {0}")] DbError(String),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RagEngineStats {
    pub total_vectors: usize,
    pub embedding_dimension: usize,
    pub context_window: ContextManagerStats,
}
