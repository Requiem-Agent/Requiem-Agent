use axum::{
    extract::{Path, State},
    http::StatusCode,
    Extension, Json,
};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct SaveFileBody {
    pub content: String,
    pub name: Option<String>,
}
use serde_json::{json, Value};
use std::sync::Arc;
use crate::AppState;
use crate::storage;
use super::UserId;

#[derive(Deserialize)]
pub struct SaveFileReq {
    pub content: String,
}

/// POST /api/sessions/:id/files — save with filename in body
pub async fn save_file_body(
    State(_state): State<Arc<AppState>>,
    Extension(UserId(user_id)): Extension<UserId>,
    Path(session_id): Path<String>,
    Json(body): Json<SaveFileBody>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let file_name = body.name.as_deref().unwrap_or("unnamed");
    if file_name.contains("..") || file_name.contains('/') {
        return Err((StatusCode::BAD_REQUEST, Json(json!({"error":"Invalid filename"}))));
    }
    storage::save_file(&user_id, &session_id, file_name, &body.content)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error":e}))))?;
    Ok(Json(json!({"status":"ok","file":file_name})))
}

#[derive(Serialize)]
pub struct FileInfo {
    pub name: String,
}

/// POST /api/sessions/:id/files/:name — save a file
pub async fn save_file(
    State(_state): State<Arc<AppState>>,
    Extension(UserId(user_id)): Extension<UserId>,
    Path((session_id, file_name)): Path<(String, String)>,
    Json(body): Json<SaveFileReq>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    if file_name.contains("..") || file_name.contains('/') {
        return Err((StatusCode::BAD_REQUEST, Json(json!({"error":"Invalid filename"}))));
    }
    storage::save_file(&user_id, &session_id, &file_name, &body.content)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error":e}))))?;
    Ok(Json(json!({"status":"ok","file":file_name})))
}

