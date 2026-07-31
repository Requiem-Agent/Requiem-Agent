// ─── SmartRouter V2 ──────────────────────────────────────────────────────
// Phase 16.2: PerformanceHistory + AdaptiveRouting + LoadBalancer

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// سجل أداء النموذج
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceRecord {
    pub model: String,
    pub task_type: String,
    pub success: bool,
    pub latency_ms: u64,
    pub tokens_used: usize,
    pub timestamp: String,
}

/// إحصائيات أداء النموذج
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelPerformance {
    pub model: String,
    pub total_calls: u64,
    pub successes: u64,
    pub failures: u64,
    pub avg_latency_ms: f64,
    pub avg_tokens: f64,
    pub last_seen: String,
    /// الأداء حسب نوع المهمة
    pub by_task_type: HashMap<String, TaskTypeStats>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskTypeStats {
    pub calls: u64,
    pub successes: u64,
    pub avg_latency: f64,
}

impl ModelPerformance {
    pub fn new(model: &str) -> Self {
        Self {
            model: model.to_string(),
            total_calls: 0,
            successes: 0,
            failures: 0,
            avg_latency_ms: 0.0,
            avg_tokens: 0.0,
            last_seen: chrono::Utc::now().to_rfc3339(),
            by_task_type: HashMap::new(),
        }
    }

    pub fn success_rate(&self) -> f64 {
        if self.total_calls == 0 { return 0.5; }
        self.successes as f64 / self.total_calls as f64
    }
}

/// معامل التوجيه المتكيف
#[derive(Debug, Clone)]
pub struct AdaptiveRouter {
    /// سجل الأداء
    pub history: HashMap<String, ModelPerformance>,
    /// معامل الاستكشاف (0.0 = استغلال فقط، 1.0 = استكشاف فقط)
    pub exploration_rate: f64,
    /// حجم نافذة التاريخ للأداء الحديث
    pub recency_window: usize,
}

impl Default for AdaptiveRouter {
    fn default() -> Self {
        Self {
            history: HashMap::new(),
            exploration_rate: 0.1,
            recency_window: 100,
        }
    }
}

impl AdaptiveRouter {
    pub fn new() -> Self { Self::default() }

    /// تسجيل نتيجة نموذج
    pub fn record(&mut self, model: &str, task_type: &str, success: bool, latency: u64, tokens: usize) {
        let perf = self.history.entry(model.to_string())
            .or_insert_with(|| ModelPerformance::new(model));
        perf.total_calls += 1;
        if success { perf.successes += 1; } else { perf.failures += 1; }
        perf.avg_latency_ms = (perf.avg_latency_ms * (perf.total_calls - 1) as f64 + latency as f64) / perf.total_calls as f64;
        perf.avg_tokens = (perf.avg_tokens * (perf.total_calls - 1) as f64 + tokens as f64) / perf.total_calls as f64;
        perf.last_seen = chrono::Utc::now().to_rfc3339();

        let task_stats = perf.by_task_type.entry(task_type.to_string())
            .or_insert_with(|| TaskTypeStats { calls: 0, successes: 0, avg_latency: 0.0 });
        task_stats.calls += 1;
        if success { task_stats.successes += 1; }
        task_stats.avg_latency = (task_stats.avg_latency * (task_stats.calls - 1) as f64 + latency as f64) / task_stats.calls as f64;
    }

    /// اختيار أفضل نموذج لمهمة
    pub fn select_best(&mut self, task_type: &str, available_models: &[String]) -> Option<String> {
        if available_models.is_empty() { return None; }

        // استكشاف: اختر عشوائياً
        if rand::random::<f64>() < self.exploration_rate {
            let idx = rand::random::<usize>() % available_models.len();
            return Some(available_models[idx].clone());
        }

        // استغلال: اختر الأفضل حسب نوع المهمة
        let mut best_model = None;
        let mut best_score = f64::MIN;

        for model in available_models {
            let score = self.score_model(model, task_type);
            if score > best_score {
                best_score = score;
                best_model = Some(model.clone());
            }
        }

        best_model
    }

    /// تقييم نموذج لمهمة محددة
    fn score_model(&self, model: &str, task_type: &str) -> f64 {
        let perf = match self.history.get(model) {
            Some(p) => p,
            None => return 0.5, // نموذج جديد: ثقة متوسطة
        };

        let mut score = 0.0;

        // الأداء العام
        score += perf.success_rate() * 0.4;

        // الأداء حسب نوع المهمة
        if let Some(task_stats) = perf.by_task_type.get(task_type) {
            let task_success = if task_stats.calls > 0 {
                task_stats.successes as f64 / task_stats.calls as f64
            } else { 0.5 };
            score += task_success * 0.4;

            // السرعة (عكسياً)
            let speed_score = if task_stats.avg_latency > 0.0 {
                (1000.0 / task_stats.avg_latency).min(1.0)
            } else { 0.5 };
            score += speed_score * 0.2;
        } else {
            // لا توجد بيانات كافية لهذا النوع
            score += 0.3;
        }

        score
    }

    pub fn set_exploration(&mut self, rate: f64) { self.exploration_rate = rate.clamp(0.0, 1.0); }
    pub fn summary(&self) -> Vec<Value> {
        self.history.iter().map(|(name, perf)| {
            serde_json::json!({
                "model": name,
                "total_calls": perf.total_calls,
                "success_rate": perf.success_rate(),
                "avg_latency_ms": perf.avg_latency_ms,
                "avg_tokens": perf.avg_tokens,
                "task_types": perf.by_task_type.len(),
            })
        }).collect()
    }
}

use serde_json::Value;

/// موازن الحمل
#[derive(Debug, Clone)]
pub struct LoadBalancer {
    /// الحد الأقصى للحمل المتزامن لكل نموذج
    pub max_concurrent: HashMap<String, usize>,
    /// الأحمال الحالية
    pub current_load: HashMap<String, usize>,
    /// استراتيجية التوازن
    pub strategy: BalanceStrategy,
}

#[derive(Debug, Clone, PartialEq)]
pub enum BalanceStrategy {
    /// أقل حمل أولاً
    LeastLoaded,
    /// توزيع متساوٍ
    RoundRobin,
    /// حسب الأداء
    PerformanceBased,
}

impl Default for LoadBalancer {
    fn default() -> Self {
        let mut max_concurrent = HashMap::new();
        max_concurrent.insert("gpt-4o".to_string(), 10);
        max_concurrent.insert("claude-3-opus".to_string(), 5);
        max_concurrent.insert("claude-3-sonnet".to_string(), 15);
        max_concurrent.insert("gemini-pro".to_string(), 20);
        Self {
            max_concurrent,
            current_load: HashMap::new(),
            strategy: BalanceStrategy::LeastLoaded,
        }
    }
}

impl LoadBalancer {
    pub fn new() -> Self { Self::default() }

    /// هل يمكن للنموذج قبول مهمة جديدة؟
    pub fn can_accept(&self, model: &str) -> bool {
        let max = self.max_concurrent.get(model).copied().unwrap_or(5);
        let current = self.current_load.get(model).copied().unwrap_or(0);
        current < max
    }

    /// الحصول على النموذج الأقل تحميلاً
    pub fn least_loaded(&self, available: &[String]) -> Option<String> {
        available.iter()
            .filter(|m| self.can_accept(m))
            .min_by_key(|m| self.current_load.get(*m).copied().unwrap_or(0))
            .cloned()
    }

    /// بدء مهمة على نموذج
    pub fn start_task(&mut self, model: &str) {
        *self.current_load.entry(model.to_string()).or_insert(0) += 1;
    }

    /// إنهاء مهمة على نموذج
    pub fn end_task(&mut self, model: &str) {
        if let Some(count) = self.current_load.get_mut(model) {
            if *count > 0 { *count -= 1; }
        }
    }

    pub fn set_max_concurrent(&mut self, model: &str, max: usize) {
        self.max_concurrent.insert(model.to_string(), max);
    }

    pub fn load_report(&self) -> Vec<Value> {
        self.max_concurrent.keys().map(|model| {
            let current = self.current_load.get(model).copied().unwrap_or(0);
            let max = self.max_concurrent.get(model).copied().unwrap_or(5);
            serde_json::json!({
                "model": model,
                "current": current,
                "max": max,
                "available": current < max,
                "utilization_pct": if max > 0 { (current as f64 / max as f64 * 100.0) as u64 } else { 0 },
            })
        }).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adaptive_router_selection() {
        let mut router = AdaptiveRouter::new();
        router.exploration_rate = 0.0; // استغلال فقط
        router.record("gpt-4o", "code", true, 100, 500);
        router.record("gpt-4o", "code", true, 100, 500);
        router.record("gpt-4o", "code", true, 100, 500);
        router.record("claude-3-opus", "code", false, 200, 1000);
        let best = router.select_best("code", &["gpt-4o".to_string(), "claude-3-opus".to_string()]);
        assert_eq!(best, Some("gpt-4o".to_string()));
    }

    #[test]
    fn test_load_balancer() {
        let mut lb = LoadBalancer::new();
        assert!(lb.can_accept("gpt-4o"));
        lb.start_task("gpt-4o");
        assert_eq!(lb.current_load.get("gpt-4o").copied().unwrap(), 1);
        lb.end_task("gpt-4o");
        assert_eq!(lb.current_load.get("gpt-4o").copied().unwrap(), 0);
    }
}
