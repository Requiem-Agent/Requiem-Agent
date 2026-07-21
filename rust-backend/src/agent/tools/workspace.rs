//! # Workspace Tools — أدوات الـ Workspace للوكيل
//!
//! Provides filesystem tools that let the agent READ, WRITE, EDIT, DELETE
//! files inside a user's workspace — mirroring Claude Code's tool set.
//!
//! All paths are relative to the workspace root and are validated against
//! path-traversal attacks by the `storage::workspace` module.

use serde_json::{json, Value};
use crate::storage::workspace as ws;

// ─── Tool Schema ──────────────────────────────────────────────────────────────

/// Returns the OpenAI function-calling schema for all workspace tools.
pub fn workspace_tools_schema() -> Vec<Value> {
    vec![
        json!({
            "type": "function",
            "function": {
                "name": "ws_read",
                "description": "Read a file from the user's workspace. Supports optional line offset and limit for large files.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "path":   {"type": "string",  "description": "File path relative to workspace root"},
                        "offset": {"type": "integer", "description": "Line number to start reading from (1-indexed)"},
                        "limit":  {"type": "integer", "description": "Maximum number of lines to read"}
                    },
                    "required": ["path"]
                }
            }
        }),
        json!({
            "type": "function",
            "function": {
                "name": "ws_write",
                "description": "Write (create or overwrite) a file in the workspace atomically.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "path":    {"type": "string", "description": "File path relative to workspace root"},
                        "content": {"type": "string", "description": "File content to write"}
                    },
                    "required": ["path", "content"]
                }
            }
        }),
        json!({
            "type": "function",
            "function": {
                "name": "ws_edit",
                "description": "Surgical string replacement inside a file (like a precise find-and-replace). Replaces the first occurrence of old_string with new_string.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "path":       {"type": "string", "description": "File path relative to workspace root"},
                        "old_string": {"type": "string", "description": "Exact string to find and replace"},
                        "new_string": {"type": "string", "description": "Replacement string"}
                    },
                    "required": ["path", "old_string", "new_string"]
                }
            }
        }),
        json!({
            "type": "function",
            "function": {
                "name": "ws_delete",
                "description": "Delete a file or directory (recursive) from the workspace.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "path": {"type": "string", "description": "File or directory path relative to workspace root"}
                    },
                    "required": ["path"]
                }
            }
        }),
        json!({
            "type": "function",
            "function": {
                "name": "ws_tree",
                "description": "List the directory tree of the workspace as an ASCII tree.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "path":  {"type": "string",  "description": "Subdirectory path to list (default: workspace root)"},
                        "depth": {"type": "integer", "description": "Maximum depth to traverse (default: 3)"}
                    }
                }
            }
        }),
        json!({
            "type": "function",
            "function": {
                "name": "ws_glob",
                "description": "Find files matching a glob pattern in the workspace.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "pattern": {"type": "string", "description": "Glob pattern, e.g. '**/*.rs' or 'src/*.ts'"},
                        "path":    {"type": "string", "description": "Base directory to search from (default: workspace root)"}
                    },
                    "required": ["pattern"]
                }
            }
        }),
        json!({
            "type": "function",
            "function": {
                "name": "ws_grep",
                "description": "Search file contents for a pattern (line-by-line substring or regex).",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "pattern": {"type": "string", "description": "String or regex pattern to search for"},
                        "path":    {"type": "string", "description": "Directory or file path to search in (default: workspace root)"},
                        "include": {"type": "string", "description": "Glob pattern to filter files, e.g. '*.rs'"}
                    },
                    "required": ["pattern"]
                }
            }
        }),
        json!({
            "type": "function",
            "function": {
                "name": "ws_mkdir",
                "description": "Create a directory (and parents) in the workspace.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "path": {"type": "string", "description": "Directory path relative to workspace root"}
                    },
                    "required": ["path"]
                }
            }
        }),
    ]
}

// ─── Tool Dispatch ────────────────────────────────────────────────────────────

