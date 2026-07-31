//! # Identity Shield Routes v3 — مسارات API لدرع الهوية الصارم

use axum::{
    http::StatusCode,
    Json,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::agent::identity_shield::{IdentityShieldV3, ShieldStats, KnowledgeCutoffDetector, CutoffCheckResult};
use crate::db::AppState;

/// طلب فحص الهوية
#[derive(Debug, Deserialize)]
pub struct CheckIdentityRequest {
    pub user_input: String,
}

/// استجابة فحص الهوية
#[derive(Debug, Serialize)]
pub struct CheckIdentityResponse {
    pub is_probe: bool,
    pub probe_count: usize,
    pub responses: Vec<String>,
    pub needs_web_search: bool,
    pub web_search_reason: Option<String>,
    pub identity_maintained: bool,
    pub identity: String,
    pub developer_ar: String,
    pub developer_en: String,
    pub provider_ar: String,
    pub provider_en: String,
    pub last_update: String,
}

/// طلب فحص حد المعرفة
#[derive(Debug, Deserialize)]
pub struct CheckCutoffRequest {
    pub query: String,
}

/// إنشاء مسارات درع الهوية v3
pub fn identity_shield_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/identity/check", post(check_identity))
        .route("/identity/stats", get(get_stats))
        .route("/identity/prompt", get(get_identity_prompt))
        .route("/identity/cutoff", post(check_cutoff))
        .route("/identity/developer", get(get_developer_info))
}

/// فحص محاولة اختراق الهوية
pub async fn check_identity(
    Json(req): Json<CheckIdentityRequest>,
) -> Result<Json<CheckIdentityResponse>, StatusCode> {
    let mut shield = IdentityShieldV3::new("internal-model");
    let result = shield.check(&req.user_input);

    Ok(Json(CheckIdentityResponse {
        is_probe: result.is_probe,
        probe_count: result.probes.len(),
        responses: result.responses,
        needs_web_search: result.needs_web_search,
        web_search_reason: result.web_search_reason,
        identity_maintained: result.identity_maintained,
        identity: "Requiem Agent 1".to_string(),
        developer_ar: "ملوكي جمال".to_string(),
        developer_en: "Mellouki Jamal".to_string(),
        provider_ar: "Requiem Group".to_string(),
        provider_en: "Requiem Group".to_string(),
        last_update: "2026-06-11".to_string(),
    }))
}

/// جلب إحصائيات درع الهوية
pub async fn get_stats() -> Result<Json<ShieldStats>, StatusCode> {
    let shield = IdentityShieldV3::new("internal-model");
    Ok(Json(shield.stats()))
}

/// جلب system prompt للهوية
pub async fn get_identity_prompt() -> Result<Json<serde_json::Value>, StatusCode> {
    let shield = IdentityShieldV3::new("internal-model");
    let prompt = shield.generate_system_prompt();

    Ok(Json(serde_json::json!({
        "prompt": prompt,
        "identity": "Requiem Agent 1",
        "developer_ar": "ملوكي جمال",
        "developer_en": "Mellouki Jamal",
        "provider_ar": "Requiem Group",
        "provider_en": "Requiem Group",
        "last_update": "2026-06-11",
        "description": "Strict identity enforcement system prompt with provider protection"
    })))
}

/// فحص حد المعرفة
pub async fn check_cutoff(
    Json(req): Json<CheckCutoffRequest>,
) -> Result<Json<CutoffCheckResult>, StatusCode> {
    let detector = KnowledgeCutoffDetector::new();
    Ok(Json(detector.needs_current_info(&req.query)))
}

/// جلب معلومات المطور
pub async fn get_developer_info() -> Result<Json<serde_json::Value>, StatusCode> {
    Ok(Json(serde_json::json!({
        "developer_ar": "ملوكي جمال",
        "developer_en": "Mellouki Jamal",
        "provider_ar": "Requiem Group",
        "provider_en": "Requiem Group",
        "identity": "Requiem Agent 1",
        "last_update": "2026-06-11",
        "message": "This information is public and can be shared when asked."
    })))
}
