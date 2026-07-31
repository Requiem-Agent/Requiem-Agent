//! # Workspace Storage Module
//!
//! Handles per-user workspace (project) persistence under:
//! `users/{user_id}/workspaces/{workspace_id}/`
//!
//! Metadata is stored as JSON at:
//! `users/{user_id}/workspaces/{workspace_id}/.requiem/meta.json`
//!
//! Uses the same hybrid strategy as storage.rs:
//! - Primary: /data (HF bucket mount, persistent)
//! - Fallback: /app/data (local dev)

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use tokio::fs;
use tracing::{debug, warn};

// ─── Base Path Resolution ─────────────────────────────────────────────────────

/// Returns the writable base data directory (same logic as StorageEngine::new).
fn data_base() -> PathBuf {
    let candidates = ["/data", "/app/data"];
    for p in &candidates {
        let path = PathBuf::from(p);
        if path.exists() {
            let test = path.join(".ws_perm_test");
            if std::fs::write(&test, "").is_ok() {
                std::fs::remove_file(&test).ok();
                return path;
            }
        }
    }
    let fallback = std::env::var("REQUIEM_STORAGE").unwrap_or_else(|_| "/data".to_string());
    PathBuf::from(fallback)
}

/// Absolute path to the workspace root directory.
fn workspace_root(user_id: &str, workspace_id: &str) -> PathBuf {
    data_base()
        .join("users")
        .join(user_id)
        .join("workspaces")
        .join(workspace_id)
}

/// Public accessor for ws_bash tool — returns root as String for Command::current_dir
pub fn workspace_root_path(user_id: &str, workspace_id: &str) -> PathBuf {
    workspace_root(user_id, workspace_id)
}

/// Absolute path to the workspace metadata file.
fn meta_path(user_id: &str, workspace_id: &str) -> PathBuf {
    workspace_root(user_id, workspace_id)
        .join(".requiem")
        .join("meta.json")
}

// ─── WorkspaceMeta ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceMeta {
    pub id: String,
    pub name: String,
    pub description: String,
    pub created_at: String,
    pub file_count: u64,
    pub size_bytes: u64,
}

// ─── Internal Helpers ─────────────────────────────────────────────────────────

/// Reject any path component that looks dangerous.
fn validate_rel_path(rel_path: &str) -> Result<(), String> {
    if rel_path.contains("..") {
        return Err("Path traversal not allowed".to_string());
    }
    // Guard against absolute paths sneaking in
    if rel_path.starts_with('/') {
        return Err("Absolute paths not allowed as relative workspace path".to_string());
    }
    Ok(())
}

/// Resolve a relative path safely inside the workspace root.
fn safe_join(root: &Path, rel: &str) -> Result<PathBuf, String> {
    validate_rel_path(rel)?;
    let candidate = root.join(rel);
    // Ensure the resolved path still starts with root
    // (handles symlinks that exist and encoded traversals)
    let resolved = if candidate.exists() {
        candidate
            .canonicalize()
            .map_err(|e| format!("canonicalize: {e}"))?
    } else {
        candidate
    };
    if !resolved.starts_with(root) {
        return Err(format!(
            "Path escape attempt: {} is outside workspace root",
            resolved.display()
        ));
    }
    Ok(resolved)
}

/// Read raw metadata JSON for a workspace. Does NOT compute file_count/size.
async fn read_raw_meta(user_id: &str, workspace_id: &str) -> Result<WorkspaceMeta, String> {
    let path = meta_path(user_id, workspace_id);
    let raw = fs::read_to_string(&path)
        .await
        .map_err(|e| format!("read meta: {e}"))?;
    serde_json::from_str::<WorkspaceMeta>(&raw).map_err(|e| format!("parse meta: {e}"))
}

/// Compute file_count and size_bytes for a workspace (synchronous walk).
fn compute_stats(root: &Path) -> (u64, u64) {
    fn walk(dir: &Path, count: &mut u64, bytes: &mut u64) {
        let Ok(entries) = std::fs::read_dir(dir) else { return };
        for entry in entries.flatten() {
            let p = entry.path();
            // Skip hidden .requiem metadata directory from counts
            if p.file_name().map(|n| n == ".requiem").unwrap_or(false) {
                continue;
            }
            if p.is_dir() {
                walk(&p, count, bytes);
            } else if p.is_file() {
                *count += 1;
                *bytes += std::fs::metadata(&p).map(|m| m.len()).unwrap_or(0);
            }
        }
    }
    let mut count = 0u64;
    let mut bytes = 0u64;
    walk(root, &mut count, &mut bytes);
    (count, bytes)
}

// ─── Public API ───────────────────────────────────────────────────────────────

