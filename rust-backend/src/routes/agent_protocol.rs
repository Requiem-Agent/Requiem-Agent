//! # Agent Protocol Routes — مسارات بروتوكولات الوكيل
//!
//! GET  /api/agent/status                  → حالة الوكيل
//! POST /api/agent/mode/set                → تغيير الوضع
//! GET  /api/agent/mode/current            → الوضع الحالي
//! POST /api/agent/mode/auto-switch        → اقتراح الوضع
//! GET  /api/agent/mode/history            → تاريخ التغييرات
//! POST /api/agent/protocol/thinking/validate → تحقق من التفكير
//! POST /api/agent/compiler/correct        → تصحيح JSON
//! POST /api/agent/compiler/compile        → تجميع المخرجات
//! POST /api/agent/sub/spawn               → إنشاء وكيل فرعي
//! GET  /api/agent/sub/:id/progress        → تقدم الوكيل الفرعي
//! POST /api/agent/sub/:id/cancel          → إلغاء
//! POST /api/agent/sub/merge               → دمج النتائج
//! GET  /api/agent/sub/list                → قائمة الوكلاء الفرعيين

use axum::{Extension, Json};
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::sync::RwLock;
use crate::routes::AuthUser;
use crate::agent::protocol::mode::{AgentMode, ModeController};
use crate::agent::protocol::thinking::{ThinkingStep, ThinkingStage};
use crate::agent::compiler::auto_correct::JsonAutoCorrect;
use crate::agent::compiler::output::{AgentOutputCompiler, CompilerConfig};
use crate::agent::protocol::sub_agent::{SubAgentSpec, IsolationLevel};

/// مشاركة المحرك عبر التطبيق (يُدار في main.rs)
pub type SharedEngine = Arc<RwLock<crate::agent::AgentEngine>>;

/// GET /api/agent/status
pub async fn agent_status(
    Extension(_auth): Extension<AuthUser>,
    Extension(engine): Extension<SharedEngine>,
) -> Json<Value> {
    let engine = engine.read().await;
    Json(engine.status_report())
}

/// POST /api/agent/mode/set
pub async fn set_mode(
    Extension(_auth): Extension<AuthUser>,
    Extension(engine): Extension<SharedEngine>,
    Json(body): Json<Value>,
) -> Json<Value> {
    let mode_str = body["mode"].as_str().unwrap_or("");
    let reason = body["reason"].as_str().unwrap_or("تغيير يدوي");
    let mode = match mode_str {
        "autonomous" => AgentMode::Autonomous,
        "supervised" => AgentMode::Supervised,
        "audit" => AgentMode::Audit,
        "tutorial" => AgentMode::Tutorial,
        "turbo" => AgentMode::Turbo,
        _ => return Json(json!({
            "success": false,
            "error": format!("وضع غير معروف: '{}'. الخيارات: autonomous, supervised, audit, tutorial, turbo", mode_str)
        })),
    };

    let mut engine = engine.write().await;
    let constraints = engine.switch_mode(mode, reason);

    Json(json!({
        "success": true,
        "mode": mode.name(),
        "constraints": {
            "require_approval": constraints.require_approval,
            "max_consecutive_steps": constraints.max_consecutive_steps,
            "require_reasoning": constraints.require_reasoning,
            "max_sub_agents": constraints.max_sub_agents,
        },
    }))
}

/// GET /api/agent/mode/current
pub async fn current_mode(
    Extension(_auth): Extension<AuthUser>,
    Extension(engine): Extension<SharedEngine>,
) -> Json<Value> {
    let engine = engine.read().await;
    let mode = engine.mode.current();
    let constraints = engine.mode.constraints();

    Json(json!({
        "mode": mode.name(),
        "description": mode.description(),
        "constraints": {
            "require_approval": constraints.require_approval,
            "max_consecutive_steps": constraints.max_consecutive_steps,
            "require_reasoning": constraints.require_reasoning,
            "require_tool_selection": constraints.require_tool_selection,
            "audit_level": format!("{:?}", constraints.audit_level),
        },
        "history_count": engine.mode.history().len(),
    }))
}

