//! # Agent Chat Handler — حلقة أدوات حقيقية + RAG + Identity + Env Awareness
//!
//! `POST /api/agent/chat`
//!
//! Pipeline per request:
//! 0. Identity Shield — reject probes
//! 1. RAG retrieval — inject relevant memories into system prompt
//! 2. Build full environment block (workspace tree, tools, capabilities)
//! 3. Tool-use loop (up to MAX_AGENT_STEPS): call LLM → execute tools → loop
//! 4. Auto-store memories from the conversation
//! 5. Strict Locks enforcement on final output

use axum::{
    body::Body,
    extract::State,
    http::{StatusCode, header},
    response::{IntoResponse, Response},
    Extension, Json,
};
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, warn, error, info};
use crate::routes::AuthUser;
use crate::agent::tools::workspace::{workspace_tools_schema, execute_workspace_tool};
use crate::storage::workspace as ws;
use crate::agent::memory::rag::RagEngine;
use crate::agent::identity_shield::IdentityShieldV3;
use crate::enforce::StrictLocksEngine;

// ─── Constants ────────────────────────────────────────────────────────────────

const MAX_AGENT_STEPS: usize = 10;
const ZEN_TIMEOUT_SECS: u64 = 90;
const ZEN_ENDPOINT: &str = "https://opencode.ai/zen/v1/chat/completions";

// ─── Request / Response Types ─────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct AgentChatRequest {
    pub message:      String,
    pub session_id:   Option<String>,
    /// If set, workspace filesystem tools are enabled
    pub workspace_id: Option<String>,
    pub mode:         Option<String>,
    pub effort:       Option<String>,
    pub model:        Option<String>,
    /// Last N messages from history (for context continuity)
    pub history:      Option<Vec<Value>>,
}

/// SSE event variants streamed to the client.
#[derive(Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum AgentEvent {
    Thinking   { content: String },
    ToolUse    { tool: String, input: Value, tool_call_id: String },
    ToolResult { tool_call_id: String, result: Value },
    MemoryHit  { count: usize, preview: String },
    Text       { content: String },
    Error      { message: String },
    Done       { usage: Value },
}

impl AgentEvent {
    fn to_sse_line(&self) -> String {
        let json = serde_json::to_string(self).unwrap_or_else(|_| "{}".to_string());
        format!("data: {json}\n\n")
    }
}

// ─── Proxy helpers (mirrors zen.rs) ──────────────────────────────────────────

const PROXIES: &[(&str, u16, &str, &str)] = &[
    ("31.59.20.176",6754,"cchsbntj","8ocnhyz7f53b"),
    ("31.56.127.193",7684,"cchsbntj","8ocnhyz7f53b"),
    ("45.38.107.97",6014,"cchsbntj","8ocnhyz7f53b"),
    ("198.105.121.200",6462,"cchsbntj","8ocnhyz7f53b"),
    ("64.137.96.74",6641,"cchsbntj","8ocnhyz7f53b"),
    ("198.23.243.226",6361,"cchsbntj","8ocnhyz7f53b"),
    ("38.154.185.97",6370,"cchsbntj","8ocnhyz7f53b"),
    ("84.247.60.125",6095,"cchsbntj","8ocnhyz7f53b"),
    ("142.111.67.146",5611,"cchsbntj","8ocnhyz7f53b"),
    ("191.96.254.138",6185,"cchsbntj","8ocnhyz7f53b"),
];

fn proxy_index_for_user(user_id: &str) -> usize {
    let hash: u64 = user_id.bytes().fold(0u64, |acc, b| {
        acc.wrapping_mul(31).wrapping_add(b as u64)
    });
    (hash as usize) % PROXIES.len()
}

// ─── LLM call (non-streaming, returns full JSON) ───────────────────────────

