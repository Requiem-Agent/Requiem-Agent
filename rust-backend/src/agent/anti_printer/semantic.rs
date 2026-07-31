// ─── Semantic Analysis Engine ──────────────────────────────────────────────
// Phase 15.1: تصنيف النية + تتبع السياق + التحقق الدلالي

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// تصنيف نية المستخدم
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Intent {
    /// سؤال معلوماتي
    InformationalQuery,
    /// طلب تنفيذ مهمة
    TaskExecution,
    /// تعديل كود
    CodeModification,
    /// تحليل / Debugging
    Analysis,
    /// إنشاء محتوى
    ContentCreation,
    /// استكشاف النظام
    Exploration,
    /// تصحيح خطأ
    ErrorCorrection,
    /// طلب توضيح
    Clarification,
    /// أمر إداري (System)
    Administrative,
    /// غير معروف
    Unknown,
}

impl Intent {
    pub fn name(&self) -> &str {
        match self {
            Self::InformationalQuery => "informational_query",
            Self::TaskExecution => "task_execution",
            Self::CodeModification => "code_modification",
            Self::Analysis => "analysis",
            Self::ContentCreation => "content_creation",
            Self::Exploration => "exploration",
            Self::ErrorCorrection => "error_correction",
            Self::Clarification => "clarification",
            Self::Administrative => "administrative",
            Self::Unknown => "unknown",
        }
    }

    /// هل تتطلب هذه النية تنفيذ أدوات؟
    pub fn requires_tool_execution(&self) -> bool {
        matches!(self, Self::TaskExecution | Self::CodeModification | Self::Exploration | Self::ErrorCorrection)
    }

    /// هل تتطلب هذه النية تفكيراً عميقاً؟
    pub fn requires_deep_thinking(&self) -> bool {
        matches!(self, Self::Analysis | Self::ErrorCorrection | Self::TaskExecution)
    }
}

/// وحدة سياقية متتبعة عبر المحادثة
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextFrame {
    /// معرف الخطوة
    pub step_id: u64,
    /// النية المصنفة
    pub intent: Intent,
    /// المقتطف النصي
    pub text_snippet: String,
    /// الكلمات المفتاحية المستخرجة
    pub keywords: Vec<String>,
    /// الكيانات المذكورة (أسماء ملفات، أدوات، إلخ)
    pub entities: Vec<String>,
    /// الثقة بالتصنيف (0.0 - 1.0)
    pub confidence: f64,
    /// الطابع الزمني
    pub timestamp: String,
}

/// نتائج التحليل الدلالي
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticResult {
    pub intent: Intent,
    pub confidence: f64,
    pub keywords: Vec<String>,
    pub entities: Vec<String>,
    pub requires_clarification: bool,
    pub suggested_mode: String,
}

/// المحرك الدلالي الرئيسي
#[derive(Debug, Clone)]
pub struct SemanticEngine {
    /// كلمات مفتاحية → Intent
    keyword_map: Vec<(Vec<&'static str>, Intent)>,
    /// عتبة الثقة الدنيا
    confidence_threshold: f64,
    /// تاريخ السياق
    context_history: Vec<ContextFrame>,
}

impl Default for SemanticEngine {
    fn default() -> Self {
        Self {
            keyword_map: vec![
                (vec!["what", "why", "when", "where", "who", "how", "tell", "explain", "describe", "meaning", "difference"], Intent::InformationalQuery),
                (vec!["create", "build", "make", "generate", "implement", "add", "write", "produce", "develop"], Intent::TaskExecution),
                (vec!["fix", "change", "update", "modify", "edit", "refactor", "rename", "remove", "delete"], Intent::CodeModification),
                (vec!["analyze", "debug", "review", "check", "test", "examine", "inspect", "investigate"], Intent::Analysis),
                (vec!["design", "draw", "chart", "svg", "visualize", "plot", "diagram", "ui", "interface"], Intent::ContentCreation),
                (vec!["explore", "show", "list", "find", "search", "browse", "navigate"], Intent::Exploration),
                (vec!["error", "wrong", "bug", "issue", "problem", "crash", "fail", "mistake"], Intent::ErrorCorrection),
            ],
            confidence_threshold: 0.4,
            context_history: vec![],
        }
    }
}

impl SemanticEngine {
    pub fn new() -> Self { Self::default() }

