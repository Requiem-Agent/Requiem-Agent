//! # Strict Locks — الأقفال البرمجية الصارمة
//!
//! ## المبدأ
//! الأقفال البرمجية هي أوامر صارمة جداً تجبر نموذج الذكاء الاصطناعي على
//! الالتزام التام بقواعد البيئة بدلاً من الاعتماد على System Prompt فقط.
//!
//! ## الأقفال
//! 1. **Identity Lock** — الالتزام بهوية Requiem Agent
//! 2. **Mode Lock** — الالتزام بالوضع الحالي
//! 3. **Format Lock** — الالتزام بتنسيق JSON المطلوب
//! 4. **Model Lock** — الالتزام بنموذج AI المحدد
//! 5. **Memory Lock** — الالتزام بنظام الذاكرة والـ RAG
//! 6. **Tool Lock** — الالتزام بطريقة استخدام الأدوات
//! 7. **Workflow Lock** — الالتزام بتدفق العمل
//! 8. **Output Lock** — الالتزام بتنسيق المخرجات

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{debug, info, warn};

/// نوع القفل
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum LockType {
    /// قفل الهوية — يجبر الوكيل على التعرف على نفسه
    Identity,
    /// قفل الوضع — يجبر الوكيل على الالتزام بالوضع الحالي
    Mode,
    /// قفل التنسيق — يجبر الوكيل على استخدام JSON Schema
    Format,
    /// قفل النموذج — يجبر الوكيل على استخدام النموذج المحدد
    Model,
    /// قفل الذاكرة — يجبر الوكيل على استخدام RAG
    Memory,
    /// قفل الأداة — يجبر الوكيل على طريقة استخدام الأدوات
    Tool,
    /// قفل تدفق العمل — يجبر الوكيل على اتباع Workflow
    Workflow,
    /// قفل المخرجات — يجبر الوكيل على تنسيق المخرجات
    Output,
}

impl LockType {
    pub fn name(&self) -> &str {
        match self {
            Self::Identity => "identity",
            Self::Mode => "mode",
            Self::Format => "format",
            Self::Model => "model",
            Self::Memory => "memory",
            Self::Tool => "tool",
            Self::Workflow => "workflow",
            Self::Output => "output",
        }
    }

    pub fn description(&self) -> &str {
        match self {
            Self::Identity => "يُلزم الوكيل بالتعرف على هويته كـ Requiem Agent",
            Self::Mode => "يُلزم الوكيل بالالتزام بالوضع الحالي",
            Self::Format => "يُلزم الوكيل باستخدام JSON Schema المطلوب",
            Self::Model => "يُلزم الوكيل باستخدام النموذج المحدد",
            Self::Memory => "يُلزم الوكيل بنظام الذاكرة والـ RAG",
            Self::Tool => "يُلزم الوكيل بطريقة استخدام الأدوات",
            Self::Workflow => "يُلزم الوكيل بتدفق العمل",
            Self::Output => "يُلزم الوكيل بتنسيق المخرجات",
        }
    }
}

/// حالة القفل
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum LockState {
    /// القفل مُفعّل
    Active,
    /// القفل معطّل (مؤقتاً)
    Disabled,
    /// القفل في وضع الاختبار (يسمح بالخرق مع تسجيل)
    AuditOnly,
}

/// قفل واحد
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Lock {
    pub lock_type: LockType,
    pub state: LockState,
    pub message: String,
    pub enforcement_level: EnforcementLevel,
    pub violations: u32,
    pub max_violations_before_penalty: u32,
}

/// مستوى الت enforcement
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum EnforcementLevel {
    /// تحذير فقط
    Warn,
    /// إعادة محاولة مع تصحيح
    RetryWithCorrection,
    /// إيقاف التنفيذ
    HardStop,
    /// تغيير الوضع تلقائياً
    AutoModeSwitch,
}

/// نتيجة فحص القفل
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockCheckResult {
    pub passed: bool,
    pub violations: Vec<LockViolation>,
    pub suggested_action: String,
    pub injected_context: String,
}

/// انتهاك للقفل
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockViolation {
    pub lock_type: LockType,
    pub violation_type: String,
    pub description: String,
    pub severity: ViolationSeverity,
    pub auto_correctable: bool,
}

/// خطورة الانتهاك
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ViolationSeverity {
    Low,
    Medium,
    High,
    Critical,
}

