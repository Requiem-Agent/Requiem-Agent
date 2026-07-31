// ─── Model Synergy Engine ────────────────────────────────────────────────
// Phase 16: تنسيق النماذج المتوازية — Consensus, Critique, Pipeline
// ==========================================================================

mod patterns;
mod smart_router;
mod coordinator;

pub use patterns::*;
pub use smart_router::*;
pub use coordinator::*;

use serde::{Deserialize, Serialize};

/// نتيجة تنفيذ نموذج
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelOutput {
    pub model_name: String,
    pub output: String,
    pub confidence: f64,
    pub latency_ms: u64,
    pub tokens_used: usize,
    pub success: bool,
}

/// ملخص جولة تآزر
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SynergyRound {
    pub round_id: String,
    pub pattern: SynergyPattern,
    pub models_used: Vec<String>,
    pub outputs: Vec<ModelOutput>,
    pub final_output: String,
    pub consensus_score: f64,
    pub total_latency_ms: u64,
    pub total_tokens: usize,
}

/// أنواع أنماط التآزر
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SynergyPattern {
    /// 3+ نماذج تنتج إجماعاً
    Consensus,
    /// نموذج يقيّم مخرجات آخر
    Critique,
    /// تدفق مرحلي (نموذج ← آخر)
    Pipeline,
    /// نموذجان جنباً لجنب
    Pair,
}

impl SynergyPattern {
    pub fn name(&self) -> &str {
        match self {
            Self::Consensus => "consensus",
            Self::Critique => "critique",
            Self::Pipeline => "pipeline",
            Self::Pair => "pair",
        }
    }
}

/// توزيع الأصوات في الإجماع
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsensusVote {
    pub model: String,
    pub choice: String,
    pub confidence: f64,
    pub reasoning: String,
}
