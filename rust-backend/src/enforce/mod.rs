//! # Enforce Module — النظام البرمجي الصارم
//!
//! ## المبادئ
//! 1. **Audit Log** — كل إجراء يُسجّل مع user_id, action, params, timestamp
//! 2. **Security Scan** — كشف API keys, tokens, secrets قبل التنفيذ
//! 3. **Path Enforce** — التحقق من أن المسارات ضمن جذر المستخدم (موجود في path_safety.rs)
//! 4. **Tool Validation** — التحقق من صحة معاملات الأداة ضد JSON Schema
//!
//! ## التكامل
//! ```rust
//! let enforcer = Enforcer::new(user_id);
//! enforcer.check_security(&code)?;    // افحص الأمان
//! enforcer.validate_path(&path)?;     // افحص المسار
//! enforcer.audit("code_exec", &params); // سجّل
//! ```

pub mod audit;
pub mod scanner;
pub mod locks;

use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::Arc;
use tokio::sync::RwLock;
use crate::enforce::audit::AuditLog;
use crate::enforce::scanner::SecurityScanner;
pub use locks::*;

// ─── Enforcer ──────────────────────────────────────────────────────────────

/// الـ Enforcer الرئيسي — يربط جميع أنظمة الفحص
pub struct Enforcer {
    pub user_id: String,
    pub audit_log: Arc<RwLock<AuditLog>>,
    scanner: SecurityScanner,
    pub strict_locks: StrictLocksEngine,
}

impl Enforcer {
    /// إنشاء Enforcer جديد لمستخدم معين
    pub fn new(user_id: &str, audit_log: Arc<RwLock<AuditLog>>) -> Self {
        Self {
            user_id: user_id.to_string(),
            audit_log,
            scanner: SecurityScanner::new(),
            strict_locks: StrictLocksEngine::new(),
        }
    }

    /// فحص أمني للكود — يبحث عن API keys, tokens, secrets
    pub fn check_security(&self, content: &str) -> Result<(), SecurityViolation> {
        self.scanner.scan(content)
    }

    /// التحقق من أمان مسار — يعيد استخدام path_safety::ensure_safe_path
    pub fn validate_path(&self, path: &Path, user_root: &Path) -> Result<std::path::PathBuf, String> {
        crate::path_safety::ensure_safe_path(path, user_root)
            .map_err(|e| e.to_string())
    }

    /// تسجيل إجراء في سجل التدقيق
    pub async fn audit(&self, action: &str, params: &serde_json::Value, success: bool) {
        let mut log = self.audit_log.write().await;
        log.record(&self.user_id, action, params, success);
    }

    /// تسجيل إجراء متزامن (للأماكن التي لا يمكن استخدام async فيها)
    pub fn audit_sync(&self, action: &str, params: &serde_json::Value, success: bool) {
        let mut log = self.audit_log.try_write()
            .expect("AuditLog try_write failed (deadlock?)");
        log.record(&self.user_id, action, params, success);
    }

    /// فحص الأقفال الصارمة
    pub fn check_strict_locks(
        &self,
        current_mode: &str,
        current_model: &str,
        response_format: &str,
        output: &str,
    ) -> LockCheckResult {
        self.strict_locks.check_all(current_mode, current_model, response_format, output)
    }

    /// توليد سياق الأقفال
    pub fn generate_lock_context(&self, mode: &str, effort: &str) -> String {
        self.strict_locks.generate_lock_context(mode, effort)
    }

    /// إحصائيات الأقفال
    pub fn locks_stats(&self) -> LocksStats {
        self.strict_locks.stats()
    }
}

// ─── SecurityViolation ─────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityViolation {
    pub severity: Severity,
    pub pattern: String,
    pub description: String,
    pub snippet: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Severity {
    /// معلومات عامة (مثلاً: يوجد تعليق يذكر كلمة password)
    Info,
    /// تحذير (مثلاً: متغير اسمه SECRET)
    Warning,
    /// خطير — يمنع التنفيذ (مثلاً: API key حقيقية)
    Critical,
}