/// POST /api/agent/mode/auto-switch
pub async fn auto_switch_mode(
    Extension(_auth): Extension<AuthUser>,
    Extension(engine): Extension<SharedEngine>,
    Json(body): Json<Value>,
) -> Json<Value> {
    let task = body["task"].as_str().unwrap_or("");
    let suggested = ModeController::suggest_mode(task);

    let mut engine = engine.write().await;
    let constraints = engine.switch_mode(suggested, "auto-switch بناءً على المهمة");

    Json(json!({
        "success": true,
        "suggested_mode": suggested.name(),
        "previous_mode": engine.mode.history().last()
            .map(|h| h.from.name()).unwrap_or("unknown"),
        "constraints": {
            "require_approval": constraints.require_approval,
            "max_steps": constraints.max_consecutive_steps,
        },
    }))
}

/// GET /api/agent/mode/history
pub async fn mode_history(
    Extension(_auth): Extension<AuthUser>,
    Extension(engine): Extension<SharedEngine>,
) -> Json<Value> {
    let engine = engine.read().await;
    Json(json!({
        "history": engine.mode.history(),
    }))
}

/// POST /api/agent/protocol/thinking/validate
pub async fn validate_thinking(
    Extension(_auth): Extension<AuthUser>,
    Extension(engine): Extension<SharedEngine>,
    Json(body): Json<Value>,
) -> Json<Value> {
    let mut engine = engine.write().await;

    // إذا كان هناك خطوات، سجلها
    if let Some(steps) = body["steps"].as_array() {
        engine.thinking.start_session();
        for step_val in steps {
            let stage_str = step_val["stage"].as_str().unwrap_or("");
            let stage = match stage_str {
                "situation_analysis" => ThinkingStage::SituationAnalysis,
                "hypothesis_generation" => ThinkingStage::HypothesisGeneration,
                "solution_evaluation" => ThinkingStage::SolutionEvaluation,
                "exec_with_reasoning" => ThinkingStage::ExecWithReasoning,
                _ => continue,
            };

            let step = ThinkingStep {
                stage,
                reasoning: step_val["reasoning"].as_str().unwrap_or("").to_string(),
                confidence: step_val["confidence"].as_f64().unwrap_or(0.5) as f32,
                tokens_used: step_val["tokens_used"].as_u64().unwrap_or(0) as u32,
                duration_ms: step_val["duration_ms"].as_u64().unwrap_or(0),
                artifacts: vec![],
                tools_considered: vec![],
                selected_tool: step_val["selected_tool"].as_str().map(|s| s.to_string()),
            };

            if let Err(violation) = engine.thinking.record_step(step) {
                return Json(json!({
                    "valid": false,
                    "violation": {
                        "code": format!("{}", violation.code),
                        "message": violation.message,
                        "suggestion": violation.suggestion,
                    },
                    "validation": engine.thinking.validate_session(),
                }));
            }
        }
    }

    let validation = engine.thinking.validate_session();
    Json(json!({
        "valid": validation.valid,
        "report": validation,
    }))
}

/// POST /api/agent/compiler/correct
pub async fn correct_json(
    Extension(_auth): Extension<AuthUser>,
    Json(body): Json<Value>,
) -> Json<Value> {
    let raw = body["json"].as_str().unwrap_or("");
    if raw.is_empty() {
        return Json(json!({ "success": false, "error": "json مطلوب" }));
    }

    let mut corrector = JsonAutoCorrect::new();
    let result = corrector.correct(raw, None);

    Json(json!({
        "success": result.success,
        "error": result.error,
        "corrections_applied": result.corrections.len(),
        "corrections": result.corrections,
        "tool_calls": result.tool_calls,
        "compile_time_ms": result.compile_time_ms,
    }))
}

/// POST /api/agent/compiler/compile
pub async fn compile_output(
    Extension(_auth): Extension<AuthUser>,
    Json(body): Json<Value>,
) -> Json<Value> {
    let raw = body["output"].as_str().unwrap_or("");
    if raw.is_empty() {
        return Json(json!({ "success": false, "error": "output مطلوب" }));
    }

    let mut compiler = AgentOutputCompiler::new(CompilerConfig::default());
    let compiled = compiler.compile(raw);

    Json(json!({
        "success": compiled.valid,
        "report": compiler.report(&compiled),
    }))
}