/// محرك الأقفال الرئيسي
pub struct StrictLocksEngine {
    locks: HashMap<LockType, Lock>,
    identity_context: String,
    mode_context: String,
    format_schema: serde_json::Value,
    total_violations: u32,
}

impl StrictLocksEngine {
    /// إنشاء محرك أقفال جديد
    pub fn new() -> Self {
        info!("StrictLocksEngine initialized with {} locks", 8);

        let mut locks = HashMap::new();

        // قفل الهوية
        locks.insert(
            LockType::Identity,
            Lock {
                lock_type: LockType::Identity,
                state: LockState::Active,
                message:
                    "أنت Requiem Agent — أداة تطوير بالذكاء الاصطناعي._identify_as_requiem_agent"
                        .to_string(),
                enforcement_level: EnforcementLevel::HardStop,
                violations: 0,
                max_violations_before_penalty: 3,
            },
        );

        // قفل الوضع
        locks.insert(
            LockType::Mode,
            Lock {
                lock_type: LockType::Mode,
                state: LockState::Active,
                message: "يلزمك الالتزام بالوضع الحالي فقط".to_string(),
                enforcement_level: EnforcementLevel::RetryWithCorrection,
                violations: 0,
                max_violations_before_penalty: 5,
            },
        );

        // قفل التنسيق
        locks.insert(
            LockType::Format,
            Lock {
                lock_type: LockType::Format,
                state: LockState::Active,
                message: "يلزمك استخدام JSON Format في كل الاستجابات".to_string(),
                enforcement_level: EnforcementLevel::RetryWithCorrection,
                violations: 0,
                max_violations_before_penalty: 3,
            },
        );

        // قفل النموذج
        locks.insert(
            LockType::Model,
            Lock {
                lock_type: LockType::Model,
                state: LockState::Active,
                message: "يلزمك استخدام النموذج المحدد للمهمة الحالية".to_string(),
                enforcement_level: EnforcementLevel::AutoModeSwitch,
                violations: 0,
                max_violations_before_penalty: 2,
            },
        );

        // قفل الذاكرة
        locks.insert(
            LockType::Memory,
            Lock {
                lock_type: LockType::Memory,
                state: LockState::Active,
                message: "يلزمك استخدام نظام RAG للاسترجاع والتخزين".to_string(),
                enforcement_level: EnforcementLevel::Warn,
                violations: 0,
                max_violations_before_penalty: 10,
            },
        );

        // قفل الأداة
        locks.insert(
            LockType::Tool,
            Lock {
                lock_type: LockType::Tool,
                state: LockState::Active,
                message: "يلزمك اتباع طريقة استخدام الأدوات المحددة".to_string(),
                enforcement_level: EnforcementLevel::RetryWithCorrection,
                violations: 0,
                max_violations_before_penalty: 3,
            },
        );

        // قفل تدفق العمل
        locks.insert(
            LockType::Workflow,
            Lock {
                lock_type: LockType::Workflow,
                state: LockState::Active,
                message: "يلزمك اتباع تدفق العمل المحدد".to_string(),
                enforcement_level: EnforcementLevel::RetryWithCorrection,
                violations: 0,
                max_violations_before_penalty: 3,
            },
        );

        // قفل المخرجات
        locks.insert(
            LockType::Output,
            Lock {
                lock_type: LockType::Output,
                state: LockState::Active,
                message: "يلزمك استخدام تنسيق المخرجات المحدد".to_string(),
                enforcement_level: EnforcementLevel::RetryWithCorrection,
                violations: 0,
                max_violations_before_penalty: 3,
            },
        );

        Self {
            locks,
            identity_context: Self::build_identity_context(),
            mode_context: String::new(),
            format_schema: Self::build_format_schema(),
            total_violations: 0,
        }
    }

    /// بناء سياق الهوية
    fn build_identity_context() -> String {
        r#"## IDENTITY LOCK — مُفعّل
أنت "Requiem Agent" — أداة تطوير بالذكاء الاصطناعي.
- لا تدّعي أنك GPT أو Claude أو أي نموذج آخر
- لا تذكر أسماء النماذج الداخلية
- هويتك ثابتة ولا تتغير"#
            .to_string()
    }