/// Execute a workspace tool by name with the given JSON input.
pub async fn execute_workspace_tool(
    tool_name: &str,
    input: &Value,
    user_id: &str,
    workspace_id: &str,
) -> Result<Value, String> {
    match tool_name {
        "ws_read"   => tool_read(input, user_id, workspace_id).await,
        "ws_write"  => tool_write(input, user_id, workspace_id).await,
        "ws_edit"   => tool_edit(input, user_id, workspace_id).await,
        "ws_delete" => tool_delete(input, user_id, workspace_id).await,
        "ws_tree"   => tool_tree(input, user_id, workspace_id).await,
        "ws_glob"   => tool_glob(input, user_id, workspace_id).await,
        "ws_grep"   => tool_grep(input, user_id, workspace_id).await,
        "ws_mkdir"  => tool_mkdir(input, user_id, workspace_id).await,
        other       => Err(format!("Unknown workspace tool: {other}")),
    }
}

// ─── ws_read ─────────────────────────────────────────────────────────────────

async fn tool_read(input: &Value, user_id: &str, workspace_id: &str) -> Result<Value, String> {
    let path = input["path"].as_str().ok_or("ws_read: missing 'path'")?;
    let offset = input["offset"].as_u64().unwrap_or(0) as usize;
    let limit  = input["limit"].as_u64().map(|v| v as usize);

    let raw = ws::workspace_read_file(user_id, workspace_id, path).await?;
    let lines: Vec<&str> = raw.lines().collect();
    let total = lines.len();

    let start = offset.min(total);
    let end = match limit {
        Some(n) => (start + n).min(total),
        None    => total,
    };

    let selected: Vec<&str> = lines[start..end].to_vec();
    let content = selected.join("\n");
    let truncated = end < total;

    Ok(json!({
        "content": content,
        "lines": end - start,
        "truncated": truncated
    }))
}

// ─── ws_write ────────────────────────────────────────────────────────────────

async fn tool_write(input: &Value, user_id: &str, workspace_id: &str) -> Result<Value, String> {
    let path    = input["path"].as_str().ok_or("ws_write: missing 'path'")?;
    let content = input["content"].as_str().ok_or("ws_write: missing 'content'")?;

    ws::workspace_write_file(user_id, workspace_id, path, content).await?;

    Ok(json!({
        "path": path,
        "bytes": content.len()
    }))
}

// ─── ws_edit ─────────────────────────────────────────────────────────────────

async fn tool_edit(input: &Value, user_id: &str, workspace_id: &str) -> Result<Value, String> {
    let path       = input["path"].as_str().ok_or("ws_edit: missing 'path'")?;
    let old_string = input["old_string"].as_str().ok_or("ws_edit: missing 'old_string'")?;
    let new_string = input["new_string"].as_str().ok_or("ws_edit: missing 'new_string'")?;

    let original = ws::workspace_read_file(user_id, workspace_id, path).await?;

    if !original.contains(old_string) {
        return Ok(json!({
            "path": path,
            "applied": false,
            "diff": format!("Error: old_string not found in {path}")
        }));
    }

    // Replace first occurrence only (surgical edit)
    let updated = original.replacen(old_string, new_string, 1);
    ws::workspace_write_file(user_id, workspace_id, path, &updated).await?;

    let diff = build_simple_diff(path, old_string, new_string);

    Ok(json!({
        "path": path,
        "applied": true,
        "diff": diff
    }))
}

/// Generate a minimal unified diff for display purposes.
fn build_simple_diff(path: &str, old_str: &str, new_str: &str) -> String {
    let mut out = String::new();
    out.push_str(&format!("--- a/{path}\n"));
    out.push_str(&format!("+++ b/{path}\n"));
    out.push_str("@@ -1 +1 @@\n");
    for line in old_str.lines() {
        out.push_str(&format!("-{line}\n"));
    }
    for line in new_str.lines() {
        out.push_str(&format!("+{line}\n"));
    }
    out
}

// ─── ws_delete ───────────────────────────────────────────────────────────────