/// GET /api/sessions/:id/files — list files
pub async fn list_files(
    State(_state): State<Arc<AppState>>,
    Extension(UserId(user_id)): Extension<UserId>,
    Path(session_id): Path<String>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let files = storage::list_files(&user_id, &session_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error":e}))))?;
    Ok(Json(json!({"files": files})))
}

/// GET /api/sessions/:id/files/:name — read a file
pub async fn get_file(
    State(_state): State<Arc<AppState>>,
    Extension(UserId(user_id)): Extension<UserId>,
    Path((session_id, file_name)): Path<(String, String)>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    if file_name.contains("..") || file_name.contains('/') {
        return Err((StatusCode::BAD_REQUEST, Json(json!({"error":"Invalid filename"}))));
    }
    let content = storage::read_file(&user_id, &session_id, &file_name)
        .await
        .map_err(|e| (StatusCode::NOT_FOUND, Json(json!({"error":e}))))?;
    Ok(Json(json!({"file":file_name,"content":content})))
}

/// DELETE /api/sessions/:id/files/:name — delete a file
pub async fn delete_file(
    State(_state): State<Arc<AppState>>,
    Extension(UserId(user_id)): Extension<UserId>,
    Path((session_id, file_name)): Path<(String, String)>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    if file_name.contains("..") || file_name.contains('/') {
        return Err((StatusCode::BAD_REQUEST, Json(json!({"error":"Invalid filename"}))));
    }
    storage::delete_file(&user_id, &session_id, &file_name)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error":e}))))?;
    Ok(Json(json!({"status":"deleted","file":file_name})))
}

/// GET /api/sessions/:id/context — load session context
pub async fn get_context(
    State(_state): State<Arc<AppState>>,
    Extension(UserId(user_id)): Extension<UserId>,
    Path(session_id): Path<String>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let ctx = storage::load_session_context(&user_id, &session_id)
        .await
        .map_err(|e| (StatusCode::NOT_FOUND, Json(json!({"error":e}))))?;
    let parsed: Value = serde_json::from_str(&ctx).unwrap_or(json!({}));
    Ok(Json(parsed))
}

/// POST /api/sessions/:id/context — save session context
pub async fn save_context(
    State(_state): State<Arc<AppState>>,
    Extension(UserId(user_id)): Extension<UserId>,
    Path(session_id): Path<String>,
    Json(body): Json<Value>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let ctx_str = serde_json::to_string(&body)
        .map_err(|e| (StatusCode::BAD_REQUEST, Json(json!({"error":e.to_string()}))))?;
    storage::save_session_context(&user_id, &session_id, &ctx_str)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error":e}))))?;
    Ok(Json(json!({"status":"saved"})))
}

// ─── Global File Routes (/api/files) ─────────────────────────────────────────
// Maps to session_id = "__global__" in storage layer

const GLOBAL_SESSION: &str = "__global__";

/// GET /api/files — list all global files for user
pub async fn list_user_files(
    State(_state): State<Arc<AppState>>,
    Extension(UserId(user_id)): Extension<UserId>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let files = storage::list_files(&user_id, GLOBAL_SESSION)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error":e}))))?;
    Ok(Json(json!({ "files": files })))
}

/// POST /api/files/upload — upload global file (JSON body: {name, content})
pub async fn upload_user_file(
    State(_state): State<Arc<AppState>>,
    Extension(UserId(user_id)): Extension<UserId>,
    headers: axum::http::HeaderMap,
    body: axum::body::Bytes,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let content_type = headers
        .get(axum::http::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    if content_type.contains("application/json") {
        // JSON body: { name, content }
        let v: serde_json::Value = serde_json::from_slice(&body)
            .map_err(|e| (StatusCode::BAD_REQUEST, Json(json!({"error": e.to_string()}))))?;
        let name = v["name"].as_str().unwrap_or("unnamed.txt");
        let content = v["content"].as_str().unwrap_or("");
        if name.contains("..") || name.contains('/') {
            return Err((StatusCode::BAD_REQUEST, Json(json!({"error":"Invalid filename"}))));
        }
        storage::save_file(&user_id, GLOBAL_SESSION, name, content)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error":e}))))?;
        return Ok(Json(json!({ "status": "ok", "file": name })));
    }

    // Multipart/form-data: parse boundary manually
    let boundary = content_type.split("boundary=").nth(1).unwrap_or("").trim();
    if boundary.is_empty() {
        return Err((StatusCode::BAD_REQUEST, Json(json!({"error":"Missing boundary or content-type"}))));
    }
    let raw = String::from_utf8_lossy(&body);
    let sep = format!("--{boundary}");
    let mut saved: Vec<String> = Vec::new();

    for part in raw.split(&sep).skip(1) {
        if part.trim_start().starts_with("--") { continue; }
        let Some(header_end) = part.find("\r\n\r\n") else { continue; };
        let header = &part[..header_end];
        let body_part = &part[header_end + 4..];
        let body_trimmed = body_part.trim_end_matches("\r\n");

        let filename = header.lines()
            .find(|l| l.contains("filename="))
            .and_then(|l| l.split("filename=").nth(1))
            .map(|s| s.trim_matches('"').trim_matches('\'').to_string())
            .unwrap_or_else(|| format!("file_{}.txt", uuid::Uuid::new_v4()));

        if filename.contains("..") || filename.contains('/') { continue; }
        storage::save_file(&user_id, GLOBAL_SESSION, &filename, body_trimmed)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error":e}))))?;
        saved.push(filename);
    }

    if saved.is_empty() {
        return Err((StatusCode::BAD_REQUEST, Json(json!({"error":"No files found in request"}))));
    }
    Ok(Json(json!({ "files": saved, "count": saved.len() })))
}

/// GET /api/files/:name — read global file
pub async fn get_user_file(
    State(_state): State<Arc<AppState>>,
    Extension(UserId(user_id)): Extension<UserId>,
    Path(file_name): Path<String>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    if file_name.contains("..") || file_name.contains('/') {
        return Err((StatusCode::BAD_REQUEST, Json(json!({"error":"Invalid filename"}))));
    }
    let content = storage::read_file(&user_id, GLOBAL_SESSION, &file_name)
        .await
        .map_err(|e| (StatusCode::NOT_FOUND, Json(json!({"error":e}))))?;
    Ok(Json(json!({ "name": file_name, "content": content })))
}

/// DELETE /api/files/:name — delete global file
pub async fn delete_user_file(
    State(_state): State<Arc<AppState>>,
    Extension(UserId(user_id)): Extension<UserId>,
    Path(file_name): Path<String>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    if file_name.contains("..") || file_name.contains('/') {
        return Err((StatusCode::BAD_REQUEST, Json(json!({"error":"Invalid filename"}))));
    }
    storage::delete_file(&user_id, GLOBAL_SESSION, &file_name)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error":e}))))?;
    Ok(Json(json!({ "status": "deleted", "file": file_name })))
}

/// GET /api/usage — get user storage + quota usage
pub async fn get_storage_usage(
    State(_state): State<Arc<AppState>>,
    Extension(UserId(user_id)): Extension<UserId>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let size = storage::user_storage_usage(&user_id).await.unwrap_or(0);
    let sessions = storage::list_user_sessions(&user_id).await.unwrap_or_default();
    let files_count = {
        let mut total = 0usize;
        for sid in &sessions {
            if let Ok(files) = storage::list_files(&user_id, sid).await {
                total += files.len();
            }
        }
        total
    };
    Ok(Json(json!({
        "storageBytes": size,
        "storageKB": size / 1024,
        "sessionCount": sessions.len(),
        "totalFiles": files_count,
    })))
}
