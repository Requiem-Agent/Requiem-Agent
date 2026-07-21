pub mod auth;
pub mod bots;
pub mod health;
pub mod messages;
pub mod models;
pub mod sessions;
pub mod usage;
pub mod files;
pub mod zen;
pub mod tools;
pub mod sandbox;
pub mod formats;
pub mod enforce;
pub mod agent_protocol;
pub mod tasks;
pub mod user_questions;
pub mod agent_env;
pub mod anti_printer;
pub mod synergy;
pub mod rag;
// S4-03: WebSocket agent streaming
pub mod ws_agent;
pub mod strict_locks;
pub mod identity_shield;
pub mod workspaces;
pub mod agent_chat;

use axum::{
    extract::State,
    http::{header::AUTHORIZATION, Request, StatusCode},
    middleware::Next,
    response::Response,
};
use std::sync::Arc;
use tracing::{warn, debug};
use crate::{AppState};
use crate::auth::{verify_token, token_info, AuthenticatedUser};


use std::ops::Deref;

/// Extension Key: يحمل هوية المستخدم الموثقة بعد المصادقة
#[derive(Debug, Clone)]
pub struct AuthUser(pub AuthenticatedUser);

impl Deref for AuthUser {
    type Target = AuthenticatedUser;
    fn deref(&self) -> &Self::Target { &self.0 }
}

/// Middleware للمصادقة — يستخرج user_id من Bearer token
///
/// ## تدفق العمل:
/// 1. استخراج التوكن من `Authorization: Bearer <token>`
/// 2. التحقق من التوقيع HMAC-SHA256
/// 3. التحقق من صلاحية التوكن (30 يوماً)
/// 4. إرفاق `AuthUser` مع الطلب لكل الملفات بالأسفل
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
            warn!("Missing or invalid Authorization header");
            StatusCode::UNAUTHORIZED
        })?;

    let token_str = token.to_string();

    // ⚠️ قبول التوكنات المحلية للتطوير فقط — مع تسجيل تحذير
    if token_str.starts_with("tg-") || token_str.starts_with("local-") || token_str == "dev-mode" {
        warn!("⚠️ Dev-mode token accepted (should not be used in production): {}", token_str);
        let user_id_for_ext = token_str.clone();
        let guest = AuthenticatedUser {
            user_id: user_id_for_ext.clone(),
            telegram_id: 0,
            first_name: "Development".to_string(),
            token: token_str,
            is_guest: true,
        };
        // Insert both new and legacy extensions for backward compat
        req.extensions_mut().insert(AuthUser(guest));
        req.extensions_mut().insert(UserId(user_id_for_ext));
        return Ok(next.run(req).await);
    }

    // التحقق الرسمي من التوكن
    let user_id_str = verify_token(&token_str, &state.session_secret)
        .ok_or_else(|| {
            warn!("Token verification failed: {}", token_info(&token_str));
            StatusCode::UNAUTHORIZED
        })?;

    debug!("Auth OK: user={}, token={}", user_id_str, token_info(&token_str));

    // Insert both new and legacy extensions for backward compat
    let auth_user = AuthenticatedUser {
        user_id: user_id_str.clone(),
        telegram_id: 0,
        first_name: String::new(),
        token: token_str,
        is_guest: false,
    };
    req.extensions_mut().insert(AuthUser(auth_user));
    req.extensions_mut().insert(UserId(user_id_str));
    Ok(next.run(req).await)
}

/// ⚠️ قديم — يُستخدم للتوافق مع الكود القديم فقط
/// استخدام `AuthUser` الجديد بدلاً منه
#[derive(Clone)]
pub struct UserId(pub String);