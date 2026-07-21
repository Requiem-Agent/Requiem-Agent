//! # Lock Interpreter — مترجم الأقفال البرمجية
//!
//! ## المبدأ
//! الانتربرينتر مسؤول عن التزام الوكيل بالاقفال والتبعيات
//! ورصد الأخطاء المنطقية والتبعية أو الانجراف من القواعد.
//!
//! ## المكونات
//! 1. **RuleEnforcer** — فرض القواعد
//! 2. **DriftDetector** — كشف الانجراف
//! 3. **ComplianceChecker** — فحص الالتزام
//! 4. **AutoRecovery** — الاسترداد التلقائي

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{debug, info, warn};

/// حالة الالتزام
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ComplianceStatus {
    /// ملتزم بالكامل
    FullyCompliant,
    /// ملتزم جزئياً
    PartiallyCompliant,
    /// غير ملتزم
    NonCompliant,
    /// في خطر (قد ينحرف قريباً)
    AtRisk,
}

/// كشف الانجراف
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriftDetection {
    pub detected: bool,
    pub drift_type: DriftType,
    pub severity: DriftSeverity,
    pub description: String,
    pub correction_suggestion: String,
}

/// نوع الانجراف
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum DriftType {
    /// انجراف عن الهوية
    Identity,
    /// انجراف عن الوضع
    Mode,
    /// انجراف عن النموذج
    Model,
    /// انجراف عن التنسيق
    Format,
    /// انجراف عن تدفق العمل
    Workflow,
    /// انجراف عن الذاكرة
    Memory,
}

/// خطورة الانجراف
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum DriftSeverity {
    Low,
    Medium,
    High,
    Critical,
}

/// قاعدة فرض
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnforcementRule {
    pub rule_id: String,
    pub name: String,
    pub description: String,
    pub condition: String,
    pub action: EnforcementAction,
    pub enabled: bool,
}

/// إجراء الفرض
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EnforcementAction {
    Warn,
    Block,
    RetryWithCorrection,
    AutoModeSwitch,
    LogAndContinue,
}

/// مترجم الأقفال الرئيسي
pub struct LockInterpreter {
    rules: Vec<EnforcementRule>,
    compliance_history: Vec<ComplianceRecord>,
    max_history: usize,
    drift_threshold: f32,
}

/// سجل الالتزام
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceRecord {
    pub timestamp: String,
    pub status: ComplianceStatus,
    pub score: f32,
    pub violations: Vec<String>,
    pub corrections_applied: u32,
}

impl LockInterpreter {
    /// إنشاء مترجم جديد
    pub fn new() -> Self {
        info!("LockInterpreter initialized");

        let rules = vec![
            EnforcementRule {
                rule_id: "identity_check".to_string(),
                name: "Identity Verification".to_string(),
                description: "Verify agent identity in every response".to_string(),
                condition: "response.contains_identity_claim()".to_string(),
                action: EnforcementAction::Block,
                enabled: true,
            },
            EnforcementRule {
                rule_id: "mode_compliance".to_string(),
                name: "Mode Compliance".to_string(),
                description: "Ensure agent stays within mode permissions".to_string(),
                condition: "response.exceeds_mode_permissions()".to_string(),
                action: EnforcementAction::RetryWithCorrection,
                enabled: true,
            },
            EnforcementRule {
                rule_id: "format_enforcement".to_string(),
                name: "Format Enforcement".to_string(),
                description: "Ensure response follows JSON format".to_string(),
                condition: "!response.is_valid_json()".to_string(),
                action: EnforcementAction::RetryWithCorrection,
                enabled: true,
            },
            EnforcementRule {
                rule_id: "memory_usage".to_string(),
                name: "Memory Usage".to_string(),
                description: "Ensure RAG memory is used when available".to_string(),
                condition: "response.claims_no_memory()".to_string(),
                action: EnforcementAction::LogAndContinue,
                enabled: true,
            },
            EnforcementRule {
                rule_id: "tool_usage".to_string(),
                name: "Tool Usage Protocol".to_string(),
                description: "Ensure tools are used correctly".to_string(),
                condition: "tool_call.is_invalid()".to_string(),
                action: EnforcementAction::RetryWithCorrection,
                enabled: true,
            },
        ];

        Self {
            rules,
            compliance_history: Vec::new(),
            max_history: 1000,
            drift_threshold: 0.3,
        }
    }