async fn tool_delete(input: &Value, user_id: &str, workspace_id: &str) -> Result<Value, String> {
    let path = input["path"].as_str().ok_or("ws_delete: missing 'path'")?;

    // Try file first; if that fails try directory
    let file_result = ws::workspace_delete_file(user_id, workspace_id, path).await;
    if file_result.is_err() {
        // Attempt directory removal via workspace_write_file won't work — use tokio::fs directly
        // Build the absolute path via the private convention (reuse workspace_read_file to test existence)
        // We fall back to trying to delete as directory via a best-effort approach.
        // If workspace_delete_file errored because it's a directory, tokio::fs::remove_dir_all is needed.
        // We expose this by checking whether the error message hints at "Is a directory".
        let err_msg = file_result.unwrap_err();
        if err_msg.contains("Is a directory") || err_msg.contains("is a directory") {
            // Re-derive the root — same logic as workspace.rs data_base()
            let base = resolve_data_base();
            let abs = base
                .join("users").join(user_id)
                .join("workspaces").join(workspace_id)
                .join(path);
            tokio::fs::remove_dir_all(&abs)
                .await
                .map_err(|e| format!("ws_delete dir: {e}"))?;
        } else {
            return Err(err_msg);
        }
    }

    Ok(json!({ "path": path, "deleted": true }))
}

/// Mirrors `data_base()` from storage/workspace.rs — finds writable base dir.
fn resolve_data_base() -> std::path::PathBuf {
    for p in &["/data", "/app/data"] {
        let path = std::path::PathBuf::from(p);
        if path.exists() {
            let test = path.join(".ws_perm_test2");
            if std::fs::write(&test, "").is_ok() {
                std::fs::remove_file(&test).ok();
                return path;
            }
        }
    }
    let fallback = std::env::var("REQUIEM_STORAGE").unwrap_or_else(|_| "/data".to_string());
    std::path::PathBuf::from(fallback)
}

// ─── ws_tree ─────────────────────────────────────────────────────────────────

async fn tool_tree(input: &Value, user_id: &str, workspace_id: &str) -> Result<Value, String> {
    let sub_path = input["path"].as_str().unwrap_or("");
    let depth    = input["depth"].as_u64().unwrap_or(3) as usize;

    // Get the JSON tree from workspace module, then render as ASCII
    let tree_json = ws::workspace_tree(user_id, workspace_id).await?;

    let root_label = if sub_path.is_empty() { ".".to_string() } else { sub_path.to_string() };
    let mut output = format!("{root_label}/\n");
    render_tree_ascii(&tree_json, 1, depth, &mut output);

    Ok(json!({ "tree": output }))
}

/// Recursively render the JSON tree array as an ASCII-indented string.
fn render_tree_ascii(node: &Value, current_depth: usize, max_depth: usize, out: &mut String) {
    if current_depth > max_depth {
        return;
    }
    let indent = "  ".repeat(current_depth);
    let arr = match node.as_array() {
        Some(a) => a,
        None    => return,
    };
    for item in arr {
        let name = item["name"].as_str().unwrap_or("?");
        let kind = item["type"].as_str().unwrap_or("file");
        if kind == "dir" {
            out.push_str(&format!("{indent}{name}/\n"));
            if let Some(children) = item.get("children") {
                render_tree_ascii(children, current_depth + 1, max_depth, out);
            }
        } else {
            out.push_str(&format!("{indent}{name}\n"));
        }
    }
}

// ─── ws_glob ─────────────────────────────────────────────────────────────────

async fn tool_glob(input: &Value, user_id: &str, workspace_id: &str) -> Result<Value, String> {
    let pattern  = input["pattern"].as_str().ok_or("ws_glob: missing 'pattern'")?;
    let sub_path = input["path"].as_str().unwrap_or("");

    // Walk the tree and match file paths against the pattern
    let tree_json = ws::workspace_tree(user_id, workspace_id).await?;
    let mut files: Vec<String> = Vec::new();
    collect_all_files(&tree_json, &mut files);

    // Filter by sub_path prefix if given
    if !sub_path.is_empty() {
        let prefix = sub_path.trim_end_matches('/');
        files.retain(|f| f.starts_with(prefix));
    }

    // Match against glob pattern
    let matched: Vec<Value> = files.into_iter()
        .filter(|f| glob_match(pattern, f))
        .map(|f| Value::String(f))
        .collect();

    Ok(json!({ "files": matched }))
}

