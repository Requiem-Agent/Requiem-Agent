// ─── Work Distribution Model (RAG-like Context Router) ────────────────────
// Phase 15.4: يوزع المهام على النماذج حسب الكفاءة ويدير نافذة السياق

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// ملف تعريف كفاءة النموذج
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelProfile {
    pub name: String,
    /// نقاط القوة (مثل: code, reasoning, creative, analysis)
    pub strengths: Vec<String>,
    /// حجم نافذة السياق القصوى
    pub max_context: usize,
    /// سرعة النموذج (tokens/second)
    pub speed: f64,
    /// موثوقية JSON output (0.0 - 1.0)
    pub json_reliability: f64,
    /// عدد مرات الاستخدام
    pub usage_count: u64,
    /// نسبة النجاح
    pub success_rate: f64,
}

impl ModelProfile {
    pub fn new(
        name: &str,
        strengths: Vec<&str>,
        max_context: usize,
        speed: f64,
        json_rel: f64,
    ) -> Self {
        Self {
            name: name.to_string(),
            strengths: strengths.iter().map(|s| s.to_string()).collect(),
            max_context,
            speed,
            json_reliability: json_rel,
            usage_count: 0,
            success_rate: 1.0,
        }
    }
}

/// توزيع المهمة على نموذج
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskDistribution {
    pub task_id: String,
    pub assigned_model: String,
    pub confidence: f64,
    pub reason: String,
    pub estimated_tokens: usize,
}

/// نوع التوزيع
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DistributionStrategy {
    /// أفضل نموذج للمهمة
    BestFit,
    /// توزيع عشوائي (اختبار)
    RoundRobin,
    /// أسرع نموذج
    Fastest,
    /// الأكثر موثوقية
    MostReliable,
    /// توزيع الحمل
    LoadBalance,
}

impl DistributionStrategy {
    pub fn name(&self) -> &str {
        match self {
            Self::BestFit => "best_fit",
            Self::RoundRobin => "round_robin",
            Self::Fastest => "fastest",
            Self::MostReliable => "most_reliable",
            Self::LoadBalance => "load_balance",
        }
    }
}

/// مدير سياق النافذة
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextWindowManager {
    /// الحد الأقصى لحجم النافذة
    pub max_window: usize,
    /// النسبة المستخدمة حالياً (0.0 - 1.0)
    pub utilization: f64,
    /// عدد الرموز المستخدمة
    pub used_tokens: usize,
    /// المقاطع النصية في النافذة
    pub segments: Vec<ContextSegment>,
    /// استراتيجية الضغط
    pub compression_strategy: CompressionStrategy,
}

/// مقطع سياقي
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextSegment {
    pub id: String,
    pub content: String,
    pub tokens: usize,
    pub priority: u8, // 0-10
    pub age: u64,     // عدد الخطوات منذ آخر استخدام
}

/// استراتيجية الضغط
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CompressionStrategy {
    /// إزالة الأقدم أولاً
    EvictOldest,
    /// إزالة الأقل أهمية
    EvictLowestPriority,
    /// دمج المقاطع المتجاورة
    MergeAdjacent,
    /// الاحتفاظ بالكل (بدون ضغط)
    KeepAll,
}

impl CompressionStrategy {
    pub fn name(&self) -> &str {
        match self {
            Self::EvictOldest => "evict_oldest",
            Self::EvictLowestPriority => "evict_lowest_priority",
            Self::MergeAdjacent => "merge_adjacent",
            Self::KeepAll => "keep_all",
        }
    }
}

impl Default for ContextWindowManager {
    fn default() -> Self {
        Self {
            max_window: 128_000, // 128K tokens
            utilization: 0.0,
            used_tokens: 0,
            segments: vec![],
            compression_strategy: CompressionStrategy::EvictLowestPriority,
        }
    }
}

impl ContextWindowManager {
    pub fn new(max_window: usize) -> Self {
        Self {
            max_window,
            ..Default::default()
        }
    }

    /// إضافة مقطع للنافذة
    pub fn add_segment(&mut self, id: &str, content: &str, tokens: usize, priority: u8) {
        let seg = ContextSegment {
            id: id.to_string(),
            content: content.to_string(),
            tokens,
            priority,
            age: 0,
        };
        self.segments.push(seg);
        self.used_tokens += tokens;
        self.utilization = self.used_tokens as f64 / self.max_window as f64;

        // ضغط إذا تجاوزنا الحد
        if self.used_tokens > self.max_window {
            self.compress();
        }
    }

