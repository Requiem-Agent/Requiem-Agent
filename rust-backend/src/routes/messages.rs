use axum::{extract::{Path, State}, http::StatusCode, Extension, Json};
use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::Arc;
use uuid::Uuid;
use chrono::Utc;

use crate::AppState;
use super::UserId;

pub async fn list_messages(
    State(state): State<Arc<AppState>>,
    Extension(UserId(user_id)): Extension<UserId>,
    Path(id): Path<String>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let mut check = state.conn.query(
        "SELECT id FROM sessions WHERE id = ?1 AND user_id = ?2",
        libsql::params![id.clone(), user_id],
    ).await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": e.to_string() }))))?;

    if check.next().await.ok().flatten().is_none() {
        return Err((StatusCode::NOT_FOUND, Json(json!({ "error": "Session not found" }))));
    }

    let mut rows = state.conn.query(
        "SELECT id, session_id, role, content, model_used, mode, effort, tool_calls, code_changes, created_at FROM messages WHERE session_id = ?1 ORDER BY created_at ASC",
        [id],
    ).await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": e.to_string() }))))?;

    let mut messages = Vec::new();
    while let Ok(Some(r)) = rows.next().await {
        messages.push(json!({
            "id": r.get::<String>(0).unwrap_or_default(),
            "sessionId": r.get::<String>(1).unwrap_or_default(),
            "role": r.get::<String>(2).unwrap_or_default(),
            "content": r.get::<String>(3).unwrap_or_default(),
            "modelUsed": r.get::<Option<String>>(4).unwrap_or(None),
            "mode": r.get::<Option<String>>(5).unwrap_or(None),
            "effort": r.get::<Option<String>>(6).unwrap_or(None),
            "toolCalls": r.get::<Option<String>>(7).unwrap_or(None),
            "codeChanges": r.get::<Option<String>>(8).unwrap_or(None),
            "createdAt": r.get::<String>(9).unwrap_or_default(),
        }));
    }
    Ok(Json(json!(messages)))
}

#[derive(Deserialize)]
pub struct AddMessageBody {
    pub role: String,
    pub content: String,
    #[serde(rename = "modelUsed")]
    pub model_used: Option<String>,
    pub mode: Option<String>,
    pub effort: Option<String>,
    #[serde(rename = "toolCalls")]
    pub tool_calls: Option<String>,
    #[serde(rename = "codeChanges")]
    pub code_changes: Option<String>,
}

pub async fn add_message(
    State(state): State<Arc<AppState>>,
    Extension(UserId(user_id)): Extension<UserId>,
    Path(id): Path<String>,
    Json(body): Json<AddMessageBody>,
) -> Result<(StatusCode, Json<Value>), (StatusCode, Json<Value>)> {
    let mut check = state.conn.query(
        "SELECT id FROM sessions WHERE id = ?1 AND user_id = ?2",
        libsql::params![id.clone(), user_id.clone()],
    ).await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": e.to_string() }))))?;

    if check.next().await.ok().flatten().is_none() {
        return Err((StatusCode::NOT_FOUND, Json(json!({ "error": "Session not found" }))));
    }

    let msg_id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();

    state.conn.execute(
        "INSERT INTO messages (id, session_id, role, content, model_used, mode, effort, tool_calls, code_changes, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        libsql::params![msg_id.clone(), id.clone(), body.role.clone(), body.content.clone(), body.model_used.clone(), body.mode.clone(), body.effort.clone(), body.tool_calls.clone(), body.code_changes.clone(), now.clone()],
    ).await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": e.to_string() }))))?;

    state.conn.execute(
        "UPDATE sessions SET message_count = message_count + 1, last_message_at = ?1, updated_at = ?1 WHERE id = ?2",
        libsql::params![now.clone(), id.clone()],
    ).await.ok();

    // Track quota
    if body.role == "user" {
        state.conn.execute("UPDATE users SET quota_read_used = quota_read_used + 1 WHERE id = ?1", [user_id]).await.ok();
    } else if body.role == "assistant" {
        state.conn.execute("UPDATE users SET quota_write_used = quota_write_used + 1 WHERE id = ?1", [user_id]).await.ok();
    }

    Ok((StatusCode::CREATED, Json(json!({
        "id": msg_id,
        "sessionId": id,
        "role": body.role,
        "content": body.content,
        "modelUsed": body.model_used,
        "mode": body.mode,
        "effort": body.effort,
        "toolCalls": body.tool_calls,
        "codeChanges": body.code_changes,
        "createdAt": now,
    }))))
}
