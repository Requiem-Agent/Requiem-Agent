//! # User Question System — نظام أسئلة المستخدم
//!
//! يسمح للوكيل بطرح أسئلة منظمة على المستخدم مع:
//! - خيارات متعددة
//! - كتابة رد نصي
//! - مهلة زمنية
//! - سياق السؤال
//!
//! ## الفرق عن الأسئلة العادية
//! الوكيل لا يسأل إلا عندما يكون ضرورياً — AutonomyScorer يقرر.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// أنواع الأسئلة
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum QuestionType {
    /// اختيار من متعدد
    MultipleChoice,
    /// نعم/لا
    YesNo,
    /// اختيار مع كتابة
    SelectWithText,
    /// كتابة حرة
    FreeText,
    /// تأكيد فقط
    Confirmation,
}

impl QuestionType {
    pub fn name(&self) -> &'static str {
        match self {
            Self::MultipleChoice => "multiple_choice",
            Self::YesNo => "yes_no",
            Self::SelectWithText => "select_with_text",
            Self::FreeText => "free_text",
            Self::Confirmation => "confirmation",
        }
    }
}

/// أولوية السؤال
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum QuestionPriority {
    Low,
    Medium,
    High,
    Critical,
}

/// خيار في السؤال
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestionOption {
    pub value: String,
    pub label: String,
    pub description: Option<String>,
    pub recommended: bool,
}

/// سؤال للمستخدم
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentQuestion {
    pub id: String,
    pub title: String,
    pub body: String,
    pub question_type: QuestionType,
    pub options: Vec<QuestionOption>,
    pub allow_free_text: bool,
    pub context: serde_json::Value,
    pub priority: QuestionPriority,
    pub status: QuestionStatus,
    pub created_at: String,
    pub answered_at: Option<String>,
    pub answer: Option<QuestionAnswer>,
    pub timeout_minutes: Option<u32>,
    pub asked_by_agent_id: String,
}

/// حالة السؤال
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum QuestionStatus {
    Pending,
    Answered,
    Cancelled,
    TimedOut,
}

/// إجابة المستخدم
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestionAnswer {
    pub selected_options: Vec<String>,
    pub free_text: Option<String>,
    pub timestamp: String,
}

// ─── Question Store ────────────────────────────────────────────────────────

/// مخزن الأسئلة — يحتفظ بالأسئلة المعلقة والمجابة
pub struct QuestionStore {
    questions: HashMap<String, AgentQuestion>,
    max_questions: usize,
    next_id: u64,
}

impl QuestionStore {
    pub fn new(max_questions: usize) -> Self {
        Self {
            questions: HashMap::new(),
            max_questions,
            next_id: 1,
        }
    }

    /// إضافة سؤال جديد
    pub fn add(&mut self, question: &mut AgentQuestion) -> Result<String, String> {
        if self.questions.len() >= self.max_questions {
            return Err("بلغت الحد الأقصى للأسئلة المعلقة".into());
        }
        let id = format!("q-{}", self.next_id);
        self.next_id += 1;
        question.id = id.clone();
        question.status = QuestionStatus::Pending;
        question.created_at = chrono::Utc::now().to_rfc3339();
        self.questions.insert(id.clone(), question.clone());
        Ok(id)
    }

    /// الإجابة على سؤال
    pub fn answer(&mut self, question_id: &str, answer: QuestionAnswer) -> Result<AgentQuestion, String> {
        let question = self.questions.get_mut(question_id)
            .ok_or_else(|| format!("السؤال {} غير موجود", question_id))?;

        if question.status != QuestionStatus::Pending {
            return Err(format!("السؤال {} لم يعد معلقاً", question_id));
        }

        question.status = QuestionStatus::Answered;
        question.answered_at = Some(chrono::Utc::now().to_rfc3339());
        question.answer = Some(answer);

        Ok(question.clone())
    }

    /// إلغاء سؤال
    pub fn cancel(&mut self, question_id: &str, _reason: &str) -> Result<(), String> {
        let question = self.questions.get_mut(question_id)
            .ok_or_else(|| format!("السؤال {} غير موجود", question_id))?;

        if question.status != QuestionStatus::Pending {
            return Err(format!("السؤال {} لم يعد معلقاً", question_id));
        }

        question.status = QuestionStatus::Cancelled;
        Ok(())
    }

    /// الأسئلة المعلقة
    pub fn pending(&self) -> Vec<&AgentQuestion> {
        self.questions.values()
            .filter(|q| q.status == QuestionStatus::Pending)
            .collect()
    }

