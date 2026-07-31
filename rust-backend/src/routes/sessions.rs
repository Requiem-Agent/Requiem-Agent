use axum::{extract::{Path, State}, http::StatusCode, Extension, Json};
use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::Arc;
use uuid::Uuid;
use chrono::Utc;

use crate::{AppState, storage};
use super::UserId;

fn session_row_to_json(r: &libsql::Row) -> Value {
    json!({
        "id": r.get::<String>(0).unwrap_or_default(),
        "name": r.get::<String>(1).unwrap_or_default(),
        "mode": r.get::<String>(2).unwrap_or_default(),
        "effort": r.get::<String>(3).unwrap_or_default(),
        "activeModel": r.get::<Option<String>>(4).unwrap_or(None),
        "messageCount": r.get::<i64>(5).unwrap_or(0),
        "lastMessageAt": r.get::<Option<String>>(6).unwrap_or(None),
        "createdAt": r.get::<String>(7).unwrap_or_default(),
        "updatedAt": r.get::<String>(8).unwrap_or_default(),
    })
}

pub async fn list_sessions(
    State(state): State<Arc<AppState>>,
    Extension(UserId(user_id)): Extension<UserId>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let mut rows = state.conn.query(
        "SELECT id, name, mode, effort, active_model, message_count, last_message_at, created_at, updated_at FROM sessions WHERE user_id = ?1 ORDER BY updated_at DESC",
        [user_id],
    ).await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": e.to_string() }))))?;

    let mut sessions = Vec::new();
    while let Ok(Some(r)) = rows.next().await {
        sessions.push(session_row_to_json(&r));
    }
    Ok(Json(json!(sessions)))
}

#[derive(Deserialize)]
pub struct CreateSessionBody {
    pub name: String,
    pub mode: Option<String>,
    pub effort: Option<String>,
}