    /// ضغط النافذة عند الحاجة
    pub fn compress(&mut self) {
        match self.compression_strategy {
            CompressionStrategy::EvictOldest => {
                self.segments.sort_by_key(|s| s.age);
            }
            CompressionStrategy::EvictLowestPriority => {
                self.segments.sort_by_key(|s| s.priority);
            }
            CompressionStrategy::MergeAdjacent => {
                // دمج المتجاورات ذات الأولوية المتشابهة
                let mut merged: Vec<ContextSegment> = vec![];
                for seg in self.segments.drain(..) {
                    if let Some(last) = merged.last_mut() {
                        if last.priority == seg.priority && last.id.starts_with(&seg.id[..2]) {
                            last.content.push_str(&seg.content);
                            last.tokens += seg.tokens;
                            continue;
                        }
                    }
                    merged.push(seg);
                }
                self.segments = merged;
                return;
            }
            CompressionStrategy::KeepAll => return,
        }

        // إزالة المقاطع من الأقل أهمية حتى نعود تحت الحد
        while self.used_tokens > self.max_window && !self.segments.is_empty() {
            if let Some(removed) = self.segments.pop() {
                self.used_tokens = self.used_tokens.saturating_sub(removed.tokens);
            }
        }
        self.utilization = self.used_tokens as f64 / self.max_window as f64;
    }

    /// تحديث عمر كل المقاطع (يُستدعى كل خطوة)
    pub fn tick(&mut self) {
        for seg in &mut self.segments {
            seg.age += 1;
        }
    }

    pub fn reset(&mut self) {
        self.segments.clear();
        self.used_tokens = 0;
        self.utilization = 0.0;
    }
}

/// موجه السياق الرئيسي
#[derive(Debug, Clone)]
pub struct ContextRouter {
    /// سجل النماذج
    pub models: Vec<ModelProfile>,
    /// الاستراتيجية الحالية
    pub strategy: DistributionStrategy,
    /// مدير النافذة
    pub window: ContextWindowManager,
    /// مؤشر RoundRobin
    rr_index: usize,
}

impl Default for ContextRouter {
    fn default() -> Self {
        Self {
            models: vec![
                ModelProfile::new(
                    "gpt-4o",
                    vec!["code", "reasoning", "analysis", "creative"],
                    128_000,
                    50.0,
                    0.98,
                ),
                ModelProfile::new(
                    "claude-3-opus",
                    vec!["reasoning", "analysis", "code", "creative"],
                    200_000,
                    30.0,
                    0.97,
                ),
                ModelProfile::new(
                    "claude-3-sonnet",
                    vec!["code", "reasoning", "speed"],
                    128_000,
                    80.0,
                    0.95,
                ),
                ModelProfile::new(
                    "gemini-pro",
                    vec!["analysis", "creative", "speed"],
                    128_000,
                    100.0,
                    0.92,
                ),
            ],
            strategy: DistributionStrategy::BestFit,
            window: ContextWindowManager::default(),
            rr_index: 0,
        }
    }
}

impl ContextRouter {
    pub fn new() -> Self {
        Self::default()
    }

    /// توزيع مهمة على أفضل نموذج
    pub fn route(
        &mut self,
        task_description: &str,
        task_id: &str,
        estimated_tokens: usize,
    ) -> TaskDistribution {
        match self.strategy {
            DistributionStrategy::BestFit => {
                self.route_best_fit(task_description, task_id, estimated_tokens)
            }
            DistributionStrategy::RoundRobin => self.route_round_robin(task_id, estimated_tokens),
            DistributionStrategy::Fastest => self.route_fastest(task_id, estimated_tokens),
            DistributionStrategy::MostReliable => {
                self.route_most_reliable(task_id, estimated_tokens)
            }
            DistributionStrategy::LoadBalance => self.route_load_balance(task_id, estimated_tokens),
        }
    }

    fn route_best_fit(&mut self, task: &str, task_id: &str, tokens: usize) -> TaskDistribution {
        let lower = task.to_lowercase();
        let mut best_score = 0;
        let mut best_idx = 0;

        for (i, model) in self.models.iter().enumerate() {
            let score = model
                .strengths
                .iter()
                .filter(|s| lower.contains(s.as_str()))
                .count();
            if score > best_score {
                best_score = score;
                best_idx = i;
            }
        }

        let model = &mut self.models[best_idx];
        model.usage_count += 1;

        TaskDistribution {
            task_id: task_id.to_string(),
            assigned_model: model.name.clone(),
            confidence: best_score as f64 / 3.0_f64.max(1.0),
            reason: format!("best fit: {} strengths match", best_score),
            estimated_tokens: tokens,
        }
    }