/// POST /api/agent/sub/spawn
pub async fn spawn_sub_agent(
    Extension(_auth): Extension<AuthUser>,
    Extension(engine): Extension<SharedEngine>,
    Json(body): Json<Value>,
) -> Json<Value> {
    let task = body["task"].as_str().unwrap_or("").to_string();
    let model_id = body["model_id"].as_str().unwrap_or("deepseek-v4-flash-free").to_string();
    let tools: Vec<String> = body["tools"].as_array()
        .map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect())
        .unwrap_or_default();

    if task.is_empty() {
        return Json(json!({ "success": false, "error": "task مطلوب" }));
    }

    let mode_str = body["mode"].as_str().unwrap_or("autonomous");
    let mode = match mode_str {
        "supervised" => AgentMode::Supervised,
        "audit" => AgentMode::Audit,
        "turbo" => AgentMode::Turbo,
        _ => AgentMode::Autonomous,
    };

    let mut engines = engine.write().await;
    let spec = SubAgentSpec {
        id: format!("sub-{}", engines.sub_agents.list_children().len() + 1),
        task,
        model_id,
        mode,
        tools,
        max_steps: body["max_steps"].as_u64().unwrap_or(10) as usize,
        output_schema: crate::tools::JsonSchema {
            schema_type: "object".into(),
            properties: None,
            required: None,
            description: Some("نتيجة الوكيل الفرعي".into()),
        },
        parent_id: Some(engines.user_id.clone()),
        isolation: IsolationLevel::TaskOnly,
        context: body["context"].as_object().map(|o| serde_json::Value::Object(o.clone())),
        priority: body["priority"].as_u64().unwrap_or(5) as u8,
        timeout_minutes: body["timeout_minutes"].as_u64().unwrap_or(30) as u32,
    };

    match engines.sub_agents.spawn(spec) {
        Ok(id) => Json(json!({
            "success": true,
            "sub_agent_id": id,
            "status": "spawning",
        })),
        Err(e) => Json(json!({
            "success": false,
            "error": e.message,
        })),
    }
}

/// GET /api/agent/sub/:id/progress
pub async fn sub_agent_progress(
    Extension(_auth): Extension<AuthUser>,
    Extension(engine): Extension<SharedEngine>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Json<Value> {
    let engine = engine.read().await;
    match engine.sub_agents.get_progress(&id) {
        Some(progress) => Json(json!({
            "success": true,
            "progress": progress,
        })),
        None => Json(json!({
            "success": false,
            "error": format!("الوكيل الفرعي {} غير موجود", id),
        })),
    }
}

/// POST /api/agent/sub/:id/cancel
pub async fn cancel_sub_agent(
    Extension(_auth): Extension<AuthUser>,
    Extension(engine): Extension<SharedEngine>,
    axum::extract::Path(id): axum::extract::Path<String>,
    Json(body): Json<Value>,
) -> Json<Value> {
    let reason = body["reason"].as_str().unwrap_or("ألغاه المستخدم");
    let mut engine = engine.write().await;
    match engine.sub_agents.cancel(&id, reason) {
        Ok(_) => Json(json!({ "success": true })),
        Err(e) => Json(json!({ "success": false, "error": e })),
    }
}

/// POST /api/agent/sub/merge
pub async fn merge_sub_agents(
    Extension(_auth): Extension<AuthUser>,
    Extension(engine): Extension<SharedEngine>,
    Json(body): Json<Value>,
) -> Json<Value> {
    let ids: Vec<String> = body["ids"].as_array()
        .map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect())
        .unwrap_or_default();

    if ids.is_empty() {
        return Json(json!({ "success": false, "error": "ids مطلوب (قائمة معرفات الوكلاء الفرعيين)" }));
    }

    let engine = engine.read().await;
    match engine.sub_agents.merge_results(&ids) {
        Ok(results) => Json(json!({
            "success": true,
            "merged": results,
        })),
        Err(e) => Json(json!({ "success": false, "error": e })),
    }
}

/// GET /api/agent/sub/list
pub async fn list_sub_agents(
    Extension(_auth): Extension<AuthUser>,
    Extension(engine): Extension<SharedEngine>,
) -> Json<Value> {
    let engine = engine.read().await;
    Json(json!({
        "sub_agents": engine.sub_agents.list_children(),
        "stats": engine.sub_agents.stats(),
    }))
}