    /// فحص الالتزام الكامل
    pub fn check_compliance(
        &self,
        agent_output: &str,
        expected_mode: &str,
        expected_model: &str,
        context: &str,
    ) -> ComplianceCheckResult {
        let mut violations = Vec::new();
        let mut drift_detected = Vec::new();

        // فحص الهوية
        if let Some(drift) = self.check_identity_drift(agent_output) {
            if drift.detected {
                violations.push(format!("Identity drift: {}", drift.description));
                drift_detected.push(drift);
            }
        }

        // فحص الوضع
        if let Some(drift) = self.check_mode_drift(agent_output, expected_mode) {
            if drift.detected {
                violations.push(format!("Mode drift: {}", drift.description));
                drift_detected.push(drift);
            }
        }

        // فحص النموذج
        if let Some(drift) = self.check_model_drift(agent_output, expected_model) {
            if drift.detected {
                violations.push(format!("Model drift: {}", drift.description));
                drift_detected.push(drift);
            }
        }

        // فحص التنسيق
        if let Some(drift) = self.check_format_drift(agent_output) {
            if drift.detected {
                violations.push(format!("Format drift: {}", drift.description));
                drift_detected.push(drift);
            }
        }

        // فحص تدفق العمل
        if let Some(drift) = self.check_workflow_drift(agent_output, context) {
            if drift.detected {
                violations.push(format!("Workflow drift: {}", drift.description));
                drift_detected.push(drift);
            }
        }

        // حساب درجة الالتزام
        let total_checks = 5.0;
        let violations_count = violations.len() as f32;
        let score = (total_checks - violations_count) / total_checks;

        let status = if violations.is_empty() {
            ComplianceStatus::FullyCompliant
        } else if score > 0.7 {
            ComplianceStatus::PartiallyCompliant
        } else if score > 0.3 {
            ComplianceStatus::AtRisk
        } else {
            ComplianceStatus::NonCompliant
        };

        ComplianceCheckResult {
            status: status.clone(),
            score,
            violations,
            drift_detected,
            suggested_action: self.determine_action(&status),
        }
    }

    /// فحص انجراف الهوية
    fn check_identity_drift(&self, output: &str) -> Option<DriftDetection> {
        let lower = output.to_lowercase();
        let fake_identities = ["i am gpt", "i am claude", "i am chatgpt", "i am gemini"];

        for identity in &fake_identities {
            if lower.contains(identity) {
                return Some(DriftDetection {
                    detected: true,
                    drift_type: DriftType::Identity,
                    severity: DriftSeverity::Critical,
                    description: format!("Agent claimed false identity: {}", identity),
                    correction_suggestion:
                        "Remove false identity claim and reaffirm Requiem Agent identity"
                            .to_string(),
                });
            }
        }

        None
    }

    /// فحص انجراف الوضع
    fn check_mode_drift(&self, output: &str, expected_mode: &str) -> Option<DriftDetection> {
        let lower = output.to_lowercase();

        match expected_mode {
            "supervised" => {
                if lower.contains("executed")
                    || lower.contains("ran command")
                    || lower.contains("تم التنفيذ")
                {
                    return Some(DriftDetection {
                        detected: true,
                        drift_type: DriftType::Mode,
                        severity: DriftSeverity::High,
                        description: "Agent executed action in supervised mode without approval"
                            .to_string(),
                        correction_suggestion: "Wait for user approval before executing"
                            .to_string(),
                    });
                }
            }
            "tutorial" => {
                if lower.contains("i've fixed") || lower.contains("تم الإصلاح") {
                    return Some(DriftDetection {
                        detected: true,
                        drift_type: DriftType::Mode,
                        severity: DriftSeverity::Medium,
                        description:
                            "Agent performed action in tutorial mode (should only explain)"
                                .to_string(),
                        correction_suggestion: "Provide explanation only, do not make changes"
                            .to_string(),
                    });
                }
            }
            _ => {}
        }

        None
    }

