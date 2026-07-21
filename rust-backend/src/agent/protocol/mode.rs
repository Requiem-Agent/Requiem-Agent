//! # Agent Mode Protocol — التنقل بين الأوضاع برمجياً
//!
//! ## الأوضاع
//! - **Autonomous**: الوكيل يقرر بنفسه، أقل تدخل بشري
//! - **Supervised**: كل خطوة تحتاج موافقة المستخدم
//! - **Audit**: الوكيل يعمل لكن يسجل كل شيء بتفصيل
//! - **Tutorial**: الوكيل يشرح كل خطوة بالتفصيل
//! - **Turbo**: أقصى أداء، بدون تحقق زائد

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// أوضاع الوكيل
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AgentMode {
    /// الوكيل يقرر بنفسه — الحد الأعلى من الاستقلالية
    Autonomous,
    /// كل خطوة تحتاج موافقة المستخدم
    Supervised,
    /// تسجيل كامل مع تفاصيل
    Audit,
    /// شرح كل خطوة بالتفصيل
    Tutorial,
    /// أقصى أداء بدون تحقق (للمهام البسيطة)
    Turbo,
}

impl AgentMode {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Autonomous => "autonomous",
            Self::Supervised => "supervised",
            Self::Audit => "audit",
            Self::Tutorial => "tutorial",
            Self::Turbo => "turbo",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            Self::Autonomous => "وضع مستقل — الوكيل يقرر بنفسه بدون تدخل بشري",
            Self::Supervised => "وضع تحت الإشراف — كل خطوة تحتاج موافقة",
            Self::Audit => "وضع تدقيق — يسجل كل شيء مع تفاصيل دقيقة",
            Self::Tutorial => "وضع تعليمي — يشرح كل خطوة بالتفصيل",
            Self::Turbo => "وضع توربو — أقصى أداء بدون تحقق زائد",
        }
    }

    /// القيود المرتبطة بكل وضع
    pub fn constraints(&self) -> ModeConstraints {
        match self {
            Self::Autonomous => ModeConstraints {
                require_approval: false,
                max_consecutive_steps: 25,
                require_reasoning: false,
                require_tool_selection: true,
                audit_level: AuditLevel::Normal,
                thinking_mode: crate::agent::protocol::thinking::ProtocolMode::Moderate,
                max_sub_agents: 5,
            },
            Self::Supervised => ModeConstraints {
                require_approval: true,
                max_consecutive_steps: 1,
                require_reasoning: true,
                require_tool_selection: true,
                audit_level: AuditLevel::Detailed,
                thinking_mode: crate::agent::protocol::thinking::ProtocolMode::Strict,
                max_sub_agents: 2,
            },
            Self::Audit => ModeConstraints {
                require_approval: false,
                max_consecutive_steps: 10,
                require_reasoning: true,
                require_tool_selection: true,
                audit_level: AuditLevel::Full,
                thinking_mode: crate::agent::protocol::thinking::ProtocolMode::Strict,
                max_sub_agents: 3,
            },
            Self::Tutorial => ModeConstraints {
                require_approval: false,
                max_consecutive_steps: 5,
                require_reasoning: true,
                require_tool_selection: true,
                audit_level: AuditLevel::Detailed,
                thinking_mode: crate::agent::protocol::thinking::ProtocolMode::Strict,
                max_sub_agents: 1,
            },
            Self::Turbo => ModeConstraints {
                require_approval: false,
                max_consecutive_steps: 50,
                require_reasoning: false,
                require_tool_selection: false,
                audit_level: AuditLevel::Minimal,
                thinking_mode: crate::agent::protocol::thinking::ProtocolMode::Disabled,
                max_sub_agents: 10,
            },
        }
    }
}

/// القيود المفروضة على الوكيل حسب وضعه
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModeConstraints {
    /// هل كل خطوة تحتاج موافقة المستخدم؟
    pub require_approval: bool,
    /// أقصى عدد من الخطوات المتتالية بدون موافقة
    pub max_consecutive_steps: u32,
    /// هل يجب على الوكيل تقديم reasoning مع كل إجراء؟
    pub require_reasoning: bool,
    /// هل يجب اختيار أداة قبل التنفيذ؟
    pub require_tool_selection: bool,
    /// مستوى التدقيق
    pub audit_level: AuditLevel,
    /// وضع التفكير المطلوب
    pub thinking_mode: crate::agent::protocol::thinking::ProtocolMode,
    /// أقصى عدد من الوكلاء الفرعيين
    pub max_sub_agents: usize,
}