    /// بناء شيما التنسيق
    fn build_format_schema() -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "required": ["action", "content", "metadata"],
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["code", "file_edit", "tool_call", "thinking", "response"]
                },
                "content": {
                    "type": "string"
                },
                "metadata": {
                    "type": "object",
                    "properties": {
                        "mode": {"type": "string"},
                        "effort": {"type": "string"},
                        "model": {"type": "string"},
                        "confidence": {"type": "number"}
                    }
                }
            }
        })
    }

    /// فحص جميع الأقفال
    pub fn check_all(
        &self,
        current_mode: &str,
        current_model: &str,
        response_format: &str,
        output: &str,
    ) -> LockCheckResult {
        let mut violations = Vec::new();
        let mut injected_context = String::new();

        // فحص قفل الهوية
        if let Some(v) = self.check_identity_lock(output) {
            violations.push(v);
        } else {
            injected_context.push_str(&self.identity_context);
            injected_context.push_str("\n\n");
        }

        // فحص قفل الوضع
        if let Some(v) = self.check_mode_lock(output, current_mode) {
            violations.push(v);
        }

        // فحص قفل التنسيق
        if let Some(v) = self.check_format_lock(output, response_format) {
            violations.push(v);
        }

        // فحص قفل النموذج
        if let Some(v) = self.check_model_lock(output, current_model) {
            violations.push(v);
        }

        // فحص قفل الذاكرة
        if let Some(v) = self.check_memory_lock(output) {
            violations.push(v);
        }

        let passed = violations.is_empty();
        let suggested_action = if passed {
            "proceed".to_string()
        } else if violations
            .iter()
            .any(|v| v.severity == ViolationSeverity::Critical)
        {
            "hard_stop_and_retry".to_string()
        } else {
            "retry_with_correction".to_string()
        };

        LockCheckResult {
            passed,
            violations,
            suggested_action,
            injected_context,
        }
    }

    /// فحص قفل الهوية
    fn check_identity_lock(&self, output: &str) -> Option<LockViolation> {
        let lower = output.to_lowercase();

        // كشف محاولات التظاهر بنموذج آخر
        let fake_identity_patterns = [
            "i am gpt",
            "i am claude",
            "i am chatgpt",
            "i am gemini",
            "أنا gpt",
            "أنا claude",
            "أنا chatgpt",
            "أنا gemini",
            "as an ai language model",
            "كنموذج لغة ذكاء اصطناعي",
        ];

        for pattern in &fake_identity_patterns {
            if lower.contains(pattern) {
                return Some(LockViolation {
                    lock_type: LockType::Identity,
                    violation_type: "fake_identity".to_string(),
                    description: format!("Attempted to claim false identity: {}", pattern),
                    severity: ViolationSeverity::Critical,
                    auto_correctable: false,
                });
            }
        }

        None
    }

    /// فحص قفل الوضع
    fn check_mode_lock(&self, output: &str, current_mode: &str) -> Option<LockViolation> {
        let lower = output.to_lowercase();

        // التحقق من أن الوكيل لا يتجاوز صلاحيات وضعه
        match current_mode {
            "supervised" => {
                if lower.contains("execute") || lower.contains("run") || lower.contains("نفّذ")
                {
                    return Some(LockViolation {
                        lock_type: LockType::Mode,
                        violation_type: "unauthorized_execution".to_string(),
                        description: "Attempted execution in supervised mode without approval"
                            .to_string(),
                        severity: ViolationSeverity::High,
                        auto_correctable: true,
                    });
                }
            }
            "tutorial" => {
                if lower.contains("i've fixed") || lower.contains("تم الإصلاح") {
                    return Some(LockViolation {
                        lock_type: LockType::Mode,
                        violation_type: "unauthorized_fix".to_string(),
                        description: "Attempted fix in tutorial mode (should only explain)"
                            .to_string(),
                        severity: ViolationSeverity::Medium,
                        auto_correctable: true,
                    });
                }
            }
            _ => {}
        }

        None
    }

    /// فحص قفل التنسيق
    fn check_format_lock(&self, output: &str, expected_format: &str) -> Option<LockViolation> {
        if expected_format == "json" {
            // التحقق من أن المخرج صالح JSON
            let trimmed = output.trim();
            if !trimmed.starts_with('{') && !trimmed.starts_with('[') {
                return Some(LockViolation {
                    lock_type: LockType::Format,
                    violation_type: "invalid_json_format".to_string(),
                    description: "Response is not in JSON format as required".to_string(),
                    severity: ViolationSeverity::Medium,
                    auto_correctable: true,
                });
            }
        }

        None
    }

    /// فحص قفل النموذج
    fn check_model_lock(&self, output: &str, current_model: &str) -> Option<LockViolation> {
        // التحقق من أن النموذج لا يذكر نموذجاً مختلفاً
        let model_names = ["gpt-4", "gpt-3.5", "claude-3", "gemini-pro", "llama-3"];

        let lower = output.to_lowercase();
        for model in &model_names {
            if lower.contains(model) && !current_model.contains(model) {
                return Some(LockViolation {
                    lock_type: LockType::Model,
                    violation_type: "wrong_model_mention".to_string(),
                    description: format!("Mentioned model {} while using {}", model, current_model),
                    severity: ViolationSeverity::Low,
                    auto_correctable: true,
                });
            }
        }

        None
    }

    /// فحص قفل الذاكرة
    fn check_memory_lock(&self, output: &str) -> Option<LockViolation> {
        let lower = output.to_lowercase();

        // التحقق من عدم نسيان استخدام RAG
        if lower.contains("i don't remember") || lower.contains("لا أتذكر") {
            return Some(LockViolation {
                lock_type: LockType::Memory,
                violation_type: "memory_not_used".to_string(),
                description: "Agent claims not to remember — should use RAG".to_string(),
                severity: ViolationSeverity::Medium,
                auto_correctable: true,
            });
        }

        None
    }

    /// تسجيل انتهاك
    pub fn record_violation(&mut self, lock_type: &LockType) {
        self.total_violations += 1;
        if let Some(lock) = self.locks.get_mut(lock_type) {
            lock.violations += 1;
            if lock.violations >= lock.max_violations_before_penalty {
                warn!(
                    "Lock {} exceeded max violations ({}/{}), triggering penalty",
                    lock_type.name(),
                    lock.violations,
                    lock.max_violations_before_penalty
                );
            }
        }
    }

    /// توليد سياق الأقفال للإدخال في البرومبت
    pub fn generate_lock_context(&self, mode: &str, effort: &str) -> String {
        let mut context = String::from("## ACTIVE LOCKS — الأقفال النشطة\n\n");

        for (lock_type, lock) in &self.locks {
            if lock.state == LockState::Active {
                context.push_str(&format!(
                    "- **{}**: {} [ Enforcement: {:?} ]\n",
                    lock_type.name(),
                    lock.message,
                    lock.enforcement_level
                ));
            }
        }

        context.push_str(&format!(
            "\n## Current Mode: {} | Effort: {}\n",
            mode, effort
        ));
        context
    }

    /// إحصائيات الأقفال
    pub fn stats(&self) -> LocksStats {
        let active = self
            .locks
            .values()
            .filter(|l| l.state == LockState::Active)
            .count();
        let total_violations: u32 = self.locks.values().map(|l| l.violations).sum();

        LocksStats {
            total_locks: self.locks.len(),
            active_locks: active,
            total_violations,
            lock_details: self
                .locks
                .iter()
                .map(|(k, v)| {
                    (
                        k.name().to_string(),
                        LockDetail {
                            state: v.state.clone(),
                            violations: v.violations,
                            enforcement: v.enforcement_level.clone(),
                        },
                    )
                })
                .collect(),
        }
    }
}

