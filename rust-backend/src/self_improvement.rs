// self_improvement.rs — S10-02: Self-Improvement Loop
// الـ agent يُحلّل أداءه الخاص ويقترح تحسينات

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{info, warn};

// ─────────────────────────────────────────────────────────────────────────────
// Types
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    pub period_hours: u32,
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub avg_latency_ms: f64,
    pub p95_latency_ms: f64,
    pub p99_latency_ms: f64,
    pub rate_limit_hits: u64,
    pub llm_calls: u64,
    pub llm_failures: u64,
    pub avg_tokens_per_request: f64,
    pub react_steps_avg: f64,
    pub tool_usage: HashMap<String, u64>,
}

impl PerformanceMetrics {
    pub fn success_rate(&self) -> f64 {
        if self.total_requests == 0 {
            return 1.0;
        }
        self.successful_requests as f64 / self.total_requests as f64
    }

    pub fn llm_success_rate(&self) -> f64 {
        if self.llm_calls == 0 {
            return 1.0;
        }
        (self.llm_calls - self.llm_failures) as f64 / self.llm_calls as f64
    }

    pub fn error_rate(&self) -> f64 {
        1.0 - self.success_rate()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImprovementSuggestion {
    pub category: SuggestionCategory,
    pub priority: Priority,
    pub title: String,
    pub description: String,
    pub expected_impact: String,
    pub implementation_hint: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum SuggestionCategory {
    Performance,
    Reliability,
    CostOptimization,
    UserExperience,
    Security,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum Priority {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelfImprovementReport {
    pub generated_at: String,
    pub metrics_period_hours: u32,
    pub overall_health_score: f64,   // 0.0 - 100.0
    pub suggestions: Vec<ImprovementSuggestion>,
    pub top_issues: Vec<String>,
    pub strengths: Vec<String>,
}

// ─────────────────────────────────────────────────────────────────────────────
// SelfImprovementEngine
// ─────────────────────────────────────────────────────────────────────────────

pub struct SelfImprovementEngine {
    /// حد الـ error rate الذي يُشغّل تحذيراً
    pub error_rate_threshold: f64,
    /// حد الـ latency (ms) الذي يُشغّل تحذيراً
    pub latency_threshold_ms: f64,
    /// حد الـ LLM failure rate
    pub llm_failure_threshold: f64,
}

impl Default for SelfImprovementEngine {
    fn default() -> Self {
        Self {
            error_rate_threshold: 0.05,    // 5%
            latency_threshold_ms: 2000.0,  // 2 ثانية
            llm_failure_threshold: 0.10,   // 10%
        }
    }
}

impl SelfImprovementEngine {
    /// يُحلّل الـ metrics ويُولّد تقرير تحسين
    pub fn analyze(&self, metrics: &PerformanceMetrics) -> SelfImprovementReport {
        let mut suggestions = Vec::new();
        let mut top_issues = Vec::new();
        let mut strengths = Vec::new();

        // ── تحليل الـ error rate ──────────────────────────────────────────
        if metrics.error_rate() > self.error_rate_threshold {
            top_issues.push(format!(
                "معدل الأخطاء مرتفع: {:.1}% (الحد: {:.1}%)",
                metrics.error_rate() * 100.0,
                self.error_rate_threshold * 100.0
            ));
            suggestions.push(ImprovementSuggestion {
                category: SuggestionCategory::Reliability,
                priority: if metrics.error_rate() > 0.20 { Priority::Critical } else { Priority::High },
                title: "تقليل معدل الأخطاء".into(),
                description: format!(
                    "معدل الأخطاء الحالي {:.1}% يتجاوز الحد المقبول {:.1}%",
                    metrics.error_rate() * 100.0,
                    self.error_rate_threshold * 100.0
                ),
                expected_impact: "تحسين تجربة المستخدم وتقليل الشكاوى".into(),
                implementation_hint: "راجع logs الأخطاء وحدّد الـ endpoints الأكثر فشلاً".into(),
            });
        } else {
            strengths.push(format!("معدل النجاح ممتاز: {:.1}%", metrics.success_rate() * 100.0));
        }

        // ── تحليل الـ latency ─────────────────────────────────────────────
        if metrics.p95_latency_ms > self.latency_threshold_ms {
            top_issues.push(format!(
                "p95 latency مرتفع: {:.0}ms (الحد: {:.0}ms)",
                metrics.p95_latency_ms, self.latency_threshold_ms
            ));
            suggestions.push(ImprovementSuggestion {
                category: SuggestionCategory::Performance,
                priority: Priority::High,
                title: "تحسين زمن الاستجابة".into(),
                description: format!(
                    "p95 latency = {:.0}ms يتجاوز الهدف {:.0}ms",
                    metrics.p95_latency_ms, self.latency_threshold_ms
                ),
                expected_impact: "تحسين تجربة المستخدم وتقليل timeout errors".into(),
                implementation_hint: "فعّل caching للـ responses الشائعة، وحسّن DB queries".into(),
            });
        }

        // ── تحليل LLM failures ────────────────────────────────────────────
        if metrics.llm_success_rate() < (1.0 - self.llm_failure_threshold) {
            top_issues.push(format!(
                "معدل فشل LLM مرتفع: {:.1}%",
                (1.0 - metrics.llm_success_rate()) * 100.0
            ));
            suggestions.push(ImprovementSuggestion {
                category: SuggestionCategory::Reliability,
                priority: Priority::High,
                title: "تحسين موثوقية LLM calls".into(),
                description: "معدل فشل LLM مرتفع يؤثر على جودة الردود".into(),
                expected_impact: "ردود أكثر موثوقية وتقليل الأخطاء".into(),
                implementation_hint: "أضف retry logic مع exponential backoff، وفعّل fallback models".into(),
            });
        }

        // ── تحليل Rate Limit hits ─────────────────────────────────────────
        if metrics.rate_limit_hits > metrics.total_requests / 10 {
            suggestions.push(ImprovementSuggestion {
                category: SuggestionCategory::UserExperience,
                priority: Priority::Medium,
                title: "مراجعة حدود الـ rate limiting".into(),
                description: format!(
                    "{} طلب تم رفضه بسبب rate limiting ({:.1}% من الطلبات)",
                    metrics.rate_limit_hits,
                    metrics.rate_limit_hits as f64 / metrics.total_requests as f64 * 100.0
                ),
                expected_impact: "تقليل الإحباط للمستخدمين النشطين".into(),
                implementation_hint: "ارفع الحدود للمستخدمين المدفوعين، أو أضف queue system".into(),
            });
        }

        // ── تحليل Token usage ─────────────────────────────────────────────
        if metrics.avg_tokens_per_request > 3000.0 {
            suggestions.push(ImprovementSuggestion {
                category: SuggestionCategory::CostOptimization,
                priority: Priority::Medium,
                title: "تحسين استهلاك الـ tokens".into(),
                description: format!(
                    "متوسط {:.0} token/طلب مرتفع — يزيد التكلفة",
                    metrics.avg_tokens_per_request
                ),
                expected_impact: "تقليل تكلفة LLM API بنسبة 20-40%".into(),
                implementation_hint: "اضغط الـ system prompt، وفعّل conversation summarization".into(),
            });
        }

        // ── تحليل Tool usage ──────────────────────────────────────────────
        let unused_tools: Vec<&str> = ["web_search", "calculator", "code_exec"]
            .iter()
            .filter(|&&t| !metrics.tool_usage.contains_key(t))
            .copied()
            .collect();

        if !unused_tools.is_empty() {
            suggestions.push(ImprovementSuggestion {
                category: SuggestionCategory::UserExperience,
                priority: Priority::Low,
                title: "تفعيل الأدوات غير المستخدَمة".into(),
                description: format!("الأدوات التالية لم تُستخدَم: {}", unused_tools.join(", ")),
                expected_impact: "تحسين قدرات الـ agent وتوسيع نطاق المهام".into(),
                implementation_hint: "أضف أمثلة في system prompt لتشجيع استخدام هذه الأدوات".into(),
            });
        }

        // ── حساب health score ─────────────────────────────────────────────
        let health_score = calculate_health_score(metrics, self);

        if health_score > 90.0 {
            strengths.push("أداء ممتاز بشكل عام".into());
        }

        // ترتيب الاقتراحات بالأولوية
        suggestions.sort_by(|a, b| b.priority.cmp(&a.priority));

        info!(
            health_score = health_score,
            suggestions_count = suggestions.len(),
            "Self-improvement analysis complete"
        );

        SelfImprovementReport {
            generated_at: chrono::Utc::now().to_rfc3339(),
            metrics_period_hours: metrics.period_hours,
            overall_health_score: health_score,
            suggestions,
            top_issues,
            strengths,
        }
    }
}

fn calculate_health_score(metrics: &PerformanceMetrics, engine: &SelfImprovementEngine) -> f64 {
    let mut score = 100.0_f64;

    // خصم بسبب error rate
    score -= (metrics.error_rate() / engine.error_rate_threshold).min(1.0) * 30.0;

    // خصم بسبب latency
    if metrics.p95_latency_ms > engine.latency_threshold_ms {
        score -= ((metrics.p95_latency_ms - engine.latency_threshold_ms) / engine.latency_threshold_ms).min(1.0) * 20.0;
    }

    // خصم بسبب LLM failures
    score -= ((1.0 - metrics.llm_success_rate()) / engine.llm_failure_threshold).min(1.0) * 25.0;

    // خصم بسبب rate limit hits
    if metrics.total_requests > 0 {
        let rl_rate = metrics.rate_limit_hits as f64 / metrics.total_requests as f64;
        score -= rl_rate.min(0.5) * 25.0;
    }

    score.max(0.0).min(100.0)
}

// ─────────────────────────────────────────────────────────────────────────────
// Unit tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn healthy_metrics() -> PerformanceMetrics {
        PerformanceMetrics {
            period_hours: 24,
            total_requests: 1000,
            successful_requests: 980,
            failed_requests: 20,
            avg_latency_ms: 300.0,
            p95_latency_ms: 800.0,
            p99_latency_ms: 1500.0,
            rate_limit_hits: 5,
            llm_calls: 800,
            llm_failures: 10,
            avg_tokens_per_request: 1500.0,
            react_steps_avg: 3.2,
            tool_usage: HashMap::from([("calculator".into(), 50u64)]),
        }
    }

    #[test]
    fn test_healthy_system_high_score() {
        let engine = SelfImprovementEngine::default();
        let metrics = healthy_metrics();
        let report = engine.analyze(&metrics);
        assert!(report.overall_health_score > 70.0);
    }

    #[test]
    fn test_high_error_rate_triggers_suggestion() {
        let engine = SelfImprovementEngine::default();
        let mut metrics = healthy_metrics();
        metrics.failed_requests = 200;
        metrics.successful_requests = 800;
        let report = engine.analyze(&metrics);
        assert!(report.suggestions.iter().any(|s| s.category == SuggestionCategory::Reliability));
        assert!(!report.top_issues.is_empty());
    }

    #[test]
    fn test_high_latency_triggers_suggestion() {
        let engine = SelfImprovementEngine::default();
        let mut metrics = healthy_metrics();
        metrics.p95_latency_ms = 5000.0;
        let report = engine.analyze(&metrics);
        assert!(report.suggestions.iter().any(|s| s.category == SuggestionCategory::Performance));
    }

    #[test]
    fn test_high_token_usage_triggers_cost_suggestion() {
        let engine = SelfImprovementEngine::default();
        let mut metrics = healthy_metrics();
        metrics.avg_tokens_per_request = 5000.0;
        let report = engine.analyze(&metrics);
        assert!(report.suggestions.iter().any(|s| s.category == SuggestionCategory::CostOptimization));
    }

    #[test]
    fn test_suggestions_sorted_by_priority() {
        let engine = SelfImprovementEngine::default();
        let mut metrics = healthy_metrics();
        metrics.failed_requests = 300;
        metrics.successful_requests = 700;
        metrics.p95_latency_ms = 5000.0;
        let report = engine.analyze(&metrics);
        for i in 1..report.suggestions.len() {
            assert!(report.suggestions[i - 1].priority >= report.suggestions[i].priority);
        }
    }

    #[test]
    fn test_success_rate_calculation() {
        let metrics = healthy_metrics();
        assert!((metrics.success_rate() - 0.98).abs() < 0.001);
    }
}
