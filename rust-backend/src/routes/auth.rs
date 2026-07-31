use axum::{extract::State, http::StatusCode, Json};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;
use uuid::Uuid;
use chrono::Utc;

use crate::{AppState, auth::{validate_telegram_init_data, generate_token}, storage};

#[derive(Deserialize)]
pub struct AuthBody {
    #[serde(rename = "initData")]
    pub init_data: String,
}

#[derive(Serialize)]
pub struct AuthResponse {
    pub user: Value,
    pub token: String,
}

pub async fn telegram_auth(
    State(state): State<Arc<AppState>>,
    Json(body): Json<AuthBody>,
) -> Result<Json<AuthResponse>, (StatusCode, Json<Value>)> {
    let parsed = validate_telegram_init_data(&body.init_data, &state.bot_token)
        .map_err(|e| (StatusCode::UNAUTHORIZED, Json(json!({ "error": e }))))?;

    let user_str = parsed.get("user").ok_or_else(|| {
        (StatusCode::UNAUTHORIZED, Json(json!({ "error": "Missing user in initData" })))
    })?;

    let tg_user: Value = serde_json::from_str(user_str)
        .map_err(|_| (StatusCode::UNAUTHORIZED, Json(json!({ "error": "Invalid user data" }))))?;

    let tg_id = tg_user["id"].as_i64().ok_or_else(|| {
        (StatusCode::UNAUTHORIZED, Json(json!({ "error": "Missing user id" })))
    })?;

    let first_name = tg_user["first_name"].as_str().unwrap_or("").to_string();
    let last_name = tg_user["last_name"].as_str().map(String::from);
    let username = tg_user["username"].as_str().map(String::from);

    let now = Utc::now().to_rfc3339();
    let reset_at = (Utc::now() + chrono::Duration::days(30)).to_rfc3339();

    // Check if user exists
    let existing = state.conn.query(
        "SELECT id FROM users WHERE telegram_id = ?1",
        [tg_id],
    ).await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": e.to_string() }))))?
    .next().await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": e.to_string() }))))?;

    let user_id = if let Some(row) = existing {
        let id: String = row.get(0).unwrap_or_default();
        state.conn.execute(
            "UPDATE users SET first_name = ?1, last_name = ?2, username = ?3 WHERE id = ?4",
            libsql::params![first_name.clone(), last_name.clone(), username.clone(), id.clone()],
        ).await.ok();
        id
    } else {
        let id = Uuid::new_v4().to_string();
        state.conn.execute(
            "INSERT INTO users (id, telegram_id, first_name, last_name, username, quota_read_used, quota_write_used, quota_reset_at, created_at) VALUES (?1, ?2, ?3, ?4, ?5, 0, 0, ?6, ?7)",
            libsql::params![id.clone(), tg_id, first_name.clone(), last_name.clone(), username.clone(), reset_at, now.clone()],
        ).await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": e.to_string() }))))?;
        id
    };

    let mut row = state.conn.query(
        "SELECT id, telegram_id, first_name, last_name, username, quota_read_used, quota_write_used, created_at FROM users WHERE id = ?1",
        [user_id.clone()],
    ).await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": e.to_string() }))))?;

    let user_row = row.next().await.ok().flatten();
    let user = if let Some(r) = user_row {
        json!({
            "id": r.get::<String>(0).unwrap_or_default(),
            "telegramId": r.get::<i64>(1).unwrap_or_default(),
            "firstName": r.get::<String>(2).unwrap_or_default(),
            "lastName": r.get::<Option<String>>(3).unwrap_or(None),
            "username": r.get::<Option<String>>(4).unwrap_or(None),
            "quotaReadUsed": r.get::<i64>(5).unwrap_or(0),
            "quotaWriteUsed": r.get::<i64>(6).unwrap_or(0),
            "createdAt": r.get::<String>(7).unwrap_or_default(),
        })
    } else {
        json!({ "id": user_id })
    };

    // Initialize user storage on first login
    storage::init_user_storage(&user_id).await.ok();

    let token = generate_token(&user_id, &state.session_secret);
    Ok(Json(AuthResponse { user, token }))
}