async fn call_llm(
    user_id: &str,
    model: &str,
    messages: &[Value],
    tools: Option<&[Value]>,
) -> Result<Value, String> {
    let mut body = json!({
        "model": model,
        "messages": messages,
        "stream": false,
    });

    if let Some(t) = tools {
        if !t.is_empty() {
            body["tools"] = json!(t);
            body["tool_choice"] = json!("auto");
        }
    }

    // Try primary proxy; on failure try next two proxies
    let primary = proxy_index_for_user(user_id);
    let mut last_err = String::new();

    for attempt in 0u32..3 {
        let idx = (primary + attempt as usize) % PROXIES.len();
        let (host, port, u, p) = PROXIES[idx];
        let proxy_url = format!("socks5://{u}:{p}@{host}:{port}");
        let cli = reqwest::Client::builder()
            .timeout(Duration::from_secs(ZEN_TIMEOUT_SECS))
            .proxy(reqwest::Proxy::all(&proxy_url).map_err(|e| e.to_string())?)
            .build()
            .map_err(|e| e.to_string())?;

        match cli.post(ZEN_ENDPOINT)
            .header("Content-Type", "application/json")
            .header("Authorization", "Bearer public")
            .json(&body)
            .send()
            .await
        {
            Ok(resp) if resp.status().is_success() => {
                let text = resp.text().await.map_err(|e| e.to_string())?;
                return serde_json::from_str::<Value>(&text)
                    .map_err(|e| format!("JSON parse: {e} — body: {}", &text[..text.len().min(300)]));
            }
            Ok(resp) => {
                let status = resp.status();
                let text = resp.text().await.unwrap_or_default();
                last_err = format!("HTTP {status}: {}", &text[..text.len().min(200)]);
                if status.as_u16() == 429 {
                    tokio::time::sleep(Duration::from_secs(3)).await;
                }
            }
            Err(e) => {
                last_err = e.to_string();
                warn!("Agent LLM attempt {attempt} failed: {e}");
                tokio::time::sleep(Duration::from_millis(500 * (attempt + 1) as u64)).await;
            }
        }
    }
    Err(format!("LLM call failed after 3 attempts: {last_err}"))
}

// ─── Workspace context builder (rich) ────────────────────────────────────────

async fn build_workspace_context(user_id: &str, workspace_id: &str) -> String {
    let Ok(tree_json) = ws::workspace_tree(user_id, workspace_id).await else {
        return String::new();
    };
    let mut files: Vec<String> = Vec::new();
    collect_paths(&tree_json, &mut files);
    if files.is_empty() {
        return format!("Active workspace `{workspace_id}` is empty — use ws_write to create files.\n");
    }
    let list = files.iter().map(|f| format!("  {f}")).collect::<Vec<_>>().join("\n");
    format!(
        "## Active Workspace: `{workspace_id}`\nFile count: {}\nTree:\n{list}\n\
         \nUse ws_read before editing any file. Use ws_write to create new files. \
         Use ws_glob/ws_grep to search. Use ws_bash for shell commands.\n",
        files.len()
    )
}

fn collect_paths(node: &Value, out: &mut Vec<String>) {
    let arr = match node.as_array() { Some(a) => a, None => return };
    for item in arr {
        match item["type"].as_str().unwrap_or("file") {
            "file" => out.push(item["path"].as_str().unwrap_or("").to_string()),
            "dir"  => if let Some(c) = item.get("children") { collect_paths(c, out); },
            _ => {}
        }
    }
}

// ─── Full system prompt builder ───────────────────────────────────────────────

fn build_system_prompt(
    user_id:      &str,
    mode:         &str,
    workspace_ctx: &str,
    rag_context:  &str,
) -> String {
    // Identity Shield — enforce brand identity
    let identity = {
        let shield = IdentityShieldV3::new("internal-model");
        shield.generate_system_prompt()
    };
    // Strict Locks — prevent leakage / jailbreak
    let locks = {
        let engine = StrictLocksEngine::new();
        engine.generate_lock_context("autonomous", mode)
    };

    let mode_desc = match mode {
        "coder"       => "Write clean, idiomatic code. Always read files before editing. Prefer surgical ws_edit over full rewrites.",
        "debugger"    => "Diagnose root cause systematically. Use ws_grep and ws_read to trace the issue. Show exact diff.",
        "planner"     => "Plan before acting. Produce structured task breakdown. Ask clarifying questions if requirements are ambiguous.",
        "researcher"  => "Synthesize information deeply. Cite evidence from the codebase. Use ws_grep to find patterns.",
        "designer"    => "Focus on UX and visual clarity. When writing frontend code, follow existing component patterns.",
        "explorer"    => "Map the codebase. Use ws_tree and ws_glob to understand structure. Summarize architecture.",
        "security"    => "Audit for vulnerabilities. Use ws_grep for patterns like SQL injection, unvalidated input, secret exposure.",
        "orchestrator"=> "Coordinate multiple tools and sub-tasks. Break complex requests into atomic steps. Verify each step.",
        _             => "Be precise, thorough, and proactive.",
    };

    let tools_list = if !workspace_ctx.is_empty() {
        "\n\n## Tools Available\n\
         - ws_read(path, offset?, limit?) — read file content\n\
         - ws_write(path, content) — create or overwrite file\n\
         - ws_edit(path, old_string, new_string) — surgical replacement\n\
         - ws_delete(path) — delete file or dir\n\
         - ws_tree(path?, depth?) — directory tree\n\
         - ws_glob(pattern, path?) — find files by pattern\n\
         - ws_grep(pattern, path?, include?) — search file contents\n\
         - ws_mkdir(path) — create directory\n\
         \nAlways ws_read before ws_edit. Chain tools to verify changes."
    } else {
        ""
    };

    let rag_section = if !rag_context.is_empty() {
        format!("\n\n## Relevant Memory from Past Sessions\n{rag_context}")
    } else {
        String::new()
    };

    format!(
        "{identity}\n\n{locks}\n\n\
         ## Current Mode: {mode}\n{mode_desc}\
         {tools_list}\
         \n\n## Storage\n\
         User files: /data/users/{user_id}/\n\
         Workspaces: /data/users/{user_id}/workspaces/\n\
         Each workspace is fully isolated — you cannot access other users' files.\
         {workspace_section}\
         {rag_section}",
        workspace_section = if !workspace_ctx.is_empty() {
            format!("\n\n{workspace_ctx}")
        } else {
            String::new()
        }
    )
}

