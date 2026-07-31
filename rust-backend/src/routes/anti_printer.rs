// ─── Phase 15 — Anti-Printer & Compiler Advanced Routes ───────────────────
// 12 endpoints: semantic analysis, anti-printer detection, pipeline, router

use axum::{Extension, Json};
use serde_json::{json, Value};
use crate::routes::AuthUser;
use crate::agent::anti_printer::{CompilerPipeline, PatternDetector, SemanticEngine, ContextRouter, DistributionStrategy};

/// نوع المشاركات للتطبيق
use std::sync::Arc;
use tokio::sync::RwLock;

pub type SharedPipeline = Arc<RwLock<CompilerPipeline>>;
pub type SharedRouter = Arc<RwLock<ContextRouter>>;
pub type SharedDetector = Arc<RwLock<PatternDetector>>;
pub type SharedSemantic = Arc<RwLock<SemanticEngine>>;

// ─── Semantic Analysis ──────────────────────────────────────────────────

/// POST /api/agent/semantic/analyze — تحليل دلالي لنص
pub async fn semantic_analyze(
    Extension(_auth): Extension<AuthUser>,
    Extension(semantic): Extension<SharedSemantic>,
    Json(body): Json<Value>,
) -> Json<Value> {
    let text = body["text"].as_str().unwrap_or("");
    let step_id = body["step_id"].as_u64().unwrap_or(0);
    if text.is_empty() {
        return Json(json!({ "success": false, "error": "text مطلوب" }));
    }
    let mut engine = semantic.write().await;
    let result = engine.analyze(text, step_id);
    Json(json!({
        "success": true,
        "intent": result.intent.name(),
        "confidence": result.confidence,
        "keywords": result.keywords,
        "entities": result.entities,
        "requires_clarification": result.requires_clarification,
        "suggested_mode": result.suggested_mode,
    }))
}

/// GET /api/agent/semantic/context — عرض السياق الحالي
pub async fn semantic_context(
    Extension(_auth): Extension<AuthUser>,
    Extension(semantic): Extension<SharedSemantic>,
) -> Json<Value> {
    let engine = semantic.read().await;
    let context = engine.context();
    let frames: Vec<Value> = context.iter().map(|f| json!({
        "step_id": f.step_id,
        "intent": f.intent.name(),
        "confidence": f.confidence,
        "keywords": f.keywords,
        "entities": f.entities,
        "timestamp": f.timestamp,
    })).collect();
    Json(json!({ "success": true, "frames": frames, "total": frames.len() }))
}

// ─── Anti-Printer Detection ────────────────────────────────────────────