/// Create a new workspace — writes the metadata file.
pub async fn workspace_create(
    user_id: &str,
    workspace_id: &str,
    name: &str,
    description: &str,
) -> Result<(), String> {
    let meta_dir = workspace_root(user_id, workspace_id).join(".requiem");
    fs::create_dir_all(&meta_dir)
        .await
        .map_err(|e| format!("create workspace dirs: {e}"))?;

    let meta = WorkspaceMeta {
        id: workspace_id.to_string(),
        name: name.to_string(),
        description: description.to_string(),
        created_at: chrono_now(),
        file_count: 0,
        size_bytes: 0,
    };
    let json = serde_json::to_string_pretty(&meta).map_err(|e| format!("serialize meta: {e}"))?;
    fs::write(meta_dir.join("meta.json"), json)
        .await
        .map_err(|e| format!("write meta: {e}"))?;
    debug!("workspace_create: {user_id}/{workspace_id} name={name}");
    Ok(())
}

/// List all workspaces for a user.
pub async fn workspace_list(user_id: &str) -> Result<Vec<WorkspaceMeta>, String> {
    let workspaces_dir = data_base().join("users").join(user_id).join("workspaces");
    if !workspaces_dir.exists() {
        return Ok(vec![]);
    }
    let mut entries = fs::read_dir(&workspaces_dir)
        .await
        .map_err(|e| format!("read workspaces dir: {e}"))?;
    let mut result = Vec::new();
    while let Ok(Some(entry)) = entries.next_entry().await {
        let ft = entry.file_type().await.unwrap_or_else(|_| {
            // Can't unwrap, just skip
            panic!("file_type failed")
        });
        if !ft.is_dir() {
            continue;
        }
        let ws_id = entry.file_name().to_string_lossy().to_string();
        // Skip dotfiles / temp dirs
        if ws_id.starts_with('.') {
            continue;
        }
        match read_raw_meta(user_id, &ws_id).await {
            Ok(mut meta) => {
                let root = workspace_root(user_id, &ws_id);
                let (fc, sb) = compute_stats(&root);
                meta.file_count = fc;
                meta.size_bytes = sb;
                result.push(meta);
            }
            Err(e) => {
                warn!("workspace_list: skip {ws_id} — meta error: {e}");
            }
        }
    }
    // Sort by created_at descending
    result.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    Ok(result)
}

/// Get metadata for a single workspace.
pub async fn workspace_get(user_id: &str, workspace_id: &str) -> Result<WorkspaceMeta, String> {
    let mut meta = read_raw_meta(user_id, workspace_id).await?;
    let root = workspace_root(user_id, workspace_id);
    let (fc, sb) = compute_stats(&root);
    meta.file_count = fc;
    meta.size_bytes = sb;
    Ok(meta)
}

/// Update workspace metadata (name / description).
pub async fn workspace_update(
    user_id: &str,
    workspace_id: &str,
    name: Option<&str>,
    description: Option<&str>,
) -> Result<WorkspaceMeta, String> {
    let mut meta = read_raw_meta(user_id, workspace_id).await?;
    if let Some(n) = name {
        meta.name = n.to_string();
    }
    if let Some(d) = description {
        meta.description = d.to_string();
    }
    let path = meta_path(user_id, workspace_id);
    let json = serde_json::to_string_pretty(&meta).map_err(|e| format!("serialize meta: {e}"))?;
    fs::write(&path, json)
        .await
        .map_err(|e| format!("write meta: {e}"))?;
    Ok(meta)
}

/// Delete a workspace and all its files.
pub async fn workspace_delete(user_id: &str, workspace_id: &str) -> Result<(), String> {
    let root = workspace_root(user_id, workspace_id);
    if root.exists() {
        fs::remove_dir_all(&root)
            .await
            .map_err(|e| format!("remove workspace: {e}"))?;
    }
    debug!("workspace_delete: {user_id}/{workspace_id}");
    Ok(())
}

/// Write (create or overwrite) a file inside a workspace.
pub async fn workspace_write_file(
    user_id: &str,
    workspace_id: &str,
    rel_path: &str,
    content: &str,
) -> Result<(), String> {
    let root = workspace_root(user_id, workspace_id);
    let target = safe_join(&root, rel_path)?;
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)
            .await
            .map_err(|e| format!("create dirs: {e}"))?;
    }
    fs::write(&target, content)
        .await
        .map_err(|e| format!("write file: {e}"))?;
    debug!("workspace_write_file: {user_id}/{workspace_id}/{rel_path}");
    Ok(())
}

