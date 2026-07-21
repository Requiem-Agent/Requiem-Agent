//! # RAG Routes — مسارات API لنظام الذاكرة المستمر
//!
//! كل endpoint يُنشئ RagEngine مرتبط بـ user_id من التوكن.
//! العزل الكامل: WHERE user_id=? في كل استعلام.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
    routing::{get, post},
    Router,
    Extension,
};
use serde::Deserialize;
use std::sync::Arc;
use serde_json::{json, Value};

use crate::agent::memory::rag::RagEngine;
use crate::db::AppState;
use crate::routes::AuthUser;

// ── Request/Response types ─────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct StoreMemoryRequest {
    pub content: String,
    #[serde(default = "default_memory_type")]
    pub memory_type: String,
    #[serde(default = "default_priority")]
    pub priority: String,
    pub session_id: Option<String>,
}
fn default_memory_type() -> String { "context".to_string() }
fn default_priority()     -> String { "medium".to_string() }

#[derive(Deserialize)]
pub struct SearchRequest {
    pub query: String,
    pub limit: Option<usize>,
    pub session_id: Option<String>,
}

#[derive(Deserialize)]
pub struct InjectContextRequest {
    pub query: String,
    pub session_id: Option<String>,
    pub max_context_tokens: Option<usize>,
}

#[derive(Deserialize)]
pub struct AutoStoreRequest {
    pub user_message: String,
    pub assistant_response: String,
    pub session_id: String,
}

#[derive(Deserialize)]
pub struct ListMemoriesQuery {
    pub limit: Option<usize>,
    pub offset: Option<usize>,
    pub session_id: Option<String>,
    #[serde(rename = "type")]
    pub memory_type: Option<String>,
}

#[derive(Deserialize)]
pub struct ClearRequest {
    pub session_id: Option<String>,
}

/// إنشاء مسارات RAG
pub fn rag_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/rag/store",          post(store_memory))
        .route("/rag/search",         post(search_memories))
        .route("/rag/inject-context", post(inject_context))
        .route("/rag/auto-store",     post(auto_store))
        .route("/rag/memories",       get(list_memories))
        .route("/rag/memory/{id}",    get(get_memory).delete(delete_memory))
        .route("/rag/stats",          get(get_stats))
        .route("/rag/clear",          post(clear_memory))
}

// ── Handlers ───────────────────────────────────────────────────────────────

/// POST /rag/store
pub async fn store_memory(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
    Json(req): Json<StoreMemoryRequest>,
) -> Result<Json<Value>, StatusCode> {
    if req.content.trim().is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }
    let rag = RagEngine::new(state.conn.clone(), &auth.user_id);
    match rag.store(&req.content, &req.memory_type, &req.priority, req.session_id.as_deref()).await {
        Ok(id) => Ok(Json(json!({ "id": id, "stored": true }))),
        Err(e) => { tracing::error!("rag store: {}", e); Err(StatusCode::INTERNAL_SERVER_ERROR) }
    }
}

/// POST /rag/search
pub async fn search_memories(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
    Json(req): Json<SearchRequest>,
) -> Result<Json<Value>, StatusCode> {
    if req.query.trim().is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }
    let rag = RagEngine::new(state.conn.clone(), &auth.user_id);
    let limit = req.limit.unwrap_or(10).min(50);
    match rag.retrieve(&req.query, limit, 4000, req.session_id.as_deref()).await {
        Ok(memories) => Ok(Json(json!({ "memories": memories, "count": memories.len() }))),
        Err(e) => { tracing::error!("rag search: {}", e); Err(StatusCode::INTERNAL_SERVER_ERROR) }
    }
}

/// POST /rag/inject-context  — builds system context block for chat injection
pub async fn inject_context(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
    Json(req): Json<InjectContextRequest>,
) -> Result<Json<Value>, StatusCode> {
    if req.query.trim().is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }
    let rag = RagEngine::new(state.conn.clone(), &auth.user_id);
    let max_tokens = req.max_context_tokens.unwrap_or(2000).min(4000);
    match rag.build_context(&req.query, req.session_id.as_deref(), max_tokens).await {
        Ok(result) => Ok(Json(json!({
            "systemContext": result.system_context,
            "memoriesUsed": result.memories_used,
            "tokenCount": result.token_count,
            "sources": result.sources,
        }))),
        Err(e) => { tracing::error!("rag inject-context: {}", e); Err(StatusCode::INTERNAL_SERVER_ERROR) }
    }
}