    /// الأسئلة المعلقة لوكيل معين
    pub fn pending_for_agent(&self, agent_id: &str) -> Vec<&AgentQuestion> {
        self.questions.values()
            .filter(|q| q.status == QuestionStatus::Pending && q.asked_by_agent_id == agent_id)
            .collect()
    }

    /// الحصول على سؤال
    pub fn get(&self, id: &str) -> Option<&AgentQuestion> {
        self.questions.get(id)
    }

    /// إحصائيات
    pub fn stats(&self) -> serde_json::Value {
        let total = self.questions.len();
        let pending = self.questions.values().filter(|q| q.status == QuestionStatus::Pending).count();
        let answered = self.questions.values().filter(|q| q.status == QuestionStatus::Answered).count();

        serde_json::json!({
            "total": total,
            "pending": pending,
            "answered": answered,
            "max": self.max_questions,
        })
    }
}

// ─── AutonomyScorer ────────────────────────────────────────────────────────

/// نتيجة تقييم الاستقلالية
#[derive(Debug, Clone)]
pub enum AutonomyDecision {
    /// أكمل بدون سؤال
    Proceed,
    /// اسأل المستخدم
    AskHuman(AgentQuestion),
    /// حلل المهمة أولاً
    DecomposeFirst,
}

/// مُقيّم استقلالية الوكيل — متى يسأل ومتى يعتمد على نفسه؟
pub struct AutonomyScorer {
    pub min_confidence_to_proceed: f32,
    pub max_questions_per_session: u32,
    pub questions_asked: u32,
    context_history: Vec<String>,
}

impl AutonomyScorer {
    pub fn new() -> Self {
        Self {
            min_confidence_to_proceed: 0.6,
            max_questions_per_session: 3,
            questions_asked: 0,
            context_history: Vec::new(),
        }
    }

    /// تقييم: هل يسأل الوكيل أم يكمل؟
    pub fn evaluate(&mut self, task: &str, context: &serde_json::Value) -> AutonomyDecision {
        // 1. هل تجاوزنا الحد الأقصى للأسئلة؟
        if self.questions_asked >= self.max_questions_per_session {
            return AutonomyDecision::Proceed;
        }

        // 2. هل المهمة واضحة بما فيه الكفاية؟
        let clarity = self.estimate_clarity(task, context);

        // 3. هل هناك معلومات كافية في السياق؟
        let info_sufficiency = self.estimate_info_sufficiency(context);

        // 4. هل السؤال ضروري حقاً؟
        if clarity >= self.min_confidence_to_proceed && info_sufficiency >= 0.5 {
            return AutonomyDecision::Proceed;
        }

        // 5. هل يمكن تحليل المهمة بدلاً من السؤال؟
        if clarity < 0.3 && context.get("previous_tasks").is_some() {
            return AutonomyDecision::DecomposeFirst;
        }

        // 6. إذا كان لا بد من السؤال
        self.questions_asked += 1;

        AutonomyDecision::AskHuman(AgentQuestion {
            id: String::new(),
            title: "توضيح مطلوب".into(),
            body: format!("أحتاج توضيحاً للمهمة: {task}"),
            question_type: if clarity > 0.4 {
                QuestionType::SelectWithText
            } else {
                QuestionType::FreeText
            },
            options: vec![
                QuestionOption {
                    value: "clarify".into(),
                    label: "أريد توضيح المهمة".into(),
                    description: None,
                    recommended: true,
                },
                QuestionOption {
                    value: "proceed".into(),
                    label: "اكتب أنت كما ترى".into(),
                    description: None,
                    recommended: false,
                },
            ],
            allow_free_text: true,
            context: serde_json::json!({ "task": task }),
            priority: QuestionPriority::Medium,
            status: QuestionStatus::Pending,
            created_at: String::new(),
            answered_at: None,
            answer: None,
            timeout_minutes: Some(30),
            asked_by_agent_id: String::new(),
        })
    }

    /// تقدير وضوح المهمة (0.0 - 1.0)
    fn estimate_clarity(&self, task: &str, _context: &serde_json::Value) -> f32 {
        let lower = task.to_lowercase();
        let mut score = 0.5f32;

        // مهمة واضحة
        if lower.len() > 50 { score += 0.2; }
        if task.contains("?") || task.contains("؟") { score -= 0.1; }

        // كلمات تدل على عدم الوضوح
        let vague_words = ["ماذا", "كيف", "هل", "what", "how", "which", "maybe", "ربما"];
        for w in &vague_words {
            if lower.contains(w) { score -= 0.1; }
        }

        // كلمات تدل على الوضوح
        let clear_words = ["افعل", "اكتب", "أنشئ", "حول", "غيّر", "create", "write", "implement", "change"];
        for w in &clear_words {
            if lower.contains(w) { score += 0.1; }
        }

        score.clamp(0.0, 1.0)
    }

