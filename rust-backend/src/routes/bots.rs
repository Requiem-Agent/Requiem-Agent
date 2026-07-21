use axum::{extract::{Path, State}, http::StatusCode, Extension, Json};
use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::Arc;
use uuid::Uuid;
use chrono::Utc;

use crate::AppState;
use super::UserId;

fn bot_row(r: &libsql::Row) -> Value {
    json!({
        "id": r.get::<String>(0).unwrap_or_default(),
        "name": r.get::<String>(1).unwrap_or_default(),
        "username": r.get::<String>(2).unwrap_or_default(),
        "status": r.get::<String>(3).unwrap_or_default(),
        "hfSpaceUrl": r.get::<Option<String>>(4).unwrap_or(None),
        "deployedAt": r.get::<Option<String>>(5).unwrap_or(None),
        "createdAt": r.get::<String>(6).unwrap_or_default(),
    })
}

pub async fn list_bots(
    State(state): State<Arc<AppState>>,
    Extension(UserId(user_id)): Extension<UserId>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let mut rows = state.conn.query(
        "SELECT id, name, username, status, hf_space_url, deployed_at, created_at FROM bots WHERE user_id = ?1 ORDER BY created_at DESC",
        [user_id],
    ).await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": e.to_string() }))))?;

    let mut bots = Vec::new();
    while let Ok(Some(r)) = rows.next().await {
        bots.push(bot_row(&r));
    }
    Ok(Json(json!(bots)))
}

#[derive(Deserialize)]
pub struct CreateBotBody {
    pub name: String,
    pub username: String,
    pub description: Option<String>,
}

pub async fn create_bot(
    State(state): State<Arc<AppState>>,
    Extension(UserId(user_id)): Extension<UserId>,
    Json(body): Json<CreateBotBody>,
) -> Result<(StatusCode, Json<Value>), (StatusCode, Json<Value>)> {
    if body.name.trim().is_empty() || body.username.trim().is_empty() {
        return Err((StatusCode::BAD_REQUEST, Json(json!({ "error": "name and username are required" }))));
    }
    let clean_username = body.username.trim_start_matches('@').to_string();
    let id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();

    state.conn.execute(
        "INSERT INTO bots (id, user_id, name, username, description, status, created_at) VALUES (?1, ?2, ?3, ?4, ?5, 'pending', ?6)",
        libsql::params![id.clone(), user_id, body.name.trim().to_string(), clean_username, body.description.clone(), now],
    ).await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": e.to_string() }))))?;

    let mut rows = state.conn.query(
        "SELECT id, name, username, status, hf_space_url, deployed_at, created_at FROM bots WHERE id = ?1",
        [id],
    ).await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": e.to_string() }))))?;

    let row = rows.next().await.ok().flatten()
        .ok_or_else(|| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": "Failed to fetch created bot" }))))?;

    Ok((StatusCode::CREATED, Json(bot_row(&row))))
}

pub async fn get_bot(
    State(state): State<Arc<AppState>>,
    Extension(UserId(user_id)): Extension<UserId>,
    Path(id): Path<String>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let mut rows = state.conn.query(
        "SELECT id, name, username, status, hf_space_url, deployed_at, created_at FROM bots WHERE id = ?1 AND user_id = ?2",
        libsql::params![id, user_id],
    ).await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": e.to_string() }))))?;

    let row = rows.next().await.ok().flatten()
        .ok_or_else(|| (StatusCode::NOT_FOUND, Json(json!({ "error": "Bot not found" }))))?;

    Ok(Json(bot_row(&row)))
}

pub async fn delete_bot(
    State(state): State<Arc<AppState>>,
    Extension(UserId(user_id)): Extension<UserId>,
    Path(id): Path<String>,
) -> Result<StatusCode, (StatusCode, Json<Value>)> {
    let mut rows = state.conn.query(
        "SELECT id FROM bots WHERE id = ?1 AND user_id = ?2",
        libsql::params![id.clone(), user_id],
    ).await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": e.to_string() }))))?;

    if rows.next().await.ok().flatten().is_none() {
        return Err((StatusCode::NOT_FOUND, Json(json!({ "error": "Bot not found" }))));
    }
    state.conn.execute("DELETE FROM bots WHERE id = ?1", [id]).await.ok();
    Ok(StatusCode::NO_CONTENT)
}

pub async fn deploy_bot(
    State(state): State<Arc<AppState>>,
    Extension(UserId(user_id)): Extension<UserId>,
    Path(id): Path<String>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let mut rows = state.conn.query(
        "SELECT id, username FROM bots WHERE id = ?1 AND user_id = ?2",
        libsql::params![id.clone(), user_id],
    ).await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": e.to_string() }))))?;

    let row = rows.next().await.ok().flatten()
        .ok_or_else(|| (StatusCode::NOT_FOUND, Json(json!({ "error": "Bot not found" }))))?;

    let username: String = row.get(1).unwrap_or_default();
    state.conn.execute("UPDATE bots SET status = 'building' WHERE id = ?1", [id.clone()]).await.ok();

    let hf_org = state.hf_space_prdcn.split('/').next().unwrap_or("rayig").to_string();
    let hf_space_url = format!("https://huggingface.co/spaces/{hf_org}/{username}");
    let client = reqwest::Client::new();
    let resp = client
        .post("https://huggingface.co/api/spaces")
        .bearer_auth(&state.hf_token)
        .json(&json!({ "name": username, "organization": hf_org, "sdk": "docker", "private": false }))
        .timeout(std::time::Duration::from_secs(15))
        .send()
        .await;

    let deployed = matches!(resp, Ok(r) if r.status().is_success() || r.status().as_u16() == 409);
    let now = Utc::now().to_rfc3339();

    state.conn.execute(
        "UPDATE bots SET status = ?1, hf_space_url = ?2, deployed_at = ?3 WHERE id = ?4",
        libsql::params![
            if deployed { "deployed" } else { "error" },
            hf_space_url.clone(),
            if deployed { Some(now) } else { None },
            id,
        ],
    ).await.ok();

    Ok(Json(json!({
        "success": deployed,
        "message": if deployed { format!("Bot @{username} deployed successfully") } else { "Deployment queued".to_string() },
        "hfSpaceUrl": hf_space_url,
    })))
}