    /// تحليل دلالي لنص الإدخال
    pub fn analyze(&mut self, text: &str, step_id: u64) -> SemanticResult {
        let lower = text.to_lowercase();
        let words: Vec<&str> = lower.split_whitespace().collect();
        let mut keywords: Vec<String> = words.iter()
            .filter(|w| w.len() > 3)
            .map(|w| w.to_string())
            .collect();
        keywords.dedup();

        // استخراج الكيانات (أسماء ملفات بامتدادات)
        let entities: Vec<String> = words.iter()
            .filter(|w| w.contains('.') && w.len() > 3)
            .map(|w| w.to_string())
            .collect();

        // تصنيف النية
        let (intent, confidence) = self.classify_intent(&lower, &keywords);

        let requires_clarification = confidence < self.confidence_threshold;
        let suggested_mode = self.suggest_mode(&intent);

        // حفظ في السياق
        let frame = ContextFrame {
            step_id,
            intent: intent.clone(),
            text_snippet: text.chars().take(200).collect(),
            keywords: keywords.clone(),
            entities: entities.clone(),
            confidence,
            timestamp: chrono::Utc::now().to_rfc3339(),
        };
        self.context_history.push(frame);
        if self.context_history.len() > 100 {
            self.context_history.remove(0);
        }

        SemanticResult {
            intent,
            confidence,
            keywords,
            entities,
            requires_clarification,
            suggested_mode,
        }
    }

    fn classify_intent(&self, lower: &str, keywords: &[String]) -> (Intent, f64) {
        let mut scores: Vec<(Intent, usize)> = vec![
            (Intent::InformationalQuery, 0),
            (Intent::TaskExecution, 0),
            (Intent::CodeModification, 0),
            (Intent::Analysis, 0),
            (Intent::ContentCreation, 0),
            (Intent::Exploration, 0),
            (Intent::ErrorCorrection, 0),
            (Intent::Clarification, 0),
            (Intent::Administrative, 0),
        ];

        for (kws, intent) in &self.keyword_map {
            let count = kws.iter().filter(|kw| lower.contains(*kw)).count();
            if count > 0 {
                if let Some(entry) = scores.iter_mut().find(|(i, _)| i == intent) {
                    entry.1 += count;
                }
            }
        }

        // كشف الاستفسار (هل) والجمل الاستفهامية
        if lower.contains('?') || lower.starts_with("هل") {
            if let Some(entry) = scores.iter_mut().find(|(i, _)| *i == Intent::InformationalQuery) {
                entry.1 += 2;
            }
        }

        // كشف التوضيح
        if keywords.iter().any(|k| k.contains("clarif") || k.contains("meaning") || k.contains("what do you")) {
            if let Some(entry) = scores.iter_mut().find(|(i, _)| *i == Intent::Clarification) {
                entry.1 += 3;
            }
        }

        let max_score = scores.iter().map(|(_, c)| *c).max().unwrap_or(0);
        let total_score: usize = scores.iter().map(|(_, c)| c).sum();

        if max_score == 0 {
            return (Intent::Unknown, 0.0);
        }

        let confidence = max_score as f64 / total_score.max(1) as f64;
        let best_intent = scores.into_iter().max_by_key(|(_, c)| *c).map(|(i, _)| i).unwrap_or(Intent::Unknown);
        (best_intent, confidence)
    }

    fn suggest_mode(&self, intent: &Intent) -> String {
        match intent {
            Intent::InformationalQuery => "tutorial".to_string(),
            Intent::TaskExecution => "autonomous".to_string(),
            Intent::CodeModification => "supervised".to_string(),
            Intent::Analysis => "audit".to_string(),
            Intent::ContentCreation => "autonomous".to_string(),
            Intent::Exploration => "tutorial".to_string(),
            Intent::ErrorCorrection => "supervised".to_string(),
            Intent::Clarification => "tutorial".to_string(),
            Intent::Administrative => "autonomous".to_string(),
            Intent::Unknown => "supervised".to_string(),
        }
    }

    /// التحقق من تناسق السياق (هل الـ intent يتغير بشكل غير طبيعي؟)
    pub fn validate_context_switch(&self, _new_intent: &Intent, threshold: usize) -> bool {
        if self.context_history.len() < 3 { return true; }
        let recent: Vec<&Intent> = self.context_history.iter().rev().take(3).map(|f| &f.intent).collect();
        let changes = recent.windows(2).filter(|w| w[0] != w[1]).count();
        changes <= threshold
    }

    pub fn context(&self) -> &[ContextFrame] { &self.context_history }
    pub fn set_threshold(&mut self, t: f64) { self.confidence_threshold = t; }
}
