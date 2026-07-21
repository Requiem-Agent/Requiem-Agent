//! # Tools Route — يعرض الأدوات المتاحة للوكيل
//!
//! GET  /api/tools → قائمة جميع الأدوات مع JSON Schema
//! POST /api/tools/validate → التحقق من صحة معاملات الأداة

use axum::{Extension, Json};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use crate::tools::{ToolRegistry, JsonSchema, Strictness};
use crate::routes::AuthUser;

/// تعريف أداة للإخراج
#[derive(Serialize)]
pub struct ToolResponse {
    pub name: String,
    pub description: String,
    pub parameters: JsonSchema,
    pub returns: JsonSchema,
    pub strictness: String,
}

/// إحصائيات الأدوات
#[derive(Serialize)]
pub struct ToolsStats {
    pub total: usize,
    pub by_strictness: Vec<(String, usize)>,
    pub names: Vec<String>,
}

/// GET /api/tools — قائمة بجميع الأدوات
pub async fn list_tools(
    Extension(_auth): Extension<AuthUser>,
) -> Json<Value> {
    let registry = ToolRegistry::new();
    let tools: Vec<ToolResponse> = registry.list_all().iter().map(|t| {
        ToolResponse {
            name: t.name.clone(),
            description: t.description.clone(),
            parameters: t.parameters.clone(),
            returns: t.returns.clone(),
            strictness: format!("{:?}", t.strictness),
        }
    }).collect();

    let stats = ToolsStats {
        total: registry.count(),
        by_strictness: vec![
            ("Normal".into(), registry.list_all().iter().filter(|t| t.strictness == Strictness::Normal).count()),
            ("Strict".into(), registry.list_all().iter().filter(|t| t.strictness == Strictness::Strict).count()),
            ("Critical".into(), registry.list_all().iter().filter(|t| t.strictness == Strictness::Critical).count()),
        ],
        names: registry.list_all().iter().map(|t| t.name.clone()).collect(),
    };

    Json(json!({
        "tools": tools,
        "stats": stats,
        "openai_format": registry.to_openai_format(),
    }))
}

/// POST /api/tools/validate — التحقق من صحة معاملات الأداة
#[derive(Deserialize)]
pub struct ValidateRequest {
    pub tool: String,
    pub params: Value,
}

pub async fn validate_tool(
    Extension(_auth): Extension<AuthUser>,
    Json(body): Json<ValidateRequest>,
) -> Json<Value> {
    let registry = ToolRegistry::new();

    match registry.validate_params(&body.tool, &body.params) {
        Ok(_) => Json(json!({
            "valid": true,
            "tool": body.tool,
        })),
        Err(e) => Json(json!({
            "valid": false,
            "tool": body.tool,
            "error": e,
        })),
    }
}

/// GET /api/orchestrator/classify?q=... — يصنف طلب المستخدم
pub async fn classify_task(
    Extension(_auth): Extension<AuthUser>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Json<Value> {
    use crate::orchestrator::{TaskClassifier, TaskCategory, Effort};

    let query = params.get("q").map(|s| s.as_str()).unwrap_or("");
    let effort = params.get("effort").map(|s| s.as_str()).unwrap_or("medium");

    let category = TaskClassifier::classify(query);
    let effort_level = match effort {
        "lite" => Effort::Lite,
        "high" => Effort::High,
        "max" => Effort::Max,
        _ => Effort::Medium,
    };

    let models = TaskClassifier::suggest_models(category, effort_level);

    Json(json!({
        "query": query,
        "category": category.to_string(),
        "effort": effort_level.to_string(),
        "suggested_models": models,
        "model_count": models.len(),
    }))
}

// ─── POST /api/tools/diff ────────────────────────────────────────────────────

/// طلب مقارنة نصين
#[derive(Deserialize)]
pub struct DiffRequest {
    pub old_text: String,
    pub new_text: String,
}

/// POST /api/tools/diff — مقارنة نصين وإنتاج diff
pub async fn diff_texts(
    Extension(_auth): Extension<AuthUser>,
    Json(body): Json<DiffRequest>,
) -> Json<Value> {
    use crate::tools::diff_tools::compare_texts;

    match compare_texts(&body.old_text, &body.new_text).await {
        Ok(result) => Json(json!({
            "success": true,
            "file": result.file,
            "changes": result.changes.iter().map(|c| json!({
                "type": format!("{:?}", c.line_type),
                "content": c.content,
                "old_line": c.old_line_num,
                "new_line": c.new_line_num,
            })).collect::<Vec<_>>(),
            "stats": {
                "lines_added": result.stats.lines_added,
                "lines_removed": result.stats.lines_removed,
                "lines_modified": result.stats.lines_modified,
            }
        })),
        Err(e) => Json(json!({
            "success": false,
            "error": e,
        })),
    }
}

// ─── POST /api/tools/parse ───────────────────────────────────────────────────

/// طلب تحليل AST لكود أو ملف
#[derive(Deserialize)]
pub struct ParseRequest {
    pub code: Option<String>,
    pub file_path: Option<String>,
    #[serde(default = "default_language")]
    pub language: String,
}

fn default_language() -> String { "rust".to_string() }

/// POST /api/tools/parse — تحليل AST
pub async fn parse_code_handler(
    Extension(_auth): Extension<AuthUser>,
    Json(body): Json<ParseRequest>,
) -> Json<Value> {
    use crate::tools::parser_tools::{parse_code, extract_functions, extract_classes};

    // قراءة الكود من الملف أو من الطلب مباشرة
    let code = match (&body.code, &body.file_path) {
        (Some(c), _) => c.clone(),
        (None, Some(fp)) => {
            match std::fs::read_to_string(fp) {
                Ok(content) => content,
                Err(e) => return Json(json!({ "success": false, "error": format!("Cannot read file: {e}") })),
            }
        }
        (None, None) => return Json(json!({ "success": false, "error": "Provide either 'code' or 'file_path'" })),
    };

    let lang = body.language.as_str();

    let (ast, functions, classes) = tokio::join!(
        parse_code(&code, lang),
        extract_functions(&code, lang),
        extract_classes(&code, lang),
    );

    Json(json!({
        "success": true,
        "language": lang,
        "ast": ast.ok(),
        "functions": functions.unwrap_or_default().iter().map(|f| json!({
            "name": f.name,
            "start_line": f.start_line,
            "end_line": f.end_line,
        })).collect::<Vec<_>>(),
        "classes": classes.unwrap_or_default().iter().map(|c| json!({
            "name": c.name,
            "start_line": c.start_line,
            "end_line": c.end_line,
        })).collect::<Vec<_>>(),
        "line_count": code.lines().count(),
    }))
}
