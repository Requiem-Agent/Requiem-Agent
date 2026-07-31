//! # Enforce Routes — API للفحص الأمني وسجل التدقيق
//!
//! GET  /api/enforce/audit          → سجل التدقيق كاملاً
//! GET  /api/enforce/audit/user/:id → سجل مستخدم معين
//! GET  /api/enforce/audit/recent/:n → آخر N سجل
//! GET  /api/enforce/audit/stats    → إحصائيات التدقيق
//! POST /api/enforce/check-security → فحص أمني للكود
//! POST /api/enforce/validate-path  → التحقق من أمان مسار

use axum::{Extension, Json};
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::sync::RwLock;
use crate::routes::AuthUser;
use crate::enforce::scanner::SecurityScanner;
use crate::enforce::Enforcer;

/// مشاركة AuditLog عبر التطبيق
pub type SharedAuditLog = Arc<RwLock<crate::enforce::audit::AuditLog>>;

/// إنشاء AuditLog مشترك
pub fn create_audit_log(max_entries: usize) -> SharedAuditLog {
    Arc::new(RwLock::new(crate::enforce::audit::AuditLog::new(max_entries)))
}

/// GET /api/enforce/audit — كل السجلات
pub async fn get_audit_log(
    Extension(auth): Extension<AuthUser>,
    Extension(log): Extension<SharedAuditLog>,
) -> Json<Value> {
    let log = log.read().await;
    Json(log.to_json())
}

/// GET /api/enforce/audit/user/:user_id — سجل مستخدم
pub async fn get_audit_by_user(
    Extension(_auth): Extension<AuthUser>,
    Extension(log): Extension<SharedAuditLog>,
    axum::extract::Path(user_id): axum::extract::Path<String>,
) -> Json<Value> {
    let log = log.read().await;
    let entries: Vec<_> = log.by_user(&user_id).into_iter().cloned().collect();
    Json(json!({
        "user_id": user_id,
        "count": entries.len(),
        "entries": entries,
    }))
}

/// GET /api/enforce/audit/recent/:n — آخر N سجل
pub async fn get_recent_audit(
    Extension(_auth): Extension<AuthUser>,
    Extension(log): Extension<SharedAuditLog>,
    axum::extract::Path(n): axum::extract::Path<usize>,
) -> Json<Value> {
    let log = log.read().await;
    let n = n.min(100);
    Json(json!({
        "count": n,
        "entries": log.recent(n),
    }))
}

/// GET /api/enforce/audit/stats — إحصائيات
pub async fn get_audit_stats(
    Extension(_auth): Extension<AuthUser>,
    Extension(log): Extension<SharedAuditLog>,
) -> Json<Value> {
    let log = log.read().await;
    Json(json!(log.stats()))
}

/// POST /api/enforce/check-security — فحص أمني للكود
pub async fn check_security(
    Extension(_auth): Extension<AuthUser>,
    Json(body): Json<Value>,
) -> Json<Value> {
    let content = body["content"].as_str().unwrap_or("");
    if content.is_empty() {
        return Json(json!({ "success": false, "error": "content مطلوب" }));
    }
    let scanner = SecurityScanner::new();
    let violations = scanner.scan_all(content);
    let has_critical = violations.iter().any(|v| v.severity == crate::enforce::Severity::Critical);

    Json(json!({
        "success": !has_critical,
        "safe": !has_critical,
        "violations_count": violations.len(),
        "violations": violations,
    }))
}

/// POST /api/enforce/validate-path — التحقق من أمان مسار
pub async fn enforce_validate_path(
    Extension(_auth): Extension<AuthUser>,
    Json(body): Json<Value>,
) -> Json<Value> {
    let path_str = body["path"].as_str().unwrap_or("");
    let root_str = body["root"].as_str().unwrap_or("/app/data/users/default");
    if path_str.is_empty() {
        return Json(json!({ "success": false, "error": "path مطلوب" }));
    }
    let path = std::path::Path::new(path_str);
    let root = std::path::Path::new(root_str);

    let enforcer = Enforcer::new("api", create_audit_log(1000));
    match enforcer.validate_path(path, root) {
        Ok(resolved) => Json(json!({
            "success": true,
            "safe": true,
            "path": path_str,
            "resolved": resolved.to_string_lossy(),
            "root": root_str,
        })),
        Err(e) => Json(json!({
            "success": false,
            "safe": false,
            "path": path_str,
            "error": e,
        })),
    }
}
