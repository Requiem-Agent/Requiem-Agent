//! # Formats Routes — API للتنسيقات والرسوم البيانية
//!
//! GET  /api/formats              → قائمة التنسيقات المدعومة
//! POST /api/formats/:name/validate → تحقق من صحة ملف
//! POST /api/formats/:name/format  → تنسيق ملف
//! POST /api/formats/:name/convert → تحويل إلى JSON
//! POST /api/formats/detect        → كشف التنسيق من اسم الملف
//! POST /api/formats/chart         → توليد SVG chart
//! POST /api/formats/sql/exec      → تنفيذ SQL

use axum::Json;
use serde_json::{json, Value};
use crate::formats::{FormatRegistry, sql_fmt};

/// GET /api/formats
pub async fn list_formats() -> Json<Value> {
    let reg = FormatRegistry::new();
    Json(json!({
        "formats": reg.list(),
        "count": reg.list().len(),
    }))
}

/// POST /api/formats/detect
pub async fn detect_format(Json(body): Json<Value>) -> Json<Value> {
    let filename = body["filename"].as_str().unwrap_or("");
    if filename.is_empty() {
        return Json(json!({ "success": false, "error": "filename مطلوب" }));
    }
    let reg = FormatRegistry::new();
    match reg.detect(filename) {
        Some(h) => Json(json!({
            "success": true,
            "format": h.name(),
            "extensions": h.extensions(),
        })),
        None => Json(json!({
            "success": false,
            "format": "unknown",
            "error": format!("التنسيق '{}' غير مدعوم", get_ext(filename)),
        })),
    }
}

fn get_ext(filename: &str) -> String {
    filename.rsplit('.').next().unwrap_or("").to_lowercase()
}

/// POST /api/formats/:name/validate
pub async fn validate_format(
    axum::extract::Path(name): axum::extract::Path<String>,
    Json(body): Json<Value>,
) -> Json<Value> {
    let reg = FormatRegistry::new();
    let content = body["content"].as_str().unwrap_or("");
    if content.is_empty() {
        return Json(json!({ "success": false, "error": "content مطلوب" }));
    }
    match reg.get(&name) {
        Some(h) => match h.validate(content) {
            Ok(msg) => Json(json!({ "success": true, "valid": true, "message": msg, "format": name })),
            Err(e) => Json(json!({ "success": true, "valid": false, "error": e, "format": name })),
        },
        None => Json(json!({ "success": false, "error": format!("تنسيق '{}' غير مدعوم", name) })),
    }
}

/// POST /api/formats/:name/format
pub async fn format_content(
    axum::extract::Path(name): axum::extract::Path<String>,
    Json(body): Json<Value>,
) -> Json<Value> {
    let reg = FormatRegistry::new();
    let content = body["content"].as_str().unwrap_or("");
    if content.is_empty() {
        return Json(json!({ "success": false, "error": "content مطلوب" }));
    }
    match reg.get(&name) {
        Some(h) => match h.format(content) {
            Ok(formatted) => Json(json!({
                "success": true,
                "format": name,
                "original_size": content.len(),
                "formatted_size": formatted.len(),
                "content": formatted,
            })),
            Err(e) => Json(json!({ "success": false, "error": e })),
        },
        None => Json(json!({ "success": false, "error": format!("تنسيق '{}' غير مدعوم", name) })),
    }
}

/// POST /api/formats/:name/convert
pub async fn convert_format(
    axum::extract::Path(name): axum::extract::Path<String>,
    Json(body): Json<Value>,
) -> Json<Value> {
    let reg = FormatRegistry::new();
    let content = body["content"].as_str().unwrap_or("");
    if content.is_empty() {
        return Json(json!({ "success": false, "error": "content مطلوب" }));
    }
    match reg.get(&name) {
        Some(h) => match h.convert_to_json(content) {
            Ok(json_out) => Json(json!({
                "success": true,
                "from": name,
                "to": "json",
                "content": serde_json::from_str(&json_out).unwrap_or(json!({"raw": json_out})),
            })),
            Err(e) => Json(json!({ "success": false, "error": e })),
        },
        None => Json(json!({ "success": false, "error": format!("تنسيق '{}' غير مدعوم", name) })),
    }
}

/// POST /api/formats/markdown/preview
pub async fn preview_markdown(Json(body): Json<Value>) -> Json<Value> {
    let content = body["content"].as_str().unwrap_or("");
    if content.is_empty() {
        return Json(json!({ "success": false, "error": "content مطلوب" }));
    }
    let html = crate::formats::markdown_fmt::render_markdown_to_html(content);
    Json(json!({
        "success": true,
        "html": html,
        "raw": content,
    }))
}

// TODO: Fix svg_charts raw string issues with # characters
// /// POST /api/formats/chart — توليد SVG chart
// pub async fn generate_chart(Json(body): Json<Value>) -> Json<Value> {
//     let chart_req: Result<svg_charts::ChartRequest, _> = serde_json::from_value(body.clone());
//     match chart_req {
//         Ok(req) => match svg_charts::render_chart(&req) {
//             Ok(svg) => Json(json!({
//                 "success": true,
//                 "svg": svg,
//                 "chart_type": format!("{:?}", req.chart_type).to_lowercase(),
//                 "title": req.title,
//             })),
//             Err(e) => Json(json!({ "success": false, "error": e })),
//         },
//         Err(e) => Json(json!({ "success": false, "error": format!("طلب chart غير صالح: {e}") })),
//     }
// }

/// POST /api/formats/sql/exec — تنفيذ SQL
pub async fn execute_sql(
    Json(body): Json<Value>,
) -> Json<Value> {
    let query = body["query"].as_str().unwrap_or("");
    let db_url = body["db_url"].as_str().unwrap_or("");
    let db_token = body["db_token"].as_str();

    if query.is_empty() {
        return Json(json!({ "success": false, "error": "query مطلوب" }));
    }

    match sql_fmt::execute_sql(db_url, db_token, query).await {
        Ok(result) => Json(json!({
            "success": true,
            "result": result,
        })),
        Err(e) => Json(json!({ "success": false, "error": e })),
    }
}

/// POST /api/formats/csv/table — CSV ← HTML table
pub async fn csv_to_table(Json(body): Json<Value>) -> Json<Value> {
    let content = body["content"].as_str().unwrap_or("");
    let title = body["title"].as_str().unwrap_or("CSV Table");
    if content.is_empty() {
        return Json(json!({ "success": false, "error": "content مطلوب" }));
    }
    match crate::formats::csv_fmt::csv_to_html_table(content, title) {
        Ok(html) => Json(json!({ "success": true, "html": html })),
        Err(e) => Json(json!({ "success": false, "error": e })),
    }
}
