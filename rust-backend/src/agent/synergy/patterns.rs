// ─── Synergy Patterns ────────────────────────────────────────────────────
// Phase 16.1: Consensus + Critique + Pipeline

use super::{ModelOutput, SynergyRound, SynergyPattern, ConsensusVote};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ─── Consensus Pattern ──────────────────────────────────────────────────

/// معامل الإجماع
#[derive(Debug, Clone)]
pub struct ConsensusPattern {
    /// الحد الأدنى لعدد النماذج
    pub min_models: usize,
    /// عتبة الإجماع (0.0 - 1.0)
    pub consensus_threshold: f64,
    /// هل نطلب شرحاً من كل نموذج؟
    pub require_reasoning: bool,
}

impl Default for ConsensusPattern {
    fn default() -> Self {
        Self {
            min_models: 3,
            consensus_threshold: 0.6,
            require_reasoning: true,
        }
    }
}

impl ConsensusPattern {
    pub fn new() -> Self { Self::default() }

    /// تشغيل جولة إجماع
    pub fn execute(&self, _question: &str, outputs: &[ModelOutput]) -> ConsensusResult {
        if outputs.len() < self.min_models {
            return ConsensusResult::InsufficientModels {
                required: self.min_models,
                got: outputs.len(),
            };
        }

        // تحليل المخرجات وتجميعها
        let mut choice_groups: HashMap<String, Vec<&ModelOutput>> = HashMap::new();
        for output in outputs {
            let choice = self.extract_choice(&output.output);
            choice_groups.entry(choice).or_default().push(output);
        }

        // إيجاد الإجماع
        let best_group = choice_groups.iter()
            .max_by_key(|(_, group)| group.len());
        let total = outputs.len() as f64;

        match best_group {
            Some((consensus_choice, group)) => {
                let agreement = group.len() as f64 / total;
                if agreement >= self.consensus_threshold {
                    let avg_confidence = group.iter().map(|o| o.confidence).sum::<f64>() / group.len() as f64;
                    let votes: Vec<ConsensusVote> = outputs.iter().map(|o| ConsensusVote {
                        model: o.model_name.clone(),
                        choice: self.extract_choice(&o.output),
                        confidence: o.confidence,
                        reasoning: if self.require_reasoning { o.output.chars().take(200).collect() } else { String::new() },
                    }).collect();

                    ConsensusResult::Agreed(ConsensusAgreement {
                        consensus_choice: consensus_choice.clone(),
                        agreement_percentage: agreement,
                        average_confidence: avg_confidence,
                        total_votes: outputs.len(),
                        votes,
                    })
                } else {
                    ConsensusResult::NoConsensus {
                        highest_agreement: agreement,
                        threshold: self.consensus_threshold,
                        total_models: outputs.len(),
                    }
                }
            }
            None => ConsensusResult::NoConsensus {
                highest_agreement: 0.0,
                threshold: self.consensus_threshold,
                total_models: outputs.len(),
            },
        }
    }

    /// استخراج الخيار من النص (بساطة: أول 50 حرف كمفتاح)
    fn extract_choice(&self, text: &str) -> String {
        let cleaned = text.trim().chars().take(100).collect::<String>();
        if cleaned.len() > 50 { cleaned[..50].to_string() } else { cleaned }
    }
}

/// نتيجة الإجماع
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConsensusResult {
    Agreed(ConsensusAgreement),
    NoConsensus { highest_agreement: f64, threshold: f64, total_models: usize },
    InsufficientModels { required: usize, got: usize },
}

