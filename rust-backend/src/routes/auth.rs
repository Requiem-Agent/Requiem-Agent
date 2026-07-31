use axum::{extract::State, http::StatusCode, Json};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;
use uuid::Uuid;

use crate::{auth, routes::AuthUserExt, AppState};

#[derive(Deserialize)]
pub struct LoginBody {
    pub username: String,
    pub password: String,
}

#[derive(Serialize)]
pub struct LoginResponse {
    pub status: String,
    pub token: String,
    pub user_id: String,
    pub client_id: String,
    pub username: String,
    pub plan: String,
}

pub async fn login_handler(
    State(state): State<Arc<AppState>>,
    Json(body): Json<LoginBody>,
) -> Result<Json<LoginResponse>, (StatusCode, Json<Value>)> {
    // جلب الحساب من قاعدة PopCorn الموحدة (مصدر الحقيقة الوحيد)
    let mut result = state.popcorn_conn.query(
        "SELECT client_id, username, password_hash FROM customer_accounts WHERE username = ?1 LIMIT 1",
        libsql::params![body.username.clone()],
    ).await.map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": e.to_string() })))
    })?;

    let row = result.next().await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
    })?;

    let (client_id, username, stored_hash) = match row {
        Some(r) => {
            let cid: String = r.get(0).unwrap_or_default();
            let uname: String = r.get(1).unwrap_or_default();
            let hash: String = r.get(2).unwrap_or_default();
            (cid, uname, hash)
        }
        None => {
            return Err((
                StatusCode::UNAUTHORIZED,
                Json(json!({ "error": "Invalid username or password" })),
            ))
        }
    };

    // التحقق من كلمة المرور (Argon2id + legacy sha256 popcorn:{}:salt)
    if !auth::verify_password(&body.password, &stored_hash) {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(json!({ "error": "Invalid username or password" })),
        ));
    }

    // قراءة الخطة من جدول clients في PopCorn (إن فشل الاستعلام فـ 'free')
    let plan = match state
        .popcorn_conn
        .query(
            "SELECT plan FROM clients WHERE id = ?1 LIMIT 1",
            libsql::params![client_id.clone()],
        )
        .await
    {
        Ok(mut rows) => match rows.next().await {
            Ok(Some(r)) => r.get::<String>(0).unwrap_or_else(|_| "free".to_string()),
            _ => "free".to_string(),
        },
        Err(_) => "free".to_string(),
    };

    // إنشاء أو تحديث المستخدم في قاعدة Requiem المحلية
    let user_id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();
    let reset_at = (Utc::now() + chrono::Duration::days(30)).to_rfc3339();

    // التحقق من وجود المستخدم مسبقاً
    let existing = state
        .conn
        .query(
            "SELECT id FROM users WHERE popcorn_client_id = ?1 LIMIT 1",
            libsql::params![client_id.clone()],
        )
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": e.to_string() })),
            )
        })?
        .next()
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": e.to_string() })),
            )
        })?;

    let final_user_id = if let Some(row) = existing {
        let existing_id: String = row.get(0).unwrap_or_default();
        state
            .conn
            .execute(
                "UPDATE users SET username = ?1, plan = ?2 WHERE id = ?3",
                libsql::params![username, plan.clone(), existing_id.clone()],
            )
            .await
            .map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({ "error": e.to_string() })),
                )
            })?;
        existing_id
    } else {
        state.conn.execute(
            "INSERT INTO users (id, popcorn_client_id, username, plan, quota_read_used, quota_write_used, quota_reset_at, created_at) \
             VALUES (?1, ?2, ?3, ?4, 0, 0, ?5, ?6)",
            libsql::params![user_id.clone(), client_id.clone(), username, plan.clone(), reset_at, now],
        ).await.map_err(|e| {
            (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": e.to_string() })))
        })?;
        user_id
    };

    // إنشاء توكن
    let token = auth::generate_token(&final_user_id, &state.session_secret);

    Ok(Json(LoginResponse {
        status: "ok".to_string(),
        token,
        user_id: final_user_id,
        client_id,
        username: body.username,
        plan,
    }))
}

/// GET /auth/me — جلب بيانات المستخدم الحالي من قاعدة Requiem المحلية
pub async fn me_handler(
    State(state): State<Arc<AppState>>,
    axum::Extension(AuthUserExt(auth_user)): axum::Extension<AuthUserExt>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let mut rows = state
        .conn
        .query(
            "SELECT popcorn_client_id, username, plan FROM users WHERE id = ?1 LIMIT 1",
            libsql::params![auth_user.user_id.clone()],
        )
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": e.to_string() })),
            )
        })?;

    match rows.next().await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
    })? {
        Some(r) => Ok(Json(json!({
            "user_id": auth_user.user_id,
            "client_id": r.get::<String>(0).unwrap_or_default(),
            "username": r.get::<String>(1).unwrap_or_default(),
            "plan": r.get::<String>(2).unwrap_or_else(|_| "free".to_string()),
        }))),
        None => Err((
            StatusCode::UNAUTHORIZED,
            Json(json!({ "error": "User not found" })),
        )),
    }
}