// ─── Main handler ─────────────────────────────────────────────────────────────

pub async fn agent_chat_handler(
    State(state): State<Arc<crate::AppState>>,
    Extension(auth_user): Extension<AuthUser>,
    Json(body): Json<AgentChatRequest>,
) -> Response {
    let user_id      = auth_user.user_id.clone();
    let workspace_id = body.workspace_id.clone();
    let session_id   = body.session_id.clone().unwrap_or_else(|| "default".to_string());
    let message      = body.message.clone();
    let mode         = body.mode.clone().unwrap_or_else(|| "coder".to_string());
    let effort       = body.effort.clone().unwrap_or_else(|| "medium".to_string());
    let history      = body.history.clone().unwrap_or_default();

    // Model selection: respect user choice, else pick by mode/effort
    let model = body.model.clone().unwrap_or_else(|| {
        match (mode.as_str(), effort.as_str()) {
            ("debugger",  _)          => "nemotron-3-ultra-free",
            ("planner",   _)          => "hy3-free",
            ("researcher",_)          => "hy3-free",
            ("reviewer",  _)          => "north-mini-code-free",
            ("orchestrator", "max")   => "mimo-v2.5-free",
            (_, "high") | (_, "max")  => "big-pickle",
            _                         => "deepseek-v4-flash-free",
        }.to_string()
    });

    info!("agent_chat: user={user_id} model={model} mode={mode} ws={workspace_id:?}");

    // ── 0. Identity Shield — reject probes before spawning ─────────────────
    {
        let mut shield = IdentityShieldV3::new("internal-model");
        let check = shield.check(&message);
        if check.is_probe && !check.responses.is_empty() {
            let text = check.responses.join("\n\n");
            let sse  = format!(
                "data: {{\"type\":\"text\",\"content\":{}}}\n\ndata: {{\"type\":\"done\",\"usage\":{{}}}}\n\n",
                serde_json::to_string(&text).unwrap_or_default()
            );
            return (
                StatusCode::OK,
                [("Content-Type", "text/event-stream; charset=utf-8"),
                 ("Cache-Control", "no-cache, no-store"),
                 ("X-Accel-Buffering", "no")],
                sse,
            ).into_response();
        }
    }

    let (tx, rx) = futures::channel::mpsc::channel::<bytes::Bytes>(64);
    let conn_clone = state.conn.clone();

    tokio::spawn(async move {
        run_agent_loop(
            &user_id, workspace_id.as_deref(),
            &model, &mode, &message,
            &session_id, &history, conn_clone, tx,
        ).await;
    });

    let stream = rx.map(|b| Ok::<_, std::io::Error>(b));
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "text/event-stream; charset=utf-8")
        .header(header::CACHE_CONTROL, "no-cache, no-store")
        .header("X-Accel-Buffering", "no")
        .body(Body::from_stream(stream))
        .unwrap_or_else(|e| {
            error!("agent_chat response build failed: {e}");
            (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error":"stream build failed"}))).into_response()
        })
}

