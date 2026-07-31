//! # Structured Thinking Protocol (STP)
//!
//! يُجبر الوكيل على المرور بـ 4 مراحل تفكير إجبارية قبل التنفيذ:
//! 1. **SituationAnalysis** — تحليل الموقف وفهم البيئة
//! 2. **HypothesisGeneration** — طرح الفرضيات والحلول الممكنة
//! 3. **SolutionEvaluation** — تقييم كل حل ضد المعايير
//! 4. **ExecWithReasoning** — التنفيذ مع تبرير كامل
//!
//! الفرق عن system prompt: هذا protocol برمجي — إذا لم يمر الوكيل
//! بكل المراحل، الـ runtime يرفض التنفيذ ويعيد خطأ.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, Instant};

// ─── المراحل ───────────────────────────────────────────────────────────────

/// مراحل التفكير الإجبارية
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ThinkingStage {
    /// تحليل الموقف — ماذا يحدث؟ ما هي البيئة؟ من المستخدم؟
    SituationAnalysis,
    /// طرح الفرضيات — ما الحلول الممكنة؟ ماذا جربت من قبل؟
    HypothesisGeneration,
    /// تقييم الحلول — أي حل هو الأفضل؟ لماذا؟ ما المخاطر؟
    SolutionEvaluation,
    /// التنفيذ مع تبرير — لماذا اخترت هذا الحل؟ كيف ستنفذه؟
    ExecWithReasoning,
}

impl ThinkingStage {
    pub fn name(&self) -> &'static str {
        match self {
            Self::SituationAnalysis => "situation_analysis",
            Self::HypothesisGeneration => "hypothesis_generation",
            Self::SolutionEvaluation => "solution_evaluation",
            Self::ExecWithReasoning => "exec_with_reasoning",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            Self::SituationAnalysis => {
                "تحليل الموقف: فهم البيئة، المستخدم، السياق، الموارد المتاحة"
            }
            Self::HypothesisGeneration => "طرح الفرضيات: جميع الحلول الممكنة للمشكلة",
            Self::SolutionEvaluation => "تقييم الحلول: مقارنة الفرضيات ضد المعايير",
            Self::ExecWithReasoning => "التنفيذ المبرر: تنفيذ الحل مع شرح كل قرار",
        }
    }

    /// هل يمكن تخطي هذه المرحلة في وضع Turbo؟
    pub fn skippable_in_turbo(&self) -> bool {
        matches!(self, Self::HypothesisGeneration)
    }
}

// ─── Thinking Step ──────────────────────────────────────────────────────────

/// خطوة تفكير واحدة — ينتجها الوكيل في كل مرحلة
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThinkingStep {
    pub stage: ThinkingStage,
    pub reasoning: String,
    pub confidence: f32, // 0.0 - 1.0
    pub tokens_used: u32,
    pub duration_ms: u64,
    pub artifacts: Vec<Artifact>,      // روابط، كود، صور
    pub tools_considered: Vec<String>, // أدوات فكر في استخدامها
    pub selected_tool: Option<String>, // الأداة التي اختارها
}

