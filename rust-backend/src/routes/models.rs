//! # Models Routes — مع الـ Model Registry الديناميكي
//!
//! GET /api/models          → قائمة النماذج + roleMap (للتوافق)
//! GET /api/models/health   → حالة جميع النماذج مع إحصائيات
//! GET /api/models/select   → اختيار النموذج الأنسب لمهمة
//! POST /api/models/probe   → فحص صحة نموذج معين

use axum::{Extension, Json};
use serde_json::{json, Value};
use std::collections::HashMap;
use crate::models::{self, ModelRegistry};
use crate::routes::AuthUser;

/// GET /api/models — قائمة النماذج (متوافقة مع الكود القديم)
pub async fn list_models() -> Json<Value> {
    let reg = models::registry();
    let model_list = reg.list_all().await;

    // build backward-compat roleMap
    let role_map: HashMap<&str, &str> = HashMap::from([
        ("coder", "deepseek-v4-flash-free"),
        ("orchestrator", "mimo-v2.5-free"),
        ("planner", "hy3-free"),
        ("reviewer", "north-mini-code-free"),
        ("debugger", "nemotron-3-ultra-free"),
        ("designer", "mimo-v2.5-free"),
        ("researcher", "hy3-free"),
        ("explorer", "big-pickle"),
        ("security", "deepseek-v4-flash-free"),
    ]);

    Json(json!({ "models": model_list, "roleMap": role_map }))
}

/// GET /api/models/health — حالة النماذج مع إحصائيات
pub async fn models_health(
    Extension(_auth): Extension<AuthUser>,
) -> Json<Value> {
    let reg = models::registry();
    let stats = reg.get_stats().await;
    Json(stats)
}

/// GET /api/models/select?category=code&effort=high — اختيار نموذج
pub async fn select_model(
    Extension(_auth): Extension<AuthUser>,
    axum::extract::Query(params): axum::extract::Query<HashMap<String, String>>,
) -> Json<Value> {
    let category = params.get("category").map(|s| s.as_str()).unwrap_or("general");
    let effort = params.get("effort").map(|s| s.as_str()).unwrap_or("medium");
    let selection = models::pick_model(category, effort).await;

    Json(json!({
        "category": category,
        "effort": effort,
        "selection": selection,
        "all_models": ModelRegistry::supported_categories(),
    }))
}

/// POST /api/models/probe — فحص نموذج معين
pub async fn probe_model(
    Extension(_auth): Extension<AuthUser>,
    Json(body): Json<Value>,
) -> Json<Value> {
    let model_id = body["model_id"].as_str().unwrap_or("");
    if model_id.is_empty() {
        return Json(json!({ "error": "model_id required" }));
    }

    let reg = models::registry();
    let available = reg.probe_model(model_id).await;
    let cap = reg.get_capability(model_id);

    Json(json!({
        "model_id": model_id,
        "available": available,
        "capability": cap.map(|c| json!({ "name": c.name, "roles": c.roles, "vision": c.vision })),
    }))
}

/// POST /api/models/probe-all — فحص جميع النماذج
pub async fn probe_all_models(
    Extension(_auth): Extension<AuthUser>,
) -> Json<Value> {
    let results = models::probe_all_models().await;
    Json(json!({
        "results": results.iter().map(|(id, ok)| json!({ "model_id": id, "available": ok })).collect::<Vec<_>>(),
        "total": results.len(),
        "available": results.iter().filter(|(_, ok)| *ok).count(),
    }))
}