impl ConsensusResult {
    pub fn is_agreed(&self) -> bool {
        matches!(self, Self::Agreed(_))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsensusAgreement {
    pub consensus_choice: String,
    pub agreement_percentage: f64,
    pub average_confidence: f64,
    pub total_votes: usize,
    pub votes: Vec<ConsensusVote>,
}

// ─── Critique Pattern ───────────────────────────────────────────────────

/// معامل النقد
#[derive(Debug, Clone)]
pub struct CritiquePattern {
    /// الحد الأدنى لعدد النقاد
    pub min_critics: usize,
    /// عتبة القبول (0.0 - 1.0)
    pub acceptance_threshold: f64,
    /// هل يقبل النقد التحسيني؟
    pub accept_suggestions: bool,
}

impl Default for CritiquePattern {
    fn default() -> Self {
        Self {
            min_critics: 1,
            acceptance_threshold: 0.5,
            accept_suggestions: true,
        }
    }
}

impl CritiquePattern {
    pub fn new() -> Self { Self::default() }

    /// تقييم مخرجات بواسطة نماذج ناقدة
    pub fn critique(&self, original_output: &str, critics: &[ModelOutput]) -> CritiqueResult {
        if critics.is_empty() {
            return CritiqueResult::NoCritics;
        }

        let mut total_score = 0.0;
        let mut issues = vec![];
        let mut suggestions = vec![];

        for critic in critics {
            let score = self.score_critique(&critic.output);
            total_score += score;
            if score < self.acceptance_threshold {
                issues.push(format!("{}: {}", critic.model_name, critic.output.chars().take(100).collect::<String>()));
            }
            if self.accept_suggestions && critic.output.contains("suggest") || critic.output.contains("تحسين") {
                suggestions.push(critic.output.clone());
            }
        }

        let avg_score = total_score / critics.len() as f64;
        CritiqueResult::Evaluated(CritiqueEvaluation {
            original_output: original_output.to_string(),
            average_score: avg_score,
            passed: avg_score >= self.acceptance_threshold,
            issues,
            suggestions: if self.accept_suggestions { suggestions } else { vec![] },
            critics_used: critics.len(),
        })
    }

    /// تسجيل نقدي بسيط: بحث عن كلمات إيجابية/سلبية
    fn score_critique(&self, text: &str) -> f64 {
        let lower = text.to_lowercase();
        let positive = ["good", "correct", "excellent", "works", "right", "perfect", "fine", "great", "صحيح", "تمام", "ممتاز"];
        let negative = ["wrong", "incorrect", "bad", "error", "bug", "issue", "problem", "fix", "fail", "خطأ", "غلط", "مشكلة"];
        let pos_count = positive.iter().filter(|w| lower.contains(*w)).count();
        let neg_count = negative.iter().filter(|w| lower.contains(*w)).count();
        if pos_count + neg_count == 0 { return 0.5; }
        pos_count as f64 / (pos_count + neg_count) as f64
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CritiqueResult {
    Evaluated(CritiqueEvaluation),
    NoCritics,
}

impl CritiqueResult {
    pub fn passed(&self) -> bool {
        match self {
            Self::Evaluated(e) => e.passed,
            Self::NoCritics => false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CritiqueEvaluation {
    pub original_output: String,
    pub average_score: f64,
    pub passed: bool,
    pub issues: Vec<String>,
    pub suggestions: Vec<String>,
    pub critics_used: usize,
}

// ─── Pipeline Pattern ───────────────────────────────────────────────────

/// مرحلة في الـ Pipeline
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineStage_ {
    pub name: String,
    pub model: String,
    pub prompt_template: String,
    pub input_key: String,
    pub output_key: String,
}

/// معامل الـ Pipeline
#[derive(Debug, Clone)]
pub struct PipelinePattern {
    pub stages: Vec<PipelineStage_>,
}

impl PipelinePattern {
    pub fn new(stages: Vec<PipelineStage_>) -> Self {
        assert!(!stages.is_empty(), "Pipeline must have at least one stage");
        Self { stages }
    }

    /// إنشاء Pipeline افتراضي: تحليل → توليد → مراجعة
    pub fn default_pipeline() -> Self {
        Self {
            stages: vec![
                PipelineStage_ {
                    name: "analysis".to_string(),
                    model: "gpt-4o".to_string(),
                    prompt_template: "Solve this: {input}".to_string(),
                    input_key: "input".to_string(),
                    output_key: "analysis".to_string(),
                },
                PipelineStage_ {
                    name: "generation".to_string(),
                    model: "claude-3-opus".to_string(),
                    prompt_template: "Based on analysis: {analysis}, produce final answer".to_string(),
                    input_key: "analysis".to_string(),
                    output_key: "output".to_string(),
                },
            ],
        }
    }

    pub fn stage_count(&self) -> usize { self.stages.len() }
}

// ─── Pair Pattern ───────────────────────────────────────────────────────

/// نموذجان يعملان جنباً لجنب
#[derive(Debug, Clone)]
pub struct PairPattern {
    pub model_a: String,
    pub model_b: String,
    pub merge_strategy: PairMergeStrategy,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PairMergeStrategy {
    /// خذ الأفضل (حسب الثقة)
    BestOf,
    /// ادمج المخرجين
    Merge,
    /// خذ الأول
    FirstOnly,
}

impl PairPattern {
    pub fn new(model_a: &str, model_b: &str) -> Self {
        Self {
            model_a: model_a.to_string(),
            model_b: model_b.to_string(),
            merge_strategy: PairMergeStrategy::BestOf,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_consensus_agreement() {
        let pattern = ConsensusPattern::default();
        let outputs = vec![
            ModelOutput { model_name: "m1".into(), output: "A".into(), confidence: 0.9, latency_ms: 100, tokens_used: 50, success: true },
            ModelOutput { model_name: "m2".into(), output: "A".into(), confidence: 0.8, latency_ms: 100, tokens_used: 50, success: true },
            ModelOutput { model_name: "m3".into(), output: "A".into(), confidence: 0.7, latency_ms: 100, tokens_used: 50, success: true },
        ];
        let result = pattern.execute("test", &outputs);
        assert!(result.is_agreed());
    }

    #[test]
    fn test_critique_passed() {
        let pattern = CritiquePattern::default();
        let critics = vec![
            ModelOutput { model_name: "critic-1".into(), output: "This is good and correct".into(), confidence: 0.8, latency_ms: 50, tokens_used: 30, success: true },
        ];
        let result = pattern.critique("original", &critics);
        assert!(result.passed());
    }

    #[test]
    fn test_pipeline_stages() {
        let pipe = PipelinePattern::default_pipeline();
        assert_eq!(pipe.stage_count(), 2);
    }
}
