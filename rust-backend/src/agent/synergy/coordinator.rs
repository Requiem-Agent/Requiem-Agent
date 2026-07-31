// ─── Model Synergy Coordinator ────────────────────────────────────────────
// Phase 16.3: يربط Synergy Patterns + SmartRouter + AgentEngine

use super::{
    ModelOutput, SynergyRound, SynergyPattern,
    ConsensusPattern, ConsensusResult,
    CritiquePattern, CritiqueResult,
    PipelinePattern,
    AdaptiveRouter, LoadBalancer,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Instant;

/// منسّق التآزر الرئيسي
#[derive(Debug, Clone)]
pub struct ModelSynergyCoordinator {
    pub consensus: ConsensusPattern,
    pub critique: CritiquePattern,
    pub pipeline: PipelinePattern,
    pub router: AdaptiveRouter,
    pub load_balancer: LoadBalancer,
    pub history: Vec<SynergyRound>,
    pub max_history: usize,
    pub active_pattern: SynergyPattern,
}

impl Default for ModelSynergyCoordinator {
    fn default() -> Self {
        Self {
            consensus: ConsensusPattern::new(),
            critique: CritiquePattern::new(),
            pipeline: PipelinePattern::default_pipeline(),
            router: AdaptiveRouter::new(),
            load_balancer: LoadBalancer::new(),
            history: vec![],
            max_history: 50,
            active_pattern: SynergyPattern::Consensus,
        }
    }
}

impl ModelSynergyCoordinator {
    pub fn new() -> Self { Self::default() }

    /// تشغيل جولة تآزر كاملة
    pub async fn run_round(
        &mut self,
        pattern: SynergyPattern,
        question: &str,
        available_models: &[String],
        task_type: &str,
    ) -> SynergyRound {
        let start = Instant::now();
        let round_id = format!("syn-{}-{}", pattern.name(), self.history.len() + 1);
        let mut outputs = vec![];
        let mut used_models = vec![];

        // توزيع المهمة على النماذج المحددة
        for model_name in available_models {
            if !self.load_balancer.can_accept(model_name) {
                continue; // النموذج محمّل بالكامل
            }
            self.load_balancer.start_task(model_name);
            used_models.push(model_name.clone());

            // محاكاة استدعاء النموذج (في الإنتاج: استدعاء حقيقي)
            let output = self.simulate_model_call(model_name, question).await;
            outputs.push(output.clone());

            self.load_balancer.end_task(model_name);
            self.router.record(model_name, task_type, output.success, output.latency_ms, output.tokens_used);
        }

        // تطبيق النمط المختار
        let (final_output, consensus_score) = match pattern {
            SynergyPattern::Consensus => self.apply_consensus(question, &outputs),
            SynergyPattern::Critique => self.apply_critique(question, &outputs),
            SynergyPattern::Pipeline => self.apply_pipeline(question),
            SynergyPattern::Pair => self.apply_pair(&outputs),
        };

        let total_ms = start.elapsed().as_millis() as u64;
        let total_tokens: usize = outputs.iter().map(|o| o.tokens_used).sum();

        let round = SynergyRound {
            round_id,
            pattern: pattern.clone(),
            models_used: used_models,
            outputs,
            final_output,
            consensus_score,
            total_latency_ms: total_ms,
            total_tokens,
        };

        self.history.push(round.clone());
        if self.history.len() > self.max_history {
            self.history.remove(0);
        }

        round
    }

    /// تطبيق نمط الإجماع
    fn apply_consensus(&self, question: &str, outputs: &[ModelOutput]) -> (String, f64) {
        match self.consensus.execute(question, outputs) {
            ConsensusResult::Agreed(agreement) => {
                (agreement.consensus_choice.clone(), agreement.average_confidence)
            }
            ConsensusResult::NoConsensus { highest_agreement, .. } => {
                // خذ أعلى خيار
                let best = outputs.iter().max_by(|a, b| a.confidence.partial_cmp(&b.confidence).unwrap_or(std::cmp::Ordering::Equal));
                match best {
                    Some(o) => (o.output.clone(), highest_agreement),
                    None => ("No consensus reached".to_string(), 0.0),
                }
            }
            ConsensusResult::InsufficientModels { .. } => {
                (outputs.first().map(|o| o.output.clone()).unwrap_or_default(), 0.0)
            }
        }
    }

    /// تطبيق نمط النقد
    fn apply_critique(&self, _question: &str, outputs: &[ModelOutput]) -> (String, f64) {
        if outputs.is_empty() { return ("No output".to_string(), 0.0); }
        let primary = &outputs[0];
        let critics: Vec<ModelOutput> = outputs.iter().skip(1).cloned().collect();
        if critics.is_empty() {
            return (primary.output.clone(), primary.confidence);
        }
        match self.critique.critique(&primary.output, &critics) {
            CritiqueResult::Evaluated(eval) => {
                if eval.passed {
                    (primary.output.clone(), eval.average_score)
                } else if let Some(suggestion) = eval.suggestions.first() {
                    (suggestion.clone(), eval.average_score)
                } else {
                    (primary.output.clone(), eval.average_score)
                }
            }
            CritiqueResult::NoCritics => (primary.output.clone(), primary.confidence),
        }
    }

    /// تطبيق نمط الـ Pipeline
    fn apply_pipeline(&self, input: &str) -> (String, f64) {
        // في الإنتاج: كل مرحلة تستدعي نموذجاً مختلفاً
        let mut current = input.to_string();
        for stage in &self.pipeline.stages {
            current = format!("{} processed by {}", current, stage.model);
        }
        (current, 0.8)
    }

    /// تطبيق نمط الزوج
    fn apply_pair(&self, outputs: &[ModelOutput]) -> (String, f64) {
        if outputs.is_empty() { return ("No output".to_string(), 0.0); }
        if outputs.len() == 1 { return (outputs[0].output.clone(), outputs[0].confidence); }
        let a = &outputs[0];
        let b = &outputs[1];
        if a.confidence >= b.confidence {
            (a.output.clone(), a.confidence)
        } else {
            (b.output.clone(), b.confidence)
        }
    }

    /// محاكاة استدعاء نموذج (للتطوير — يستبدل بالاستدعاء الحقيقي)
    async fn simulate_model_call(&self, model: &str, question: &str) -> ModelOutput {
        let latency = rand::random::<u64>() % 500 + 100; // 100-600ms
        let tokens = rand::random::<usize>() % 1000 + 100;
        tokio::time::sleep(std::time::Duration::from_millis(10)).await; // محاكاة
        ModelOutput {
            model_name: model.to_string(),
            output: format!("{} response to: {}", model, question.chars().take(50).collect::<String>()),
            confidence: 0.5 + rand::random::<f64>() * 0.5,
            latency_ms: latency,
            tokens_used: tokens,
            success: true,
        }
    }

    pub fn set_pattern(&mut self, pattern: SynergyPattern) { self.active_pattern = pattern; }
    pub fn recent_rounds(&self, n: usize) -> Vec<SynergyRound> {
        self.history.iter().rev().take(n).cloned().collect()
    }

    /// تقرير كامل
    pub fn report(&self) -> serde_json::Value {
        serde_json::json!({
            "active_pattern": self.active_pattern.name(),
            "rounds_completed": self.history.len(),
            "models_tracked": self.router.history.len(),
            "router_exploration": self.router.exploration_rate,
            "load_balancer": self.load_balancer.load_report(),
        })
    }
}
