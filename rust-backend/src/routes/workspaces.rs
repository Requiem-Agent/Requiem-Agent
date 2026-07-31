//! # Workspace Routes
//!
//! Manages per-user workspaces (projects). Each workspace has its own file tree
//! stored at: `users/{user_id}/workspaces/{workspace_id}/`
//!
//! All routes are protected by the auth middleware defined in routes/mod.rs.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Extension, Json,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;
use tracing::warn;
use uuid::Uuid;

use crate::AppState;
use crate::storage::workspace as ws_store;
use super::UserId;

// ─── Request / Response Types ─────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct CreateWorkspaceBody {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Deserialize)]
pub struct UpdateWorkspaceBody {
    pub name: Option<String>,
    pub description: Option<String>,
}

#[derive(Deserialize)]
pub struct WriteFileBody {
    pub content: String,
}

#[derive(Deserialize)]
pub struct CloneRepoBody {
    pub url: String,
}

// ─── Helper ───────────────────────────────────────────────────────────────────

/// Converts a storage error into a 500 JSON response.
fn internal(e: String) -> (StatusCode, Json<Value>) {
    (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e})))
}

/// Converts a "not found" message into a 404 JSON response.
fn not_found(e: String) -> (StatusCode, Json<Value>) {
    (StatusCode::NOT_FOUND, Json(json!({"error": e})))
}

/// Converts a bad-request message into a 400 JSON response.
fn bad_req(msg: &str) -> (StatusCode, Json<Value>) {
    (StatusCode::BAD_REQUEST, Json(json!({"error": msg})))
}

// ─── Workspace CRUD ───────────────────────────────────────────────────────────

/// GET /workspaces — list all workspaces for the authenticated user.
pub async fn list_workspaces(
    State(_state): State<Arc<AppState>>,
    Extension(UserId(user_id)): Extension<UserId>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let workspaces = ws_store::workspace_list(&user_id)
        .await
        .map_err(internal)?;
    Ok(Json(json!({ "workspaces": workspaces })))
}

/// POST /workspaces — create a new workspace.
pub async fn create_workspace(
    State(_state): State<Arc<AppState>>,
    Extension(UserId(user_id)): Extension<UserId>,
    Json(body): Json<CreateWorkspaceBody>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    if body.name.trim().is_empty() {
        return Err(bad_req("name must not be empty"));
    }
    let workspace_id = Uuid::new_v4().to_string();
    let description = body.description.as_deref().unwrap_or("");
    ws_store::workspace_create(&user_id, &workspace_id, &body.name, description)
        .await
        .map_err(internal)?;
    let meta = ws_store::workspace_get(&user_id, &workspace_id)
        .await
        .map_err(internal)?;
    Ok(Json(json!({ "workspace": meta })))
}

/// GET /workspaces/{id} — get a single workspace's metadata.
pub async fn get_workspace(
    State(_state): State<Arc<AppState>>,
    Extension(UserId(user_id)): Extension<UserId>,
    Path(workspace_id): Path<String>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let meta = ws_store::workspace_get(&user_id, &workspace_id)
        .await
        .map_err(|e| not_found(e))?;
    Ok(Json(json!({ "workspace": meta })))
}

/// PATCH /workspaces/{id} — rename or update description.
pub async fn update_workspace(
    State(_state): State<Arc<AppState>>,
    Extension(UserId(user_id)): Extension<UserId>,
    Path(workspace_id): Path<String>,
    Json(body): Json<UpdateWorkspaceBody>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    if body.name.is_none() && body.description.is_none() {
        return Err(bad_req("provide at least one of: name, description"));
    }
    let meta = ws_store::workspace_update(
        &user_id,
        &workspace_id,
        body.name.as_deref(),
        body.description.as_deref(),
    )
    .await
    .map_err(internal)?;
    Ok(Json(json!({ "workspace": meta })))
}

/// DELETE /workspaces/{id} — delete workspace and all its files.
pub async fn delete_workspace(
    State(_state): State<Arc<AppState>>,
    Extension(UserId(user_id)): Extension<UserId>,
    Path(workspace_id): Path<String>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    ws_store::workspace_delete(&user_id, &workspace_id)
        .await
        .map_err(internal)?;
    Ok(Json(json!({ "status": "deleted", "id": workspace_id })))
}

// ─── File Tree ────────────────────────────────────────────────────────────────

/// GET /workspaces/{id}/tree — recursive file tree.
pub async fn get_tree(
    State(_state): State<Arc<AppState>>,
    Extension(UserId(user_id)): Extension<UserId>,
    Path(workspace_id): Path<String>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let meta = ws_store::workspace_get(&user_id, &workspace_id)
        .await
        .map_err(|e| not_found(e))?;
    let tree = ws_store::workspace_tree(&user_id, &workspace_id)
        .await
        .map_err(internal)?;
    Ok(Json(json!({
        "name": meta.name,
        "tree": tree
    })))
}

// ─── File Operations ──────────────────────────────────────────────────────────