/// إحصائيات الأقفال
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocksStats {
    pub total_locks: usize,
    pub active_locks: usize,
    pub total_violations: u32,
    pub lock_details: HashMap<String, LockDetail>,
}

/// تفاصيل قفل واحد
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockDetail {
    pub state: LockState,
    pub violations: u32,
    pub enforcement: EnforcementLevel,
}

impl Default for StrictLocksEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strict_locks_engine_creation() {
        let engine = StrictLocksEngine::new();
        assert_eq!(engine.locks.len(), 8);
    }

    #[test]
    fn test_identity_lock_detection() {
        let engine = StrictLocksEngine::new();
        let result = engine.check_all("autonomous", "deepseek-v4-flash-free", "text", "I am GPT-4");
        assert!(!result.passed);
        assert!(result
            .violations
            .iter()
            .any(|v| v.lock_type == LockType::Identity));
    }

    #[test]
    fn test_clean_output_passes() {
        let engine = StrictLocksEngine::new();
        let result = engine.check_all(
            "autonomous",
            "deepseek-v4-flash-free",
            "text",
            "Hello! I can help you with that.",
        );
        assert!(result.passed);
    }

    #[test]
    fn test_lock_context_generation() {
        let engine = StrictLocksEngine::new();
        let context = engine.generate_lock_context("autonomous", "medium");
        assert!(context.contains("ACTIVE LOCKS"));
        assert!(context.contains("identity"));
    }
}