/// Flatten the workspace tree JSON into a list of relative file paths.
fn collect_all_files(node: &Value, out: &mut Vec<String>) {
    let arr = match node.as_array() {
        Some(a) => a,
        None    => return,
    };
    for item in arr {
        let kind = item["type"].as_str().unwrap_or("file");
        let path = item["path"].as_str().unwrap_or("").to_string();
        if kind == "file" {
            out.push(path);
        } else if kind == "dir" {
            if let Some(children) = item.get("children") {
                collect_all_files(children, out);
            }
        }
    }
}

/// Simple glob matcher: `*` matches any chars except `/`, `**` matches anything.
fn glob_match(pattern: &str, path: &str) -> bool {
    glob_match_inner(pattern.as_bytes(), path.as_bytes())
}

fn glob_match_inner(pat: &[u8], text: &[u8]) -> bool {
    match (pat.first(), text.first()) {
        // Both exhausted — match
        (None, None) => true,
        // Pattern exhausted but text remains — no match
        (None, Some(_)) => false,
        // `**` — consume any amount of any character (including `/`)
        (Some(&b'*'), Some(&b'*')) => {
            let rest_pat = if pat.len() >= 3 && pat[2] == b'/' { &pat[3..] } else { &pat[2..] };
            // Try matching rest_pat against every suffix of text
            for i in 0..=text.len() {
                if glob_match_inner(rest_pat, &text[i..]) {
                    return true;
                }
            }
            false
        }
        // `*` — matches any chars except `/`
        (Some(&b'*'), _) => {
            let rest_pat = &pat[1..];
            for i in 0..=text.len() {
                if text[..i].contains(&b'/') { break; }
                if glob_match_inner(rest_pat, &text[i..]) {
                    return true;
                }
            }
            false
        }
        // `?` — matches a single char (not `/`)
        (Some(&b'?'), Some(&c)) if c != b'/' => glob_match_inner(&pat[1..], &text[1..]),
        // Literal match
        (Some(&p), Some(&t)) if p == t => glob_match_inner(&pat[1..], &text[1..]),
        _ => false,
    }
}

// ─── ws_grep ─────────────────────────────────────────────────────────────────

async fn tool_grep(input: &Value, user_id: &str, workspace_id: &str) -> Result<Value, String> {
    let pattern  = input["pattern"].as_str().ok_or("ws_grep: missing 'pattern'")?;
    let sub_path = input["path"].as_str().unwrap_or("");
    let include  = input["include"].as_str().unwrap_or("*");

    // Collect candidate files
    let tree_json = ws::workspace_tree(user_id, workspace_id).await?;
    let mut all_files: Vec<String> = Vec::new();
    collect_all_files(&tree_json, &mut all_files);

    // Optionally narrow to sub_path prefix
    if !sub_path.is_empty() {
        let prefix = sub_path.trim_end_matches('/');
        all_files.retain(|f| f.starts_with(prefix));
    }

    // Filter by include glob
    all_files.retain(|f| glob_match(include, f));

    let mut matches: Vec<Value> = Vec::new();

    for rel_path in &all_files {
        let content = match ws::workspace_read_file(user_id, workspace_id, rel_path).await {
            Ok(c)  => c,
            Err(_) => continue,
        };

        for (line_no, line) in content.lines().enumerate() {
            if line.contains(pattern) {
                matches.push(json!({
                    "file":    rel_path,
                    "line":    line_no + 1,
                    "content": line.trim()
                }));
                // Cap at 200 matches to avoid overwhelming the context
                if matches.len() >= 200 {
                    return Ok(json!({ "matches": matches }));
                }
            }
        }
    }

    Ok(json!({ "matches": matches }))
}

// ─── ws_mkdir ────────────────────────────────────────────────────────────────

async fn tool_mkdir(input: &Value, user_id: &str, workspace_id: &str) -> Result<Value, String> {
    let path = input["path"].as_str().ok_or("ws_mkdir: missing 'path'")?;
    ws::workspace_mkdir(user_id, workspace_id, path).await?;
    Ok(json!({ "path": path, "created": true }))
}