/// قطعة أثرية أنتجها الوكيل خلال التفكير
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Artifact {
    pub kind: ArtifactKind,
    pub content: String,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ArtifactKind {
    Code,
    Svg,
    Mermaid,
    Text,
    Json,
    Link,
}

// ─── Thinking Protocol ─────────────────────────────────────────────────────

/// نتيجة التحقق من التفكير
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThinkingValidation {
    pub valid: bool,
    pub completed_stages: Vec<ThinkingStage>,
    pub missing_stages: Vec<ThinkingStage>,
    pub total_steps: usize,
    pub total_duration_ms: u64,
    pub average_confidence: f32,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
}

/// انتهاك البروتوكول — يمنع التنفيذ
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtocolViolation {
    pub stage: Option<ThinkingStage>,
    pub code: ViolationCode,
    pub message: String,
    pub suggestion: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ViolationCode {
    MissingStage,
    EmptyReasoning,
    LowConfidence,
    StageOrder,
    NoToolSelected,
    ToxicOutput,
    Timeout,
}

impl std::fmt::Display for ViolationCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

// ─── الـ STP الرئيسي ──────────────────────────────────────────────────────

/// Structured Thinking Protocol — يتحكم في طريقة تفكير الوكيل
pub struct ThinkingProtocol {
    pub required_stages: Vec<ThinkingStage>,
    pub min_confidence: f32,
    pub max_step_duration: Duration,
    pub require_tool_selection: bool,
    pub stages_completed: Vec<ThinkingStep>,
    pub start_time: Option<Instant>,
    pub mode: ProtocolMode,
}

/// مرونة البروتوكول
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProtocolMode {
    /// صارم — كل المراحل مطلوبة، كل خطوة تحتاج confidence > 0.5
    Strict,
    /// معتدل — المراحل مطلوبة لكن يمكنAccepting low confidence
    Moderate,
    /// خفيف — يطلب المراحل لكن لا يمنع التنفيذ
    Relaxed,
    /// التوقي — بدون تحقق (للـ Turbo mode)
    Disabled,
}

impl Default for ThinkingProtocol {
    fn default() -> Self {
        Self {
            required_stages: vec![
                ThinkingStage::SituationAnalysis,
                ThinkingStage::HypothesisGeneration,
                ThinkingStage::SolutionEvaluation,
                ThinkingStage::ExecWithReasoning,
            ],
            min_confidence: 0.5,
            max_step_duration: Duration::from_secs(120),
            require_tool_selection: true,
            stages_completed: Vec::new(),
            start_time: None,
            mode: ProtocolMode::Moderate,
        }
    }
}

impl ThinkingProtocol {
    pub fn new(mode: ProtocolMode) -> Self {
        let mut p = Self::default();
        p.mode = mode;
        match mode {
            ProtocolMode::Strict => {
                p.min_confidence = 0.7;
                p.require_tool_selection = true;
            }
            ProtocolMode::Moderate => {
                p.min_confidence = 0.4;
                p.require_tool_selection = true;
            }
            ProtocolMode::Relaxed => {
                p.min_confidence = 0.2;
                p.require_tool_selection = false;
            }
            ProtocolMode::Disabled => {
                p.required_stages = vec![];
                p.min_confidence = 0.0;
                p.require_tool_selection = false;
            }
        }
        p
    }

    /// بدء جلسة تفكير جديدة
    pub fn start_session(&mut self) {
        self.stages_completed.clear();
        self.start_time = Some(Instant::now());
    }

    /// تسجيل خطوة تفكير — يتحقق من الصحة والترتيب
    pub fn record_step(&mut self, step: ThinkingStep) -> Result<(), ProtocolViolation> {
        // تحقق من الترتيب
        if let Some(last) = self.stages_completed.last() {
            let last_idx = self.required_stages.iter().position(|s| *s == last.stage);
            let curr_idx = self.required_stages.iter().position(|s| *s == step.stage);
            match (last_idx, curr_idx) {
                (Some(li), Some(ci)) if ci < li => {
                    return Err(ProtocolViolation {
                        stage: Some(step.stage),
                        code: ViolationCode::StageOrder,
                        message: format!(
                            "ترتيب المراحل: {} قبل {}. الترتيب الصحيح: {:?}",
                            step.stage.name(),
                            last.stage.name(),
                            self.required_stages
                                .iter()
                                .map(|s| s.name())
                                .collect::<Vec<_>>()
                        ),
                        suggestion: "اتبع الترتيب: تحليل → فرضيات → تقييم → تنفيذ".into(),
                    });
                }
                _ => {}
            }
        }

        // تحقق من confidence
        if self.mode != ProtocolMode::Disabled && step.confidence < self.min_confidence {
            return Err(ProtocolViolation {
                stage: Some(step.stage),
                code: ViolationCode::LowConfidence,
                message: format!(
                    "ثقة منخفضة: {:.2} (الحد الأدنى: {:.2}) في مرحلة {}",
                    step.confidence,
                    self.min_confidence,
                    step.stage.name()
                ),
                suggestion: "اجمع معلومات إضافية أو استخدم model أكثر قدرة".into(),
            });
        }

        // تحقق من اختيار أداة في مرحلة التنفيذ
        if self.require_tool_selection
            && step.stage == ThinkingStage::ExecWithReasoning
            && step.selected_tool.is_none()
        {
            return Err(ProtocolViolation {
                stage: Some(step.stage),
                code: ViolationCode::NoToolSelected,
                message: "مرحلة التنفيذ تتطلب اختيار أداة".into(),
                suggestion: "اختر أداة من Tool Registry قبل التنفيذ".into(),
            });
        }

        self.stages_completed.push(step);
        Ok(())
    }

    /// التحقق النهائي من إتمام جميع المراحل
    pub fn validate_session(&self) -> ThinkingValidation {
        let completed: Vec<ThinkingStage> = self.stages_completed.iter().map(|s| s.stage).collect();

        let missing: Vec<ThinkingStage> = self
            .required_stages
            .iter()
            .filter(|s| !completed.contains(s))
            .copied()
            .collect();

        let total_duration: u64 = self.stages_completed.iter().map(|s| s.duration_ms).sum();

        let avg_conf = if self.stages_completed.is_empty() {
            0.0
        } else {
            self.stages_completed
                .iter()
                .map(|s| s.confidence)
                .sum::<f32>()
                / self.stages_completed.len() as f32
        };

        let valid = missing.is_empty()
            && (!self.require_tool_selection
                || self
                    .stages_completed
                    .iter()
                    .any(|s| s.selected_tool.is_some()));

        let mut warnings = Vec::new();
        if avg_conf < self.min_confidence {
            warnings.push(format!("متوسط الثقة منخفض: {:.2}", avg_conf));
        }
        if self.stages_completed.iter().any(|s| s.tokens_used < 10) {
            warnings.push("بعض خطوات التفكير قصيرة جداً (< 10 توكن)".into());
        }

        ThinkingValidation {
            valid,
            completed_stages: completed,
            missing_stages: missing.clone(),
            total_steps: self.stages_completed.len(),
            total_duration_ms: total_duration,
            average_confidence: avg_conf,
            warnings,
            errors: missing
                .iter()
                .map(|s| format!("المرحلة المفقودة: {}", s.name()))
                .collect(),
        }
    }

    /// تقرير كامل عن جلسة التفكير
    pub fn report(&self) -> serde_json::Value {
        serde_json::json!({
            "mode": format!("{:?}", self.mode),
            "required_stages": self.required_stages.iter().map(|s| s.name()).collect::<Vec<_>>(),
            "completed": self.validate_session(),
            "steps": self.stages_completed,
        })
    }
}

// ─── اختبارات ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_protocol_accepts_valid_sequence() {
        let mut p = ThinkingProtocol::new(ProtocolMode::Moderate);
        p.start_session();

        assert!(p
            .record_step(ThinkingStep {
                stage: ThinkingStage::SituationAnalysis,
                reasoning: "المستخدم يطلب تطبيق ويب، البيئة: Rust backend مع Axum".into(),
                confidence: 0.9,
                tokens_used: 50,
                duration_ms: 100,
                artifacts: vec![],
                tools_considered: vec!["code_editor".into()],
                selected_tool: None,
            })
            .is_ok());

        assert!(p
            .record_step(ThinkingStep {
                stage: ThinkingStage::HypothesisGeneration,
                reasoning: "يمكن استخدام Tower للـ middleware أو Axum layers".into(),
                confidence: 0.7,
                tokens_used: 40,
                duration_ms: 80,
                artifacts: vec![],
                tools_considered: vec!["code_editor".into(), "shell".into()],
                selected_tool: None,
            })
            .is_ok());

        assert!(p
            .record_step(ThinkingStep {
                stage: ThinkingStage::SolutionEvaluation,
                reasoning: "Axum layers أفضل لأنها مدمجة في الإطار".into(),
                confidence: 0.85,
                tokens_used: 30,
                duration_ms: 60,
                artifacts: vec![],
                tools_considered: vec!["code_editor".into()],
                selected_tool: None,
            })
            .is_ok());

        assert!(p
            .record_step(ThinkingStep {
                stage: ThinkingStage::ExecWithReasoning,
                reasoning: "سأضيف middleware layer في main.rs".into(),
                confidence: 0.9,
                tokens_used: 20,
                duration_ms: 50,
                artifacts: vec![],
                tools_considered: vec!["code_editor".into()],
                selected_tool: Some("code_editor".into()),
            })
            .is_ok());

        let v = p.validate_session();
        assert!(v.valid);
        assert!(v.missing_stages.is_empty());
    }

    #[test]
    fn test_protocol_rejects_wrong_order() {
        let mut p = ThinkingProtocol::new(ProtocolMode::Strict);
        p.start_session();

        // يبدأ بالتنفيذ قبل التحليل — مرفوض
        let result = p.record_step(ThinkingStep {
            stage: ThinkingStage::ExecWithReasoning,
            reasoning: "سأعدل الملف".into(),
            confidence: 0.9,
            tokens_used: 10,
            duration_ms: 50,
            artifacts: vec![],
            tools_considered: vec![],
            selected_tool: Some("code_editor".into()),
        });
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err().code,
            ViolationCode::StageOrder
        ));
    }

    #[test]
    fn test_protocol_rejects_low_confidence() {
        let mut p = ThinkingProtocol::new(ProtocolMode::Strict);
        p.start_session();

        let result = p.record_step(ThinkingStep {
            stage: ThinkingStage::SituationAnalysis,
            reasoning: "لا أعرف".into(),
            confidence: 0.2,
            tokens_used: 5,
            duration_ms: 10,
            artifacts: vec![],
            tools_considered: vec![],
            selected_tool: None,
        });
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err().code,
            ViolationCode::LowConfidence
        ));
    }

    #[test]
    fn test_protocol_rejects_missing_stages() {
        let mut p = ThinkingProtocol::new(ProtocolMode::Strict);
        p.start_session();

        p.record_step(ThinkingStep {
            stage: ThinkingStage::SituationAnalysis,
            reasoning: "تحليل".into(),
            confidence: 0.8,
            tokens_used: 20,
            duration_ms: 50,
            artifacts: vec![],
            tools_considered: vec![],
            selected_tool: None,
        })
        .ok();

        let v = p.validate_session();
        assert!(!v.valid);
        assert_eq!(v.missing_stages.len(), 3);
    }

    #[test]
    fn test_disabled_mode_passes_anything() {
        let mut p = ThinkingProtocol::new(ProtocolMode::Disabled);
        p.start_session();
        // بدون أي خطوات، يجب أن يكون صالحاً
        let v = p.validate_session();
        assert!(v.valid);
    }
}