/// مستوى التدقيق
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AuditLevel {
    /// فقط الأحداث المهمة
    Minimal,
    /// الأحداث العادية
    Normal,
    /// تفاصيل أكثر
    Detailed,
    /// كل شيء — سجل كامل
    Full,
}

// ─── Mode Controller ───────────────────────────────────────────────────────

/// يتحكم في وضع الوكيل ويطبق القيود
pub struct ModeController {
    current: AgentMode,
    history: Vec<ModeChange>,
    max_history: usize,
}

/// تسجيل تغيير وضع
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModeChange {
    pub from: AgentMode,
    pub to: AgentMode,
    pub reason: String,
    pub timestamp: String,
    pub triggered_by: String, // "user" | "agent" | "system" | "automatic"
}

impl ModeController {
    pub fn new(initial: AgentMode) -> Self {
        Self {
            current: initial,
            history: Vec::new(),
            max_history: 100,
        }
    }

    pub fn current(&self) -> AgentMode { self.current }
    pub fn constraints(&self) -> ModeConstraints { self.current.constraints() }
    pub fn history(&self) -> &[ModeChange] { &self.history }

    /// تغيير الوضع — يسجل التغيير ويعيد القيود الجديدة
    pub fn switch(&mut self, new_mode: AgentMode, reason: &str, triggered_by: &str) -> ModeConstraints {
        let old = self.current;
        self.current = new_mode;

        if self.history.len() >= self.max_history {
            self.history.remove(0);
        }
        self.history.push(ModeChange {
            from: old,
            to: new_mode,
            reason: reason.to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            triggered_by: triggered_by.to_string(),
        });

        new_mode.constraints()
    }

    /// اقتراح الوضع المناسب بناءً على المهمة
    pub fn suggest_mode(task: &str) -> AgentMode {
        let lower = task.to_lowercase();
        if lower.contains("urgent") || lower.contains("سريع") || lower.contains("quick")
            || lower.contains("simple") || lower.contains("بسيط") {
            AgentMode::Turbo
        } else if lower.contains("تعلم") || lower.contains("learn") || lower.contains("أتعلم")
            || lower.contains("شرح") || lower.contains("explain") {
            AgentMode::Tutorial
        } else if lower.contains("audit") || lower.contains("تدقيق") || lower.contains("مراجعة")
            || lower.contains("review") || lower.contains("فحص") {
            AgentMode::Audit
        } else if lower.contains("danger") || lower.contains("delete") || lower.contains("حذف")
            || lower.contains("rm ") || lower.contains("drop") || lower.contains("format") {
            AgentMode::Supervised
        } else {
            AgentMode::Autonomous
        }
    }
}

// ─── اختبارات ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mode_constraints() {
        assert!(AgentMode::Supervised.constraints().require_approval);
        assert!(!AgentMode::Turbo.constraints().require_approval);

        assert_eq!(AgentMode::Turbo.constraints().max_consecutive_steps, 50);
        assert_eq!(AgentMode::Supervised.constraints().max_consecutive_steps, 1);
    }

    #[test]
    fn test_mode_controller_switch() {
        let mut mc = ModeController::new(AgentMode::Autonomous);
        assert_eq!(mc.current(), AgentMode::Autonomous);

        mc.switch(AgentMode::Supervised, "مهمة خطيرة", "agent");
        assert_eq!(mc.current(), AgentMode::Supervised);
        assert!(mc.constraints().require_approval);

        assert_eq!(mc.history().len(), 1);
        assert_eq!(mc.history()[0].reason, "مهمة خطيرة");
    }

    #[test]
    fn test_suggest_mode() {
        assert_eq!(ModeController::suggest_mode("quick fix"), AgentMode::Turbo);
        assert_eq!(ModeController::suggest_mode("تعلم Rust"), AgentMode::Tutorial);
        assert_eq!(ModeController::suggest_mode("audit the code"), AgentMode::Audit);
        assert_eq!(ModeController::suggest_mode("حذف قاعدة البيانات"), AgentMode::Supervised);
        assert_eq!(ModeController::suggest_mode("اكتب تطبيق ويب"), AgentMode::Autonomous);
    }

    #[test]
    fn test_audit_levels() {
        use crate::agent::protocol::thinking::ProtocolMode;
        assert_eq!(AgentMode::Audit.constraints().audit_level, AuditLevel::Full);
        assert_eq!(AgentMode::Audit.constraints().thinking_mode, ProtocolMode::Strict);
        assert_eq!(AgentMode::Turbo.constraints().thinking_mode, ProtocolMode::Disabled);
    }
}
