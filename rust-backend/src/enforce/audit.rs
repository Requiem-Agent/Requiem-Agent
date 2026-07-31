//! # AuditLog — سجل التدقيق
//!
//! يسجل كل إجراء يقوم به الوكيل مع:
//! - user_id: من قام بالإجراء
//! - action: ماذا فعل (code_exec, file_edit, tool_call, ...)
//! - params: معاملات الإجراء (بدون محتوى حساس)
//! - success: هل نجح؟
//! - timestamp: متى؟
//! - duration_ms: كم استغرق؟

use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

/// سجل واحد في التدقيق
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    pub user_id: String,
    pub action: String,
    pub params: serde_json::Value,
    pub success: bool,
    pub timestamp: String,
    pub duration_ms: Option<u64>,
}

/// سجل التدقيق — يحتفظ بآخر N سجل
pub struct AuditLog {
    entries: VecDeque<AuditEntry>,
    max_entries: usize,
}

impl AuditLog {
    /// إنشاء سجل تدقيق جديد
    pub fn new(max_entries: usize) -> Self {
        Self {
            entries: VecDeque::with_capacity(max_entries.min(10000)),
            max_entries,
        }
    }

    /// تسجيل إجراء
    pub fn record(
        &mut self,
        user_id: &str,
        action: &str,
        params: &serde_json::Value,
        success: bool,
    ) {
        let entry = AuditEntry {
            user_id: user_id.to_string(),
            action: action.to_string(),
            params: self.sanitize_params(params),
            success,
            timestamp: chrono::Utc::now().to_rfc3339(),
            duration_ms: None,
        };
        if self.entries.len() >= self.max_entries {
            self.entries.pop_front();
        }
        self.entries.push_back(entry);
    }

    /// تسجيل إجراء مع وقت التنفيذ
    pub fn record_with_duration(
        &mut self,
        user_id: &str,
        action: &str,
        params: &serde_json::Value,
        success: bool,
        duration_ms: u64,
    ) {
        let mut entry = AuditEntry {
            user_id: user_id.to_string(),
            action: action.to_string(),
            params: self.sanitize_params(params),
            success,
            timestamp: chrono::Utc::now().to_rfc3339(),
            duration_ms: Some(duration_ms),
        };
        if self.entries.len() >= self.max_entries {
            self.entries.pop_front();
        }
        self.entries.push_back(entry);
    }

    /// إزالة المحتوى الحساس من المعاملات قبل التسجيل
    fn sanitize_params(&self, params: &serde_json::Value) -> serde_json::Value {
        let sensitive_keys = [
            "token",
            "secret",
            "password",
            "api_key",
            "api-key",
            "auth_token",
            "access_token",
            "private_key",
            "session_key",
        ];
        match params {
            serde_json::Value::Object(map) => {
                let mut clean = serde_json::Map::new();
                for (k, v) in map {
                    if sensitive_keys.contains(&k.to_lowercase().as_str()) {
                        clean.insert(k.clone(), serde_json::Value::String("***".into()));
                    } else {
                        clean.insert(k.clone(), self.sanitize_params(v));
                    }
                }
                serde_json::Value::Object(clean)
            }
            serde_json::Value::Array(arr) => {
                serde_json::Value::Array(arr.iter().map(|v| self.sanitize_params(v)).collect())
            }
            other => other.clone(),
        }
    }

    /// جلب جميع السجلات
    pub fn all(&self) -> Vec<&AuditEntry> {
        self.entries.iter().collect()
    }

    /// جلب سجلات مستخدم معين
    pub fn by_user(&self, user_id: &str) -> Vec<&AuditEntry> {
        self.entries
            .iter()
            .filter(|e| e.user_id == user_id)
            .collect()
    }

    /// جلب سجلات إجراء معين
    pub fn by_action(&self, action: &str) -> Vec<&AuditEntry> {
        self.entries.iter().filter(|e| e.action == action).collect()
    }

    /// جلب آخر N سجل
    pub fn recent(&self, n: usize) -> Vec<&AuditEntry> {
        self.entries.iter().rev().take(n).collect()
    }

    /// جلب إحصائيات مختصرة
    pub fn stats(&self) -> AuditStats {
        let total = self.entries.len();
        let success = self.entries.iter().filter(|e| e.success).count();
        let actions: std::collections::HashMap<String, usize> = {
            let mut m = std::collections::HashMap::new();
            for e in &self.entries {
                *m.entry(e.action.clone()).or_insert(0) += 1;
            }
            m
        };
        AuditStats {
            total,
            success,
            failed: total - success,
            actions,
        }
    }

    /// تصدير كـ JSON
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "entries": self.entries,
            "stats": self.stats(),
        })
    }
}

/// إحصائيات سجل التدقيق
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditStats {
    pub total: usize,
    pub success: usize,
    pub failed: usize,
    pub actions: std::collections::HashMap<String, usize>,
}

// ─── الاختبارات ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audit_log_record() {
        let mut log = AuditLog::new(100);
        let params = serde_json::json!({"code": "print('hello')", "language": "python"});
        log.record("user1", "code_exec", &params, true);
        assert_eq!(log.all().len(), 1);
        assert_eq!(log.by_user("user1").len(), 1);
    }

    #[test]
    fn test_audit_log_sanitize() {
        let mut log = AuditLog::new(100);
        let params = serde_json::json!({"api_key": "sk-1234567890abcdef", "code": "print('hi')"});
        log.record("user1", "code_exec", &params, true);
        let entry = log.all().first().unwrap();
        assert_eq!(entry.params["api_key"], "***");
        assert_eq!(entry.params["code"], "print('hi')");
    }

    #[test]
    fn test_audit_log_max_entries() {
        let mut log = AuditLog::new(5);
        for i in 0..10 {
            log.record("user1", "test", &serde_json::json!({"i": i}), true);
        }
        assert_eq!(log.all().len(), 5);
    }

    #[test]
    fn test_audit_log_stats() {
        let mut log = AuditLog::new(100);
        log.record("user1", "code_exec", &serde_json::json!({}), true);
        log.record("user1", "code_exec", &serde_json::json!({}), true);
        log.record("user1", "file_edit", &serde_json::json!({}), false);
        let stats = log.stats();
        assert_eq!(stats.total, 3);
        assert_eq!(stats.success, 2);
        assert_eq!(stats.failed, 1);
    }
}
