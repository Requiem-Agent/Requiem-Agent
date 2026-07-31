// ─── Phase 16 — Model Synergy Routes ─────────────────────────────────────
// 6 endpoints: run, set_pattern, history, report, performance, load

use axum::{Extension, Json};
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::sync::RwLock;
use crate::routes::AuthUser;
use crate::agent::synergy::{ModelSynergyCoordinator, SynergyPattern};

pub type SharedSynergy = Arc<RwLock<ModelSynergyCoordinator>>;

/// POST /api/agent/synergy/run — تشغيل جولة تآزر
pub async fn synergy_run(
    Extension(_auth): Extension<AuthUser>,
    Extension(synergy): Extension<SharedSynergy>,
    Json(body): Json<Value>,
) -> Json<Value> {
    let pattern_name = body["pattern"].as_str().unwrap_or("consensus");
    let question = body["question"].as_str().unwrap_or("");
    let task_type = body["task_type"].as_str().unwrap_or("general");
    let models: Vec<String> = body["models"].as_array()
        .map(|a| a.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
        .unwrap_or_else(|| vec!["gpt-4o".to_string(), "claude-3-opus".to_string(), "gemini-pro".to_string()]);

    if question.is_empty() {
        return Json(json!({ "success": false, "error": "question مطلوب" }));
    }

    let pattern = match pattern_name {
        "consensus" => SynergyPattern::Consensus,
        "critique" => SynergyPattern::Critique,
        "pipeline" => SynergyPattern::Pipeline,
        "pair" => SynergyPattern::Pair,
        _ => return Json(json!({ "success": false, "error": format!("نمط غير معروف: {pattern_name}") })),
    };

    let mut coord = synergy.write().await;
    let round = coord.run_round(pattern, question, &models, task_type).await;

    Json(json!({
        "success": true,
        "round_id": round.round_id,
        "pattern": round.pattern.name(),
        "models_used": round.models_used,
        "final_output": round.final_output,
        "consensus_score": round.consensus_score,
        "total_latency_ms": round.total_latency_ms,
        "total_tokens": round.total_tokens,
    }))
}

/// PUT /api/agent/synergy/pattern — تغيير النمط النشط
pub async fn synergy_set_pattern(
    Extension(_auth): Extension<AuthUser>,
    Extension(synergy): Extension<SharedSynergy>,
    Json(body): Json<Value>,
) -> Json<Value> {
    let pattern_name = body["pattern"].as_str().unwrap_or("");
    let pattern = match pattern_name {
        "consensus" => SynergyPattern::Consensus,
        "critique" => SynergyPattern::Critique,
        "pipeline" => SynergyPattern::Pipeline,
        "pair" => SynergyPattern::Pair,
        _ => return Json(json!({ "success": false, "error": format!("نمط غير معروف: {pattern_name}") })),
    };
    let mut coord = synergy.write().await;
    coord.set_pattern(pattern);
    Json(json!({ "success": true, "pattern": pattern_name }))
}

/// GET /api/agent/synergy/history — عرض تاريخ الجولات
pub async fn synergy_history(
    Extension(_auth): Extension<AuthUser>,
    Extension(synergy): Extension<SharedSynergy>,
) -> Json<Value> {
    let coord = synergy.read().await;
    let rounds: Vec<Value> = coord.recent_rounds(20).iter().map(|r| json!({
        "round_id": r.round_id,
        "pattern": r.pattern.name(),
        "models": r.models_used,
        "score": r.consensus_score,
        "latency_ms": r.total_latency_ms,
        "tokens": r.total_tokens,
    })).collect();
    Json(json!({ "success": true, "rounds": rounds, "total": coord.history.len() }))
}

/// GET /api/agent/synergy/report — تقرير كامل عن حالة التآزر
pub async fn synergy_report(
    Extension(_auth): Extension<AuthUser>,
    Extension(synergy): Extension<SharedSynergy>,
) -> Json<Value> {
    let coord = synergy.read().await;
    Json(json!({ "success": true, "report": coord.report() }))
}

/// GET /api/agent/synergy/performance — أداء النماذج حسب AdaptiveRouter
pub async fn synergy_performance(
    Extension(_auth): Extension<AuthUser>,
    Extension(synergy): Extension<SharedSynergy>,
) -> Json<Value> {
    let coord = synergy.read().await;
    Json(json!({
        "success": true,
        "performance": coord.router.summary(),
        "exploration_rate": coord.router.exploration_rate,
    }))
}

/// GET /api/agent/synergy/load — تقرير أحمال النماذج
pub async fn synergy_load(
    Extension(_auth): Extension<AuthUser>,
    Extension(synergy): Extension<SharedSynergy>,
) -> Json<Value> {
    let coord = synergy.read().await;
    Json(json!({
        "success": true,
        "load": coord.load_balancer.load_report(),
    }))
}
