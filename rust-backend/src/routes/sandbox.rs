//! # Sandbox Routes — مع جدولة ذكية (2 vCPU, 16GB RAM)
//!
//! POST /api/sandbox/exec     → تنفيذ كود مع قيود الموارد
//! POST /api/sandbox/cleanup  → تنظيف
//! GET  /api/sandbox/status   → حالة الساندبوكس
//! GET  /api/sandbox/stats    → إحصائيات النظام

use axum::{Extension, Json};
use serde_json::{json, Value};
use std::collections::HashMap;
use crate::routes::AuthUser;
use crate::sandbox::{SandboxExecutor, SandboxRequest, get_sandbox_stats};

/// POST /api/sandbox/exec — تنفيذ كود مع الجدولة الذكية
pub async fn execute_code(
    Extension(auth): Extension<AuthUser>,
    Json(body): Json<Value>,
) -> Json<Value> {
    let user_id = &auth.user_id;
    let code = body["code"].as_str().unwrap_or("").to_string();
    let session_id = body["session_id"].as_str().unwrap_or("default").to_string();

    if code.is_empty() {
        return Json(json!({ "success": false, "error": "code مطلوب" }));
    }

    let req = SandboxRequest {
        code,
        language: None,
        timeout_secs: body["timeout_secs"].as_u64(),
        env: None,
        stream: None,
        wait_in_queue: Some(true),
    };

    match SandboxExecutor::execute(req, user_id, &session_id).await {
        Ok(r) => Json(json!({
            "success": r.success,
            "stdout": r.stdout,
            "stderr": r.stderr,
            "exit_code": r.exit_code,
            "duration_ms": r.duration_ms,
            "language": format!("{:?}", r.language).to_lowercase(),
            "timed_out": r.timed_out,
            "output_truncated": r.output_truncated,
            "queued_ms": r.queued_ms,
            "had_to_wait": r.had_to_wait,
        })),
        Err(e) => Json(json!({
            "success": false,
            "error": e,
        })),
    }
}

/// POST /api/sandbox/cleanup
pub async fn cleanup_sandbox(
    Extension(auth): Extension<AuthUser>,
    Json(body): Json<Value>,
) -> Json<Value> {
    let sid = body["session_id"].as_str().unwrap_or("default");
    match SandboxExecutor::cleanup(&auth.user_id, sid).await {
        Ok(_) => Json(json!({ "success": true })),
        Err(e) => Json(json!({ "success": false, "error": e })),
    }
}

/// GET /api/sandbox/status
pub async fn sandbox_status(
    Extension(auth): Extension<AuthUser>,
    axum::extract::Query(params): axum::extract::Query<HashMap<String, String>>,
) -> Json<Value> {
    let sid = params.get("session_id").map(|s| s.as_str()).unwrap_or("default");
    let sp = SandboxExecutor::spath(&auth.user_id, sid);
    let exists = sp.exists();
    let size = SandboxExecutor::size(&auth.user_id, sid).await.unwrap_or(0);
    let fc = if exists { count_files(&sp) } else { 0 };

    Json(json!({
        "exists": exists, "path": sp.to_string_lossy(),
        "size_bytes": size, "file_count": fc, "session_id": sid,
    }))
}

/// GET /api/sandbox/stats — إحصائيات الساندبوكس العالمية
pub async fn sandbox_stats(
    Extension(_auth): Extension<AuthUser>,
) -> Json<Value> {
    let stats = get_sandbox_stats();
    Json(json!({
        "active_sandboxes": stats.active_sandboxes,
        "max_concurrent_exec": stats.max_concurrent_exec,
        "max_concurrent_compile": stats.max_concurrent_compile,
        "max_total": stats.max_total,
        "queue_timeout_secs": stats.queue_timeout_secs,
        "usage_percent": if stats.max_total > 0 {
            (stats.active_sandboxes as f64 / stats.max_total as f64 * 100.0) as u32
        } else { 0 },
    }))
}

fn count_files(dir: &std::path::Path) -> usize {
    let mut c = 0;
    if let Ok(e) = std::fs::read_dir(dir) {
        for entry in e.flatten() {
            if entry.path().is_dir() { c += count_files(&entry.path()); } else { c += 1; }
        }
    }
    c
}