    fn route_round_robin(&mut self, task_id: &str, tokens: usize) -> TaskDistribution {
        let idx = self.rr_index % self.models.len();
        self.rr_index += 1;
        let model = &mut self.models[idx];
        model.usage_count += 1;
        TaskDistribution {
            task_id: task_id.to_string(),
            assigned_model: model.name.clone(),
            confidence: 0.5,
            reason: "round_robin distribution".to_string(),
            estimated_tokens: tokens,
        }
    }

    fn route_fastest(&mut self, task_id: &str, tokens: usize) -> TaskDistribution {
        let mut fastest_idx = 0;
        let mut fastest_speed = 0.0;
        for (i, m) in self.models.iter().enumerate() {
            if m.speed > fastest_speed {
                fastest_speed = m.speed;
                fastest_idx = i;
            }
        }
        let model = &mut self.models[fastest_idx];
        model.usage_count += 1;
        TaskDistribution {
            task_id: task_id.to_string(),
            assigned_model: model.name.clone(),
            confidence: 0.7,
            reason: format!("fastest model ({:.0} tok/s)", fastest_speed),
            estimated_tokens: tokens,
        }
    }

    fn route_most_reliable(&mut self, task_id: &str, tokens: usize) -> TaskDistribution {
        let mut best_idx = 0;
        let mut best_rel = 0.0;
        for (i, m) in self.models.iter().enumerate() {
            let rel = m.json_reliability * m.success_rate;
            if rel > best_rel {
                best_rel = rel;
                best_idx = i;
            }
        }
        let model = &mut self.models[best_idx];
        model.usage_count += 1;
        TaskDistribution {
            task_id: task_id.to_string(),
            assigned_model: model.name.clone(),
            confidence: best_rel,
            reason: format!("most reliable ({:.2} combined)", best_rel),
            estimated_tokens: tokens,
        }
    }

    fn route_load_balance(&mut self, task_id: &str, tokens: usize) -> TaskDistribution {
        let min_idx = self
            .models
            .iter()
            .enumerate()
            .min_by_key(|(_, m)| m.usage_count)
            .map(|(i, _)| i)
            .unwrap_or(0);
        let model = &mut self.models[min_idx];
        model.usage_count += 1;
        TaskDistribution {
            task_id: task_id.to_string(),
            assigned_model: model.name.clone(),
            confidence: 0.6,
            reason: format!("load balanced ({} uses)", model.usage_count),
            estimated_tokens: tokens,
        }
    }

    /// تسجيل نجاح/فشل نموذج
    pub fn record_outcome(&mut self, model_name: &str, success: bool) {
        if let Some(model) = self.models.iter_mut().find(|m| m.name == model_name) {
            model.usage_count += 1;
            model.success_rate = if success {
                (model.success_rate * model.usage_count as f64 + 1.0)
                    / (model.usage_count + 1) as f64
            } else {
                (model.success_rate * model.usage_count as f64) / (model.usage_count + 1) as f64
            };
        }
    }

    pub fn set_strategy(&mut self, s: DistributionStrategy) {
        self.strategy = s;
    }
    pub fn add_model(&mut self, profile: ModelProfile) {
        self.models.push(profile);
    }
    pub fn models_summary(&self) -> Vec<HashMap<String, String>> {
        self.models
            .iter()
            .map(|m| {
                let mut h = HashMap::new();
                h.insert("name".to_string(), m.name.clone());
                h.insert("strengths".to_string(), m.strengths.join(", "));
                h.insert("usage".to_string(), m.usage_count.to_string());
                h.insert("success_rate".to_string(), format!("{:.2}", m.success_rate));
                h.insert(
                    "json_reliability".to_string(),
                    format!("{:.2}", m.json_reliability),
                );
                h.insert("speed".to_string(), format!("{:.0}", m.speed));
                h
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_route_best_fit() {
        let mut router = ContextRouter::new();
        let dist = router.route("write code for a new feature", "task-1", 5000);
        assert!(!dist.assigned_model.is_empty());
        assert!(dist.confidence > 0.0);
    }

    #[test]
    fn test_context_window() {
        let mut cwm = ContextWindowManager::new(1000);
        cwm.add_segment("seg-1", "hello world", 100, 5);
        cwm.add_segment("seg-2", "more content here", 200, 8);
        assert_eq!(cwm.segments.len(), 2);
        assert_eq!(cwm.used_tokens, 300);
    }

    #[test]
    fn test_window_compression() {
        let mut cwm = ContextWindowManager::new(500);
        cwm.add_segment("seg-1", "content", 400, 1);
        cwm.add_segment("seg-2", "more", 300, 10);
        // المجموع 700 > 500، يجب ضغط
        assert!(cwm.used_tokens <= 500);
    }
}
