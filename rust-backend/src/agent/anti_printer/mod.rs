// ─── Anti-Printer & Semantic Analysis Module ───────────────────────────────
// Phase 15: يكشف أنماط الطباعة الفارغة والتفكير السطحي، ويحلل دلالات المهام

mod semantic;
mod patterns;
mod router;
pub mod pipeline;

pub use semantic::*;
pub use patterns::*;
pub use router::*;
pub use pipeline::*;

use serde::{Deserialize, Serialize};

/// درجة خطورة النمط المكتشف
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Severity {
    Info,
    Warning,
    Error,
    Critical,
}

/// نمط تم اكتشافه في تفكير الوكيل
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectedPattern {
    pub pattern_type: PatternType,
    pub description: String,
    pub severity: Severity,
    pub location: Option<String>,
    pub suggestion: String,
}

/// أنواع الأنماط التي يكتشفها المدقق
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum PatternType {
    OutputLessThinking,
    CircularReasoning,
    ShallowResponse,
    VerboseNoAction,
    PlanOnlyLoop,
    RepetitiveContent,
    OverConfidence,
    InstructionIgnore,
    IncompleteAnalysis,
    Custom(String),
}

impl PatternType {
    pub fn name(&self) -> &str {
        match self {
            Self::OutputLessThinking => "output_less_thinking",
            Self::CircularReasoning => "circular_reasoning",
            Self::ShallowResponse => "shallow_response",
            Self::VerboseNoAction => "verbose_no_action",
            Self::PlanOnlyLoop => "plan_only_loop",
            Self::RepetitiveContent => "repetitive_content",
            Self::OverConfidence => "over_confidence",
            Self::InstructionIgnore => "instruction_ignore",
            Self::IncompleteAnalysis => "incomplete_analysis",
            Self::Custom(s) => s,
        }
    }
    pub fn severity_default(&self) -> Severity {
        match self {
            Self::OutputLessThinking => Severity::Error,
            Self::CircularReasoning => Severity::Warning,
            Self::ShallowResponse => Severity::Warning,
            Self::VerboseNoAction => Severity::Warning,
            Self::PlanOnlyLoop => Severity::Error,
            Self::RepetitiveContent => Severity::Info,
            Self::OverConfidence => Severity::Info,
            Self::InstructionIgnore => Severity::Critical,
            Self::IncompleteAnalysis => Severity::Warning,
            Self::Custom(_) => Severity::Info,
        }
    }
}

/// نتيجة تحليل كاملة
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AntiPrinterReport {
    pub has_issues: bool,
    pub patterns: Vec<DetectedPattern>,
    pub quality_score: f64,
    pub requires_retry: bool,
    pub suggested_action: String,
}

impl AntiPrinterReport {
    pub fn clean() -> Self {
        Self {
            has_issues: false,
            patterns: vec![],
            quality_score: 1.0,
            requires_retry: false,
            suggested_action: "proceed".to_string(),
        }
    }

    pub fn merge(reports: Vec<AntiPrinterReport>) -> Self {
        let mut all_patterns = vec![];
        let mut min_score = 1.0;
        let mut any_retry = false;
        for r in reports {
            all_patterns.extend(r.patterns);
            if r.quality_score < min_score { min_score = r.quality_score; }
            if r.requires_retry { any_retry = true; }
        }
        Self {
            has_issues: !all_patterns.is_empty(),
            patterns: all_patterns,
            quality_score: min_score,
            requires_retry: any_retry,
            suggested_action: if any_retry { "retry_with_correction" } else { "proceed" }.to_string(),
        }
    }
}
