//! # Agent Environment & Skills Routes — البيئة والمهارات
//!
//! GET  /api/agent/environment       → وثيقة البيئة الكاملة
//! GET  /api/agent/skills            → قائمة المهارات
//! POST /api/agent/skills/suggest    → اقتراح المهارات لمهمة
//! POST /api/agent/skills/execute    → تنفيذ مهارة

use axum::{Extension, Json};
use serde_json::{json, Value};
use crate::routes::AuthUser;
use crate::agent::skills::{SkillRegistry, SkillContext};

/// GET /api/agent/environment — وثيقة البيئة الكاملة
pub async fn get_environment() -> Json<Value> {
    Json(json!({
        "platform": "requiem-agent",
        "version": env!("CARGO_PKG_VERSION"),
        "sandbox": {
            "layers": ["landlock", "seccomp", "rlimit", "user_ns"],
            "max_processes": 50,
            "max_memory_mb": 4096,
            "network_access": false,
            "persistent_storage": true,
        },
        "database": {
            "type": "turso",
            "isolation": "per-user",
            "shared": false,
        },
        "resources": {
            "cpu_cores": 2,
            "ram_gb": 16,
            "max_concurrent_sandboxes": 4,
        },
        "models": {
            "available": [
                {"id": "deepseek-v4-flash-free", "roles": ["code","general","debug","explore"]},
                {"id": "hy3-free", "roles": ["plan","research","debug","security"]},
                {"id": "mimo-v2.5-free", "roles": ["vision","code","general"]},
                {"id": "nemotron-3-ultra-free", "roles": ["debug","security","plan"]},
                {"id": "north-mini-code-free", "roles": ["review","code","explore"]},
                {"id": "big-pickle", "roles": ["multi-file","code","review"]},
            ],
            "auto_select": true,
            "health_probes": true,
        },
        "protocols": {
            "thinking": ["situation_analysis", "hypothesis_generation", "solution_evaluation", "exec_with_reasoning"],
            "modes": ["autonomous", "supervised", "audit", "tutorial", "turbo"],
            "compiler": {"auto_correct": true, "security_scan": true, "schema_validation": true},
        },
        "tools_count": crate::tools::ToolRegistry::new().count(),
        "storage": {
            "type": "isolated_per_user",
            "max_files_per_session": 1000,
            "max_storage_per_user_mb": 100,
        },
        "user_isolation": {
            "filesystem": "landlock + user namespace",
            "process": "seccomp-bpf + rlimit",
            "network": "blocked in sandbox",
        },
    }))
}

/// GET /api/agent/skills — قائمة المهارات
pub async fn list_skills() -> Json<Value> {
    let reg = SkillRegistry::new();
    Json(json!({
        "skills": reg.list(),
        "count": reg.count(),
    }))
}

/// POST /api/agent/skills/suggest — اقتراح المهارات
pub async fn suggest_skills(Json(body): Json<Value>) -> Json<Value> {
    let task = body["task"].as_str().unwrap_or("");
    let reg = SkillRegistry::new();
    let suggested = reg.suggest_for_task(task);
    Json(json!({
        "success": true,
        "task": task,
        "suggested_skills": suggested,
        "details": suggested.iter().filter_map(|s| reg.get(s)).map(|s| serde_json::json!({
            "name": s.name(),
            "description": s.description(),
            "required_tools": s.required_tools(),
        })).collect::<Vec<_>>(),
    }))
}

/// POST /api/agent/skills/execute — تنفيذ مهارة
pub async fn execute_skill(
    Extension(_auth): Extension<AuthUser>,
    Json(body): Json<Value>,
) -> Json<Value> {
    let skill_name = body["skill"].as_str().unwrap_or("");
    let task = body["task"].as_str().unwrap_or("");

    if skill_name.is_empty() {
        return Json(json!({ "success": false, "error": "skill مطلوب" }));
    }

    let reg = SkillRegistry::new();
    match reg.get(skill_name) {
        Some(skill) => {
            let context = SkillContext {
                user_id: body["user_id"].as_str().unwrap_or("unknown").to_string(),
                task: task.to_string(),
                available_tools: body["tools"].as_array()
                    .map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                    .unwrap_or_default(),
                available_models: body["models"].as_array()
                    .map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                    .unwrap_or_default(),
                environment: body["environment"].clone(),
            };

            match skill.execute(&context) {
                Ok(output) => Json(json!({
                    "success": true,
                    "skill": skill_name,
                    "output": output.result,
                    "artifacts": output.artifacts,
                    "duration_ms": output.duration_ms,
                })),
                Err(e) => Json(json!({
                    "success": false,
                    "error": e.message,
                    "recoverable": e.recoverable,
                })),
            }
        }
        None => Json(json!({
            "success": false,
            "error": format!("مهارة '{}' غير موجودة", skill_name),
            "available_skills": reg.list().iter().map(|s| s.name.clone()).collect::<Vec<_>>(),
        })),
    }
}