/// GET /workspaces/{id}/files/*path — read a file at a nested path.
pub async fn read_file(
    State(_state): State<Arc<AppState>>,
    Extension(UserId(user_id)): Extension<UserId>,
    Path((workspace_id, file_path)): Path<(String, String)>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let content = ws_store::workspace_read_file(&user_id, &workspace_id, &file_path)
        .await
        .map_err(|e| not_found(e))?;
    Ok(Json(json!({
        "path": file_path,
        "content": content
    })))
}

/// PUT /workspaces/{id}/files/*path — write / create a file at a nested path.
pub async fn write_file(
    State(_state): State<Arc<AppState>>,
    Extension(UserId(user_id)): Extension<UserId>,
    Path((workspace_id, file_path)): Path<(String, String)>,
    Json(body): Json<WriteFileBody>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    ws_store::workspace_write_file(&user_id, &workspace_id, &file_path, &body.content)
        .await
        .map_err(internal)?;
    Ok(Json(json!({ "status": "ok", "path": file_path })))
}

/// DELETE /workspaces/{id}/files/*path — delete a file at a nested path.
pub async fn delete_file(
    State(_state): State<Arc<AppState>>,
    Extension(UserId(user_id)): Extension<UserId>,
    Path((workspace_id, file_path)): Path<(String, String)>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    ws_store::workspace_delete_file(&user_id, &workspace_id, &file_path)
        .await
        .map_err(internal)?;
    Ok(Json(json!({ "status": "deleted", "path": file_path })))
}

/// POST /workspaces/{id}/files/*path/mkdir — create a directory at a nested path.
///
/// Note: `mkdir` is a literal suffix so the full Axum path pattern is:
/// `/workspaces/{id}/files/*path` with the handler deciding to treat it as mkdir.
/// In main.rs this is registered separately as
/// `POST /workspaces/{id}/mkdir/*path` to avoid wildcard collisions.
pub async fn mkdir(
    State(_state): State<Arc<AppState>>,
    Extension(UserId(user_id)): Extension<UserId>,
    Path((workspace_id, dir_path)): Path<(String, String)>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    ws_store::workspace_mkdir(&user_id, &workspace_id, &dir_path)
        .await
        .map_err(internal)?;
    Ok(Json(json!({ "status": "created", "path": dir_path })))
}

// ─── Git Clone ────────────────────────────────────────────────────────────────

/// POST /workspaces/{id}/clone — clone a git repository into the workspace.
///
/// Spawns a background tokio task; returns immediately with status "cloning".
/// The workspace directory must exist (created via POST /workspaces) beforehand.
pub async fn clone_repo(
    State(_state): State<Arc<AppState>>,
    Extension(UserId(user_id)): Extension<UserId>,
    Path(workspace_id): Path<String>,
    Json(body): Json<CloneRepoBody>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    // Basic URL sanity — must start with http/https/git/ssh
    let url = body.url.trim().to_string();
    if url.is_empty() {
        return Err(bad_req("url must not be empty"));
    }
    let allowed_schemes = ["https://", "http://", "git@", "git://", "ssh://"];
    let looks_ok = allowed_schemes.iter().any(|s| url.starts_with(s));
    if !looks_ok {
        return Err(bad_req("url must start with https://, http://, git@, git://, or ssh://"));
    }

    // Workspace must already exist (meta.json present)
    ws_store::workspace_get(&user_id, &workspace_id)
        .await
        .map_err(|e| not_found(e))?;

    // Resolve clone target directory
    let data_base = resolve_data_base();
    let clone_dir = format!(
        "{}/users/{}/workspaces/{}",
        data_base, user_id, workspace_id
    );

    let url_clone = url.clone();
    let ws_clone = workspace_id.clone();

    tokio::spawn(async move {
        match tokio::process::Command::new("git")
            .args(["clone", &url_clone, &clone_dir])
            .output()
            .await
        {
            Ok(out) if out.status.success() => {
                tracing::info!("git clone OK: {ws_clone}");
            }
            Ok(out) => {
                warn!(
                    "git clone failed for {ws_clone}: {}",
                    String::from_utf8_lossy(&out.stderr)
                );
            }
            Err(e) => {
                warn!("git clone error for {ws_clone}: {e}");
            }
        }
    });

    Ok(Json(json!({
        "status": "cloning",
        "workspace_id": workspace_id,
        "url": url
    })))
}

/// Returns the writable data base path as a String (mirrors storage/workspace.rs logic).
fn resolve_data_base() -> String {
    let candidates = ["/data", "/app/data"];
    for p in &candidates {
        let path = std::path::PathBuf::from(p);
        if path.exists() {
            let test = path.join(".clone_perm_test");
            if std::fs::write(&test, "").is_ok() {
                std::fs::remove_file(&test).ok();
                return p.to_string();
            }
        }
    }
    std::env::var("REQUIEM_STORAGE").unwrap_or_else(|_| "/data".to_string())
}