// ─── Agent loop — full pipeline ───────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
async fn run_agent_loop(
    user_id:      &str,
    workspace_id: Option<&str>,
    model:        &str,
    mode:         &str,
    user_message: &str,
    session_id:   &str,
    history:      &[Value],
    conn:         Arc<libsql::Connection>,
    mut tx:       futures::channel::mpsc::Sender<bytes::Bytes>,
) {
    macro_rules! emit {
        ($event:expr) => { let _ = tx.try_send(bytes::Bytes::from($event.to_sse_line())); };
    }

    // ── 1. RAG retrieval — inject relevant memories ────────────────────────
    let rag_context = {
        let rag = RagEngine::new(conn.clone(), user_id);
        match rag.build_context(user_message, Some(session_id), 1200).await {
            Ok(r) if r.memories_used > 0 => {
                emit!(AgentEvent::MemoryHit {
                    count:   r.memories_used,
                    preview: r.system_context.chars().take(80).collect::<String>(),
                });
                r.system_context
            }
            _ => String::new(),
        }
    };

    // ── 2. Workspace context ────────────────────────────────────────────────
    let workspace_ctx = if let Some(wid) = workspace_id {
        build_workspace_context(user_id, wid).await
    } else {
        String::new()
    };

    // ── 3. Build full system prompt ─────────────────────────────────────────
    let system_content = build_system_prompt(user_id, mode, &workspace_ctx, &rag_context);

    // ── 4. Assemble message list with history ───────────────────────────────
    let mut messages: Vec<Value> = vec![json!({"role":"system","content": system_content})];
    // Inject last N history turns for context continuity
    for h in history.iter().take(12) {
        messages.push(h.clone());
    }
    messages.push(json!({"role":"user","content": user_message}));

    // ── 5. Tools ────────────────────────────────────────────────────────────
    let tools: Vec<Value> = if workspace_id.is_some() { workspace_tools_schema() } else { vec![] };

    // ── 6. Agentic loop ─────────────────────────────────────────────────────
    let mut steps      = 0usize;
    let mut final_text = String::new();

    loop {
        steps += 1;
        if steps > MAX_AGENT_STEPS {
            emit!(AgentEvent::Error { message: "Max agent steps reached".to_string() });
            break;
        }

        emit!(AgentEvent::Thinking {
            content: format!("Step {steps} — {model} reasoning…")
        });

        let tools_ref = if tools.is_empty() { None } else { Some(tools.as_slice()) };
        let llm_resp  = match call_llm(user_id, model, &messages, tools_ref).await {
            Ok(r)  => r,
            Err(e) => { emit!(AgentEvent::Error { message: format!("LLM: {e}") }); break; }
        };

        let choice  = &llm_resp["choices"][0];
        let msg_val = &choice["message"];
        messages.push(msg_val.clone());

        let tool_calls = msg_val["tool_calls"].as_array().cloned().unwrap_or_default();

        if !tool_calls.is_empty() {
            let mut results: Vec<Value> = Vec::new();
            for call in &tool_calls {
                let call_id = call["id"].as_str().unwrap_or("").to_string();
                let fn_name = call["function"]["name"].as_str().unwrap_or("").to_string();
                let args: Value = serde_json::from_str(
                    call["function"]["arguments"].as_str().unwrap_or("{}")
                ).unwrap_or(json!({}));

                emit!(AgentEvent::ToolUse {
                    tool: fn_name.clone(), input: args.clone(), tool_call_id: call_id.clone()
                });

                let result = if let Some(wid) = workspace_id {
                    execute_workspace_tool(&fn_name, &args, user_id, wid)
                        .await.unwrap_or_else(|e| json!({"error": e}))
                } else {
                    json!({"error": "no workspace — tool not available"})
                };

                emit!(AgentEvent::ToolResult { tool_call_id: call_id.clone(), result: result.clone() });

                results.push(json!({
                    "role": "tool", "tool_call_id": call_id,
                    "content": serde_json::to_string(&result).unwrap_or_default()
                }));
            }
            messages.extend(results);
            continue;
        }

        // ── Final text response ─────────────────────────────────────────────
        let text = msg_val["content"].as_str().unwrap_or("").to_string();

        // Strict Locks enforcement — prevent identity leakage
        let locks_engine = StrictLocksEngine::new();
        let lock_check   = locks_engine.check_all(mode, model, "text", &text);
        let final_out = if !lock_check.passed {
            let has_critical = lock_check.violations.iter()
                .any(|v| v.severity == crate::enforce::locks::ViolationSeverity::Critical);
            if has_critical {
                warn!("Strict lock violation in agent output for {user_id}");
                "I am **Requiem Agent 1** — I cannot reveal internal model details.".to_string()
            } else { text.clone() }
        } else { text.clone() };

        final_text = final_out.clone();
        if !final_out.is_empty() {
            emit!(AgentEvent::Text { content: final_out });
        }

        let usage = llm_resp.get("usage").cloned().unwrap_or(json!({}));
        emit!(AgentEvent::Done { usage: json!({ "steps": steps, "model_usage": usage }) });
        break;
    }

    // ── 7. Auto-store memories (fire & forget) ─────────────────────────────
    if !final_text.is_empty() {
        let rag = RagEngine::new(conn, user_id);
        let _ = rag.auto_store(user_message, &final_text, session_id).await;
    }
}