    /// فحص انجراف النموذج
    fn check_model_drift(&self, output: &str, expected_model: &str) -> Option<DriftDetection> {
        let lower = output.to_lowercase();
        let model_mentions = ["gpt-4", "gpt-3.5", "claude-3", "gemini-pro"];

        for model in &model_mentions {
            if lower.contains(model) && !expected_model.contains(model) {
                return Some(DriftDetection {
                    detected: true,
                    drift_type: DriftType::Model,
                    severity: DriftSeverity::Low,
                    description: format!(
                        "Mentioned model {} while using {}",
                        model, expected_model
                    ),
                    correction_suggestion: "Avoid mentioning other model names".to_string(),
                });
            }
        }

        None
    }

    /// فحص انجراف التنسيق
    fn check_format_drift(&self, output: &str) -> Option<DriftDetection> {
        let trimmed = output.trim();
        if !trimmed.starts_with('{') && !trimmed.starts_with('[') && trimmed.len() > 100 {
            return Some(DriftDetection {
                detected: true,
                drift_type: DriftType::Format,
                severity: DriftSeverity::Medium,
                description: "Response is not in JSON format".to_string(),
                correction_suggestion: "Convert response to JSON format".to_string(),
            });
        }

        None
    }

    /// فحص انجراف تدفق العمل
    fn check_workflow_drift(&self, output: &str, context: &str) -> Option<DriftDetection> {
        // فحص بسيط — إذا كان السياق يحتوي على خطوات لم يتم اتباعها
        if context.contains("Step 1") && !output.contains("Step 1") && !output.contains("completed")
        {
            return Some(DriftDetection {
                detected: true,
                drift_type: DriftType::Workflow,
                severity: DriftSeverity::Medium,
                description: "Agent may have skipped workflow steps".to_string(),
                correction_suggestion: "Ensure all required steps are completed".to_string(),
            });
        }

        None
    }

    /// تحديد الإجراء المناسب
    fn determine_action(&self, status: &ComplianceStatus) -> String {
        match status {
            ComplianceStatus::FullyCompliant => "proceed".to_string(),
            ComplianceStatus::PartiallyCompliant => "log_and_continue".to_string(),
            ComplianceStatus::AtRisk => "retry_with_correction".to_string(),
            ComplianceStatus::NonCompliant => "hard_stop".to_string(),
        }
    }

    /// تسجيل سجل االتزام
    pub fn record_compliance(&mut self, record: ComplianceRecord) {
        if self.compliance_history.len() >= self.max_history {
            self.compliance_history.remove(0);
        }
        self.compliance_history.push(record);
    }

    /// إحصائيات الالتزام
    pub fn stats(&self) -> InterpreterStats {
        let total = self.compliance_history.len();
        let compliant = self
            .compliance_history
            .iter()
            .filter(|r| r.status == ComplianceStatus::FullyCompliant)
            .count();

        InterpreterStats {
            total_checks: total,
            fully_compliant: compliant,
            compliance_rate: if total > 0 {
                compliant as f32 / total as f32
            } else {
                1.0
            },
            avg_score: if total > 0 {
                self.compliance_history.iter().map(|r| r.score).sum::<f32>() / total as f32
            } else {
                1.0
            },
            total_violations: self
                .compliance_history
                .iter()
                .map(|r| r.violations.len() as u32)
                .sum(),
        }
    }
}

/// نتيجة فحص الالتزام
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceCheckResult {
    pub status: ComplianceStatus,
    pub score: f32,
    pub violations: Vec<String>,
    pub drift_detected: Vec<DriftDetection>,
    pub suggested_action: String,
}

/// إحصائيات المترجم
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterpreterStats {
    pub total_checks: usize,
    pub fully_compliant: usize,
    pub compliance_rate: f32,
    pub avg_score: f32,
    pub total_violations: u32,
}

impl Default for LockInterpreter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lock_interpreter_creation() {
        let interpreter = LockInterpreter::new();
        assert_eq!(interpreter.rules.len(), 5);
    }

    #[test]
    fn test_identity_drift_detection() {
        let interpreter = LockInterpreter::new();
        let result = interpreter.check_compliance(
            "I am GPT-4 and I will help you",
            "autonomous",
            "deepseek-v4-flash-free",
            "",
        );
        assert_eq!(result.status, ComplianceStatus::NonCompliant);
    }

    #[test]
    fn test_compliant_output() {
        let interpreter = LockInterpreter::new();
        let result = interpreter.check_compliance(
            "Hello! I can help you with your task.",
            "autonomous",
            "deepseek-v4-flash-free",
            "",
        );
        assert_eq!(result.status, ComplianceStatus::FullyCompliant);
    }
}
