pub mod agent_env;
pub mod agent_protocol;
pub mod anti_printer;
pub mod auth;
pub mod bots;
pub mod enforce;
pub mod files;
pub mod formats;
pub mod health;
pub mod messages;
pub mod models;
pub mod rag;
pub mod sandbox;
pub mod sessions;
pub mod synergy;
pub mod tasks;
pub mod tools;
pub mod usage;
pub mod user_questions;
pub mod zen;
// S4-03: WebSocket agent streaming
pub mod ws_agent;
// Sprint 2: WebSocket Terminal
pub mod ws_terminal;
// S5-02: User preferences CRUD API
pub mod preferences;
// S6-02: User API keys (encrypted LLM provider keys)
pub mod agent_chat;
pub mod identity_shield;
pub mod prdcn;
pub mod strict_locks;
pub mod user_api_keys;
pub mod workspaces;

use crate::auth::{verify_token, AuthUser};
use crate::AppState;
use axum::{
    extract::State,
    http::{header::AUTHORIZATION, Request, StatusCode},
    middleware::Next,
    response::Response,
};
use std::sync::Arc;
use tracing::{debug, warn};

/// Extension Key: المستخدم المُوثَّق
#[derive(Debug, Clone)]
pub struct AuthUserExt(pub AuthUser);

/// ⚠️ Legacy — للتوافق مع الكود القديم
#[derive(Clone)]
pub struct UserId(pub String);

/// Middleware للمصادقة عبر Popcorn tokens
pub async fn auth_middleware(
    State(state): State<Arc<AppState>>,
    mut req: Request<axum::body::Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    let token = req
        .headers()
        .get(AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or_else(|| {
            warn!("Missing Authorization header");
            StatusCode::UNAUTHORIZED
        })?;

    let token_str = token.to_string();

    // التحقق من التوكن
    let user_id = verify_token(&token_str, &state.session_secret).ok_or_else(|| {
        warn!("Token verification failed");
        StatusCode::UNAUTHORIZED
    })?;

    debug!("Auth OK: user={}", user_id);

    // تعبئة بيانات المستخدم من قاعدة Requiem المحلية
    let mut auth_user = AuthUser {
        user_id: user_id.clone(),
        client_id: String::new(),
        username: String::new(),
        plan: String::new(),
    };

    if let Ok(mut rows) = state
        .conn
        .query(
            "SELECT popcorn_client_id, username, plan FROM users WHERE id = ?1 LIMIT 1",
            libsql::params![user_id.clone()],
        )
        .await
    {
        if let Ok(Some(row)) = rows.next().await {
            auth_user.client_id = row.get::<String>(0).unwrap_or_default();
            auth_user.username = row.get::<String>(1).unwrap_or_default();
            auth_user.plan = row.get::<String>(2).unwrap_or_default();
        }
    }

    req.extensions_mut().insert(AuthUserExt(auth_user.clone()));
    req.extensions_mut().insert(UserId(user_id.clone()));
    // Legacy compat: الـ handlers القديمة تطلب Extension<AuthUser> مباشرة
    req.extensions_mut().insert(auth_user);
    Ok(next.run(req).await)
}