pub async fn create_session(
    State(state): State<Arc<AppState>>,
    Extension(UserId(user_id)): Extension<UserId>,
    Json(body): Json<CreateSessionBody>,
) -> Result<(StatusCode, Json<Value>), (StatusCode, Json<Value>)> {
    if body.name.trim().is_empty() {
        return Err((StatusCode::BAD_REQUEST, Json(json!({ "error": "name is required" }))));
    }

    // Check count and delete oldest if >= 3
    let mut count_rows = state.conn.query(
        "SELECT COUNT(*) FROM sessions WHERE user_id = ?1",
        [user_id.clone()],
    ).await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": e.to_string() }))))?;

    if let Ok(Some(r)) = count_rows.next().await {
        let count: i64 = r.get(0).unwrap_or(0);
        if count >= 3 {
            state.conn.execute(
                "DELETE FROM sessions WHERE user_id = ?1 AND id = (SELECT id FROM sessions WHERE user_id = ?1 ORDER BY updated_at ASC LIMIT 1)",
                [user_id.clone()],
            ).await.ok();
        }
    }

    let id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();
    let mode = body.mode.unwrap_or_else(|| "orchestrator".to_string());
    let effort = body.effort.unwrap_or_else(|| "medium".to_string());

    state.conn.execute(
        "INSERT INTO sessions (id, user_id, name, mode, effort, message_count, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, 0, ?6, ?7)",
        libsql::params![id.clone(), user_id.clone(), body.name.trim().to_string(), mode, effort, now.clone(), now.clone()],
    ).await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": e.to_string() }))))?;

    // Initialize per-session SQLite database
    storage::init_session_storage(&user_id, &id).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": e.to_string() }))))?;

    let mut rows = state.conn.query(
        "SELECT id, name, mode, effort, active_model, message_count, last_message_at, created_at, updated_at FROM sessions WHERE id = ?1",
        [id],
    ).await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": e.to_string() }))))?;

    let row = rows.next().await.ok().flatten()
        .ok_or_else(|| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": "Failed to fetch created session" }))))?;

    Ok((StatusCode::CREATED, Json(session_row_to_json(&row))))
}

pub async fn get_session(
    State(state): State<Arc<AppState>>,
    Extension(UserId(user_id)): Extension<UserId>,
    Path(id): Path<String>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let mut rows = state.conn.query(
        "SELECT id, name, mode, effort, active_model, message_count, last_message_at, created_at, updated_at FROM sessions WHERE id = ?1 AND user_id = ?2",
        libsql::params![id.clone(), user_id],
    ).await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": e.to_string() }))))?;

    let row = rows.next().await.ok().flatten()
        .ok_or_else(|| (StatusCode::NOT_FOUND, Json(json!({ "error": "Session not found" }))))?;

    let mut session = session_row_to_json(&row);

    let mut msg_rows = state.conn.query(
        "SELECT id, session_id, role, content, model_used, mode, effort, tool_calls, code_changes, created_at FROM messages WHERE session_id = ?1 ORDER BY created_at ASC",
        [id],
    ).await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": e.to_string() }))))?;

    let mut messages = Vec::new();
    while let Ok(Some(r)) = msg_rows.next().await {
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
    session["messages"] = json!(messages);
    Ok(Json(session))
}

#[derive(Deserialize)]
pub struct UpdateSessionBody {
    pub name: Option<String>,
    pub mode: Option<String>,
    pub effort: Option<String>,
    #[serde(rename = "activeModel")]
    pub active_model: Option<Option<String>>,
}

pub async fn update_session(
    State(state): State<Arc<AppState>>,
    Extension(UserId(user_id)): Extension<UserId>,
    Path(id): Path<String>,
    Json(body): Json<UpdateSessionBody>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let mut rows = state.conn.query(
        "SELECT id, name, mode, effort, active_model FROM sessions WHERE id = ?1 AND user_id = ?2",
        libsql::params![id.clone(), user_id],
    ).await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": e.to_string() }))))?;

    let row = rows.next().await.ok().flatten()
        .ok_or_else(|| (StatusCode::NOT_FOUND, Json(json!({ "error": "Session not found" }))))?;

    let cur_name: String = row.get(1).unwrap_or_default();
    let cur_mode: String = row.get(2).unwrap_or_default();
    let cur_effort: String = row.get(3).unwrap_or_default();
    let cur_model: Option<String> = row.get(4).unwrap_or(None);

    let now = Utc::now().to_rfc3339();
    state.conn.execute(
        "UPDATE sessions SET name = ?1, mode = ?2, effort = ?3, active_model = ?4, updated_at = ?5 WHERE id = ?6",
        libsql::params![
            body.name.unwrap_or(cur_name),
            body.mode.unwrap_or(cur_mode),
            body.effort.unwrap_or(cur_effort),
            body.active_model.unwrap_or(cur_model),
            now,
            id.clone(),
        ],
    ).await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": e.to_string() }))))?;

    let mut updated = state.conn.query(
        "SELECT id, name, mode, effort, active_model, message_count, last_message_at, created_at, updated_at FROM sessions WHERE id = ?1",
        [id],
    ).await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": e.to_string() }))))?;

    let r = updated.next().await.ok().flatten()
        .ok_or_else(|| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": "Failed to fetch updated session" }))))?;

    Ok(Json(session_row_to_json(&r)))
}

pub async fn delete_session(
    State(state): State<Arc<AppState>>,
    Extension(UserId(user_id)): Extension<UserId>,
    Path(id): Path<String>,
) -> Result<StatusCode, (StatusCode, Json<Value>)> {
    let mut rows = state.conn.query(
        "SELECT id FROM sessions WHERE id = ?1 AND user_id = ?2",
        libsql::params![id.clone(), user_id],
    ).await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": e.to_string() }))))?;

    if rows.next().await.ok().flatten().is_none() {
        return Err((StatusCode::NOT_FOUND, Json(json!({ "error": "Session not found" }))));
    }

    state.conn.execute("DELETE FROM messages WHERE session_id = ?1", [id.clone()]).await.ok();
    state.conn.execute("DELETE FROM sessions WHERE id = ?1", [id]).await.ok();
    Ok(StatusCode::NO_CONTENT)
}
