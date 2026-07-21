//! # User Question Routes — أسئلة المستخدم
//!
//! POST /api/user/question             → الوكيل يطرح سؤالاً
//! GET  /api/user/question/:id         → عرض السؤال
//! PUT  /api/user/question/:id/answer  → المستخدم يجيب
//! GET  /api/user/question/pending     → الأسئلة المعلقة
//! POST /api/user/question/:id/cancel  → إلغاء السؤال
//! GET  /api/user/question/stats      → إحصائيات

use axum::{Extension, Json};
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::sync::RwLock;
use crate::routes::AuthUser;
use crate::agent::user_questions::{
    QuestionStore, AgentQuestion, QuestionType, QuestionPriority,
    QuestionOption, QuestionStatus, QuestionAnswer,
};

/// مشاركة مخزن الأسئلة
pub type SharedQuestionStore = Arc<RwLock<QuestionStore>>;

pub fn create_question_store(max: usize) -> SharedQuestionStore {
    Arc::new(RwLock::new(QuestionStore::new(max)))
}

/// POST /api/user/question
pub async fn ask_question(
    Extension(_auth): Extension<AuthUser>,
    Extension(store): Extension<SharedQuestionStore>,
    Json(body): Json<Value>,
) -> Json<Value> {
    let title = body["title"].as_str().unwrap_or("سؤال");
    let question_body = body["body"].as_str().unwrap_or("");
    let question_type_str = body["question_type"].as_str().unwrap_or("free_text");
    let allow_free_text = body["allow_free_text"].as_bool().unwrap_or(true);

    if question_body.is_empty() {
        return Json(json!({ "success": false, "error": "body مطلوب" }));
    }

    let question_type = match question_type_str {
        "multiple_choice" => QuestionType::MultipleChoice,
        "yes_no" => QuestionType::YesNo,
        "select_with_text" => QuestionType::SelectWithText,
        "confirmation" => QuestionType::Confirmation,
        _ => QuestionType::FreeText,
    };

    let options: Vec<QuestionOption> = body["options"].as_array()
        .map(|arr| arr.iter().filter_map(|o| {
            Some(QuestionOption {
                value: o["value"].as_str()?.to_string(),
                label: o["label"].as_str().unwrap_or("").to_string(),
                description: o["description"].as_str().map(String::from),
                recommended: o["recommended"].as_bool().unwrap_or(false),
            })
        }).collect())
        .unwrap_or_default();

    let mut question = AgentQuestion {
        id: String::new(),
        title: title.to_string(),
        body: question_body.to_string(),
        question_type,
        options,
        allow_free_text,
        context: body["context"].clone(),
        priority: match body["priority"].as_str().unwrap_or("medium") {
            "low" => QuestionPriority::Low,
            "high" => QuestionPriority::High,
            "critical" => QuestionPriority::Critical,
            _ => QuestionPriority::Medium,
        },
        status: QuestionStatus::Pending,
        created_at: String::new(),
        answered_at: None,
        answer: None,
        timeout_minutes: body["timeout_minutes"].as_u64().map(|t| t as u32),
        asked_by_agent_id: body["agent_id"].as_str().unwrap_or("unknown").to_string(),
    };

    let mut store = store.write().await;
    match store.add(&mut question) {
        Ok(id) => Json(json!({
            "success": true,
            "question_id": id,
            "question": question,
        })),
        Err(e) => Json(json!({ "success": false, "error": e })),
    }
}

/// GET /api/user/question/:id
pub async fn get_question(
    Extension(_auth): Extension<AuthUser>,
    Extension(store): Extension<SharedQuestionStore>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Json<Value> {
    let store = store.read().await;
    match store.get(&id) {
        Some(q) => Json(json!({ "success": true, "question": q })),
        None => Json(json!({ "success": false, "error": format!("السؤال {} غير موجود", id) })),
    }
}

/// PUT /api/user/question/:id/answer
pub async fn answer_question(
    Extension(_auth): Extension<AuthUser>,
    Extension(store): Extension<SharedQuestionStore>,
    axum::extract::Path(id): axum::extract::Path<String>,
    Json(body): Json<Value>,
) -> Json<Value> {
    let selected: Vec<String> = body["selected_options"].as_array()
        .map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect())
        .unwrap_or_default();

    let answer = QuestionAnswer {
        selected_options: selected,
        free_text: body["free_text"].as_str().map(String::from),
        timestamp: chrono::Utc::now().to_rfc3339(),
    };

    let mut store = store.write().await;
    match store.answer(&id, answer) {
        Ok(question) => Json(json!({
            "success": true,
            "question": question,
        })),
        Err(e) => Json(json!({ "success": false, "error": e })),
    }
}

/// GET /api/user/question/pending
pub async fn pending_questions(
    Extension(_auth): Extension<AuthUser>,
    Extension(store): Extension<SharedQuestionStore>,
) -> Json<Value> {
    let store = store.read().await;
    let pending = store.pending();
    Json(json!({
        "success": true,
        "count": pending.len(),
        "questions": pending,
    }))
}

/// POST /api/user/question/:id/cancel
pub async fn cancel_question(
    Extension(_auth): Extension<AuthUser>,
    Extension(store): Extension<SharedQuestionStore>,
    axum::extract::Path(id): axum::extract::Path<String>,
    Json(body): Json<Value>,
) -> Json<Value> {
    let reason = body["reason"].as_str().unwrap_or("ألغى المستخدم");
    let mut store = store.write().await;
    match store.cancel(&id, reason) {
        Ok(_) => Json(json!({ "success": true })),
        Err(e) => Json(json!({ "success": false, "error": e })),
    }
}

/// GET /api/user/question/stats
pub async fn question_stats(
    Extension(_auth): Extension<AuthUser>,
    Extension(store): Extension<SharedQuestionStore>,
) -> Json<Value> {
    let store = store.read().await;
    Json(json!({
        "success": true,
        "stats": store.stats(),
    }))
}