/// POST /api/agent/anti-printer/check — فحص أنماط الطباعة الفارغة
pub async fn anti_printer_check(
    Extension(_auth): Extension<AuthUser>,
    Extension(detector): Extension<SharedDetector>,
    Json(body): Json<Value>,
) -> Json<Value> {
    let text = body["text"].as_str().unwrap_or("");
    let tool_calls = body["tool_calls"].as_array().map(|a| a.clone()).unwrap_or_default();
    let history: Vec<String> = body["history"].as_array()
        .map(|a| a.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
        .unwrap_or_default();
    if text.is_empty() {
        return Json(json!({ "success": false, "error": "text مطلوب" }));
    }
    let det = detector.read().await;
    let report = det.analyze(text, &tool_calls, &history);
    Json(json!({
        "success": true,
        "has_issues": report.has_issues,
        "patterns": report.patterns.iter().map(|p| json!({
            "pattern_type": p.pattern_type.name(),
            "description": p.description,
            "severity": format!("{:?}", p.severity),
            "location": p.location,
            "suggestion": p.suggestion,
        })).collect::<Vec<_>>(),
        "quality_score": report.quality_score,
        "requires_retry": report.requires_retry,
        "suggested_action": report.suggested_action,
    }))
}

// ─── Pipeline ──────────────────────────────────────────────────────────

/// POST /api/agent/anti-printer/pipeline — تشغيل الـ Pipeline الكامل
pub async fn anti_printer_pipeline(
    Extension(_auth): Extension<AuthUser>,
    Extension(pipeline): Extension<SharedPipeline>,
    Json(body): Json<Value>,
) -> Json<Value> {
    let thinking = body["thinking"].as_str().unwrap_or("");
    let tool_calls = body["tool_calls"].as_str().unwrap_or("");
    let history: Vec<String> = body["history"].as_array()
        .map(|a| a.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
        .unwrap_or_default();
    let step_id = body["step_id"].as_u64().unwrap_or(0);
    if thinking.is_empty() {
        return Json(json!({ "success": false, "error": "thinking مطلوب" }));
    }
    let mut pipe = pipeline.write().await;
    let report = pipe.run(thinking, tool_calls, &history, step_id).await;
    Json(json!({
        "success": report.passed,
        "summary": report.summary(),
        "passed": report.passed,
        "stages": report.stages.iter().map(|s| json!({
            "stage": s.stage.name(),
            "passed": s.passed,
            "issues": s.issues,
            "duration_ms": s.duration_ms,
        })).collect::<Vec<_>>(),
        "anti_printer": {
            "has_issues": report.anti_printer.has_issues,
            "quality_score": report.anti_printer.quality_score,
            "requires_retry": report.anti_printer.requires_retry,
        },
        "semantic": report.semantic.as_ref().map(|s| json!({
            "intent": s.intent.name(),
            "confidence": s.confidence,
        })),
        "corrected_output": report.corrected_output,
        "security_issues": report.security_issues,
        "total_duration_ms": report.total_duration_ms,
    }))
}

// ─── Context Router ────────────────────────────────────────────────────

/// POST /api/agent/router/route — توزيع مهمة على نموذج
pub async fn router_route(
    Extension(_auth): Extension<AuthUser>,
    Extension(router): Extension<SharedRouter>,
    Json(body): Json<Value>,
) -> Json<Value> {
    let description = body["description"].as_str().unwrap_or("");
    let task_id = body["task_id"].as_str().unwrap_or("default");
    let estimated_tokens = body["estimated_tokens"].as_u64().unwrap_or(1000) as usize;
    if description.is_empty() {
        return Json(json!({ "success": false, "error": "description مطلوب" }));
    }
    let mut rtr = router.write().await;
    let dist = rtr.route(description, task_id, estimated_tokens);
    Json(json!({
        "success": true,
        "task_id": dist.task_id,
        "assigned_model": dist.assigned_model,
        "confidence": dist.confidence,
        "reason": dist.reason,
        "estimated_tokens": dist.estimated_tokens,
    }))
}

/// GET /api/agent/router/models — قائمة النماذج مع إحصائياتها
pub async fn router_models(
    Extension(_auth): Extension<AuthUser>,
    Extension(router): Extension<SharedRouter>,
) -> Json<Value> {
    let rtr = router.read().await;
    Json(json!({
        "success": true,
        "models": rtr.models_summary(),
        "strategy": rtr.strategy.name(),
    }))
}

/// POST /api/agent/router/strategy — تغيير استراتيجية التوزيع
pub async fn router_strategy(
    Extension(_auth): Extension<AuthUser>,
    Extension(router): Extension<SharedRouter>,
    Json(body): Json<Value>,
) -> Json<Value> {
    let strategy_name = body["strategy"].as_str().unwrap_or("");
    let strategy = match strategy_name {
        "best_fit" => DistributionStrategy::BestFit,
        "round_robin" => DistributionStrategy::RoundRobin,
        "fastest" => DistributionStrategy::Fastest,
        "most_reliable" => DistributionStrategy::MostReliable,
        "load_balance" => DistributionStrategy::LoadBalance,
        _ => return Json(json!({ "success": false, "error": format!("استراتيجية غير معروفة: {strategy_name}") })),
    };
    let mut rtr = router.write().await;
    rtr.set_strategy(strategy);
    Json(json!({ "success": true, "strategy": strategy_name }))
}

/// POST /api/agent/router/outcome — تسجيل نجاح/فشل نموذج
pub async fn router_outcome(
    Extension(_auth): Extension<AuthUser>,
    Extension(router): Extension<SharedRouter>,
    Json(body): Json<Value>,
) -> Json<Value> {
    let model_name = body["model"].as_str().unwrap_or("");
    let success = body["success"].as_bool().unwrap_or(true);
    if model_name.is_empty() {
        return Json(json!({ "success": false, "error": "model مطلوب" }));
    }
    let mut rtr = router.write().await;
    rtr.record_outcome(model_name, success);
    Json(json!({ "success": true, "model": model_name, "outcome": success }))
}