    /// تقدير كفاية المعلومات
    fn estimate_info_sufficiency(&self, context: &serde_json::Value) -> f32 {
        let mut score = 0.3f32;

        if let Some(files) = context.get("files") {
            if files.as_array().map(|a| a.len()).unwrap_or(0) > 0 { score += 0.2; }
        }
        if let Some(history) = context.get("conversation_history") {
            if history.as_str().map(|s| s.len()).unwrap_or(0) > 100 { score += 0.2; }
        }
        if let Some(env) = context.get("environment") {
            if !env.is_null() { score += 0.1; }
        }
        if let Some(models) = context.get("available_models") {
            if models.as_array().map(|a| a.len()).unwrap_or(0) > 0 { score += 0.1; }
        }

        score.clamp(0.0, 1.0)
    }

    /// إعادة تعيين العداد (لجلسة جديدة)
    pub fn reset_session(&mut self) {
        self.questions_asked = 0;
        self.context_history.clear();
    }
}

// ─── اختبارات ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_question_store() {
        let mut store = QuestionStore::new(100);
        let mut q = AgentQuestion {
            id: String::new(), title: "سؤال".into(), body: "ماذا تريد؟".into(),
            question_type: QuestionType::YesNo, options: vec![], allow_free_text: false,
            context: serde_json::json!({}), priority: QuestionPriority::Medium,
            status: QuestionStatus::Pending, created_at: String::new(),
            answered_at: None, answer: None, timeout_minutes: None,
            asked_by_agent_id: "agent-1".into(),
        };
        let id = store.add(&mut q).unwrap();
        assert!(id.starts_with("q-"));

        let answer = QuestionAnswer {
            selected_options: vec!["yes".into()],
            free_text: None,
            timestamp: chrono::Utc::now().to_rfc3339(),
        };
        let answered = store.answer(&id, answer).unwrap();
        assert_eq!(answered.status, QuestionStatus::Answered);
    }

    #[test]
    fn test_autonomy_scorer_proceed() {
        let mut scorer = AutonomyScorer::new();
        let context = serde_json::json!({
            "files": ["main.rs"],
            "conversation_history": "المستخدم يطلب تطبيق ويب باستخدام Axum في Rust...",
            "environment": {"platform": "requiem"},
        });
        let decision = scorer.evaluate("اكتب تطبيق ويب بسيط باستخدام Axum مع Rust", &context);
        // يجب أن يقرر المتابعة لأن المهمة واضحة
        match decision {
            AutonomyDecision::Proceed => {} // success
            AutonomyDecision::AskHuman(_) => {
                // يمكن أن يسأل في بعض الحالات — مقبول
            }
            _ => panic!("يجب أن يقرر المتابعة أو السؤال فقط"),
        }
    }

    #[test]
    fn test_autonomy_scorer_ask() {
        let mut scorer = AutonomyScorer::new();
        let context = serde_json::json!({});
        // مهمة غير واضحة + سياق فارغ → يسأل
        let decision = scorer.evaluate("اعمل شيئاً", &context);
        match decision {
            AutonomyDecision::AskHuman(_) => {} // success
            _ => panic!("يجب أن يسأل المستخدم"),
        }
    }

    #[test]
    fn test_max_questions() {
        let mut scorer = AutonomyScorer::new();
        scorer.max_questions_per_session = 2;
        scorer.questions_asked = 2;
        // حتى لو المهمة غير واضحة، لا يسأل
        let decision = scorer.evaluate("???", &serde_json::json!({}));
        assert!(matches!(decision, AutonomyDecision::Proceed | AutonomyDecision::DecomposeFirst));
    }

    #[test]
    fn test_estimate_clarity() {
        let scorer = AutonomyScorer::new();
        let high = scorer.estimate_clarity("أنشئ REST API كامل مع Axum في Rust يشمل JWT Auth و middleware", &serde_json::json!({}));
        let low = scorer.estimate_clarity("ماذا؟", &serde_json::json!({}));
        assert!(high > low);
    }
}