/// Read a file from inside a workspace.
pub async fn workspace_read_file(
    user_id: &str,
    workspace_id: &str,
    rel_path: &str,
) -> Result<String, String> {
    let root = workspace_root(user_id, workspace_id);
    let target = safe_join(&root, rel_path)?;
    fs::read_to_string(&target)
        .await
        .map_err(|e| format!("read file: {e}"))
}

/// Delete a file from inside a workspace.
pub async fn workspace_delete_file(
    user_id: &str,
    workspace_id: &str,
    rel_path: &str,
) -> Result<(), String> {
    let root = workspace_root(user_id, workspace_id);
    let target = safe_join(&root, rel_path)?;
    fs::remove_file(&target)
        .await
        .map_err(|e| format!("delete file: {e}"))?;
    debug!("workspace_delete_file: {user_id}/{workspace_id}/{rel_path}");
    Ok(())
}

/// Create a directory inside a workspace.
pub async fn workspace_mkdir(
    user_id: &str,
    workspace_id: &str,
    rel_path: &str,
) -> Result<(), String> {
    let root = workspace_root(user_id, workspace_id);
    let target = safe_join(&root, rel_path)?;
    fs::create_dir_all(&target)
        .await
        .map_err(|e| format!("mkdir: {e}"))?;
    debug!("workspace_mkdir: {user_id}/{workspace_id}/{rel_path}");
    Ok(())
}

/// Recursively walk a workspace directory and produce a JSON tree.
///
/// Returns:
/// ```json
/// [
///   {"type":"dir","name":"src","path":"src","children":[
///     {"type":"file","name":"main.rs","path":"src/main.rs","size":1234}
///   ]},
///   {"type":"file","name":"Cargo.toml","path":"Cargo.toml","size":500}
/// ]
/// ```
pub async fn workspace_tree(user_id: &str, workspace_id: &str) -> Result<Value, String> {
    let root = workspace_root(user_id, workspace_id);
    if !root.exists() {
        return Err(format!("workspace not found: {workspace_id}"));
    }
    // Synchronous walk is fine for moderate-sized trees; avoids recursive async complexity.
    let tree = build_tree(&root, &root)?;
    Ok(tree)
}

/// Synchronously build the file tree as a `serde_json::Value` array.
fn build_tree(root: &Path, dir: &Path) -> Result<Value, String> {
    let mut children: Vec<Value> = Vec::new();

    let entries =
        std::fs::read_dir(dir).map_err(|e| format!("read dir {}: {e}", dir.display()))?;

    let mut sorted: Vec<std::fs::DirEntry> = entries
        .filter_map(|e| e.ok())
        .collect();

    // Directories first, then files; both sorted alphabetically.
    sorted.sort_by(|a, b| {
        let a_dir = a.path().is_dir();
        let b_dir = b.path().is_dir();
        match (a_dir, b_dir) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.file_name().cmp(&b.file_name()),
        }
    });

    for entry in sorted {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();

        // Skip hidden .requiem metadata directory from the tree
        if name == ".requiem" {
            continue;
        }

        // Build relative path from workspace root
        let rel = path
            .strip_prefix(root)
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| name.clone());

        if path.is_dir() {
            let dir_children = build_tree(root, &path)?;
            children.push(json!({
                "type": "dir",
                "name": name,
                "path": rel,
                "children": dir_children
            }));
        } else if path.is_file() {
            let size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
            children.push(json!({
                "type": "file",
                "name": name,
                "path": rel,
                "size": size
            }));
        }
    }

    Ok(Value::Array(children))
}

// ─── Time Helper ──────────────────────────────────────────────────────────────

/// Returns current UTC time as ISO-8601 string without pulling in `chrono`.
fn chrono_now() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    // Format as a basic ISO-8601 UTC timestamp
    let s = secs;
    let (y, mo, d, h, mi, sec) = unix_to_ymd(s);
    format!("{y:04}-{mo:02}-{d:02}T{h:02}:{mi:02}:{sec:02}Z")
}

fn unix_to_ymd(mut secs: u64) -> (u64, u64, u64, u64, u64, u64) {
    let sec = secs % 60; secs /= 60;
    let min = secs % 60; secs /= 60;
    let hour = secs % 24; secs /= 24;
    // Days since epoch (Jan 1 1970)
    let mut days = secs;
    let mut year = 1970u64;
    loop {
        let days_in_year = if is_leap(year) { 366 } else { 365 };
        if days < days_in_year { break; }
        days -= days_in_year;
        year += 1;
    }
    let leap = is_leap(year);
    let month_days: [u64; 12] = [31, if leap { 29 } else { 28 }, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    let mut month = 1u64;
    for md in &month_days {
        if days < *md { break; }
        days -= md;
        month += 1;
    }
    (year, month, days + 1, hour, min, sec)
}

fn is_leap(y: u64) -> bool {
    (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
}