/// POST /rag/auto-store
pub async fn auto_store(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
    Json(req): Json<AutoStoreRequest>,
) -> Result<Json<Value>, StatusCode> {
    let rag = RagEngine::new(state.conn.clone(), &auth.user_id);
    let ids = rag.auto_store(&req.user_message, &req.assistant_response, &req.session_id).await;
    Ok(Json(json!({ "stored": ids.len(), "ids": ids })))
}

/// GET /rag/memories
pub async fn list_memories(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
    Query(q): Query<ListMemoriesQuery>,
) -> Result<Json<Value>, StatusCode> {
    let rag = RagEngine::new(state.conn.clone(), &auth.user_id);
    let limit = q.limit.unwrap_or(20).min(100);
    let offset = q.offset.unwrap_or(0);
    match rag.list(limit, offset, q.session_id.as_deref(), q.memory_type.as_deref()).await {
        Ok(memories) => Ok(Json(json!({ "memories": memories, "count": memories.len() }))),
        Err(e) => { tracing::error!("rag list: {}", e); Err(StatusCode::INTERNAL_SERVER_ERROR) }
    }
}

/// GET /rag/memory/:id
pub async fn get_memory(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
    Path(id): Path<String>,
) -> Result<Json<Value>, StatusCode> {
    let rag = RagEngine::new(state.conn.clone(), &auth.user_id);
    // Use list with a dummy query to get by id
    let mut rows = state.conn.query(
        "SELECT id,user_id,session_id,content,memory_type,priority,access_count,created_at,updated_at FROM memories WHERE id=?1 AND user_id=?2",
        libsql::params![id, auth.user_id.clone()],
    ).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if let Ok(Some(row)) = rows.next().await {
        Ok(Json(json!({
            "id": row.get::<String>(0).unwrap_or_default(),
            "userId": row.get::<String>(1).unwrap_or_default(),
            "sessionId": row.get::<Option<String>>(2).ok().flatten(),
            "content": row.get::<String>(3).unwrap_or_default(),
            "memoryType": row.get::<String>(4).unwrap_or_default(),
            "priority": row.get::<String>(5).unwrap_or_default(),
            "accessCount": row.get::<i64>(6).unwrap_or(0),
            "createdAt": row.get::<String>(7).unwrap_or_default(),
            "updatedAt": row.get::<String>(8).unwrap_or_default(),
        })))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

/// DELETE /rag/memory/:id
pub async fn delete_memory(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
    Path(id): Path<String>,
) -> Result<Json<Value>, StatusCode> {
    let rag = RagEngine::new(state.conn.clone(), &auth.user_id);
    match rag.forget(&id).await {
        Ok(true)  => Ok(Json(json!({ "deleted": true }))),
        Ok(false) => Err(StatusCode::NOT_FOUND),
        Err(e)    => { tracing::error!("rag forget: {}", e); Err(StatusCode::INTERNAL_SERVER_ERROR) }
    }
}

/// GET /rag/stats
pub async fn get_stats(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
) -> Result<Json<Value>, StatusCode> {
    let rag = RagEngine::new(state.conn.clone(), &auth.user_id);
    match rag.stats().await {
        Ok(stats) => Ok(Json(json!(stats))),
        Err(e)    => { tracing::error!("rag stats: {}", e); Err(StatusCode::INTERNAL_SERVER_ERROR) }
    }
}

/// POST /rag/clear
pub async fn clear_memory(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
    Json(req): Json<ClearRequest>,
) -> Result<Json<Value>, StatusCode> {
    let rag = RagEngine::new(state.conn.clone(), &auth.user_id);
    match rag.clear(req.session_id.as_deref()).await {
        Ok(deleted) => Ok(Json(json!({ "deleted": deleted }))),
        Err(e)      => { tracing::error!("rag clear: {}", e); Err(StatusCode::INTERNAL_SERVER_ERROR) }
    }
}
