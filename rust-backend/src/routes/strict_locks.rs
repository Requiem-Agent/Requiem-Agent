//! # Strict Locks Routes — مسارات API للأقفال الصارمة

use axum::{
    http::StatusCode,
    Json,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::enforce::StrictLocksEngine;
use crate::db::AppState;

/// طلب فحص الأقفال
#[derive(Debug, Deserialize)]
pub struct CheckLocksRequest {
    pub mode: String,
    pub model: String,
    pub format: String,
    pub output: String,
}

/// استجابة فحص الأقفال
#[derive(Debug, Serialize)]
pub struct CheckLocksResponse {
    pub passed: bool,
    pub violations: Vec<LockViolationResponse>,
    pub suggested_action: String,
    pub lock_context: String,
}

/// انتهاك قفل (للإخراج)
#[derive(Debug, Serialize)]
pub struct LockViolationResponse {
    pub lock_type: String,
    pub violation_type: String,
    pub description: String,
    pub severity: String,
    pub auto_correctable: bool,
}

/// إحصائيات الأقفال
#[derive(Debug, Serialize)]
pub struct LocksStatsResponse {
    pub total_locks: usize,
    pub active_locks: usize,
    pub total_violations: u32,
    pub lock_details: std::collections::HashMap<String, LockDetailResponse>,
}

/// تفاصيل قفل (للإخراج)
#[derive(Debug, Serialize)]
pub struct LockDetailResponse {
    pub state: String,
    pub violations: u32,
    pub enforcement: String,
}

/// إنشاء مسارات الأقفال
pub fn strict_locks_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/locks/check", post(check_locks))
        .route("/locks/stats", get(get_stats))
        .route("/locks/context", post(get_lock_context))
}

/// فحص الأقفال
pub async fn check_locks(
    Json(req): Json<CheckLocksRequest>,
) -> Result<Json<CheckLocksResponse>, StatusCode> {
    let engine = StrictLocksEngine::new();

    let result = engine.check_all(&req.mode, &req.model, &req.format, &req.output);

    let lock_context = engine.generate_lock_context(&req.mode, "medium");

    let violations: Vec<LockViolationResponse> = result
        .violations
        .iter()
        .map(|v| LockViolationResponse {
            lock_type: v.lock_type.name().to_string(),
            violation_type: v.violation_type.clone(),
            description: v.description.clone(),
            severity: format!("{:?}", v.severity),
            auto_correctable: v.auto_correctable,
        })
        .collect();

    Ok(Json(CheckLocksResponse {
        passed: result.passed,
        violations,
        suggested_action: result.suggested_action,
        lock_context,
    }))
}

/// جلب إحصائيات الأقفال
pub async fn get_stats() -> Result<Json<LocksStatsResponse>, StatusCode> {
    let engine = StrictLocksEngine::new();
    let stats = engine.stats();

    let lock_details: std::collections::HashMap<String, LockDetailResponse> = stats
        .lock_details
        .into_iter()
        .map(|(k, v)| {
            (
                k,
                LockDetailResponse {
                    state: format!("{:?}", v.state),
                    violations: v.violations,
                    enforcement: format!("{:?}", v.enforcement),
                },
            )
        })
        .collect();

    Ok(Json(LocksStatsResponse {
        total_locks: stats.total_locks,
        active_locks: stats.active_locks,
        total_violations: stats.total_violations,
        lock_details,
    }))
}

/// جلب سياق الأقفال
pub async fn get_lock_context(
    Json(req): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let engine = StrictLocksEngine::new();
    let mode = req["mode"].as_str().unwrap_or("autonomous");
    let effort = req["effort"].as_str().unwrap_or("medium");

    let context = engine.generate_lock_context(mode, effort);

    Ok(Json(serde_json::json!({
        "context": context,
        "locks_count": 8,
        "active_locks": 8
    })))
}
