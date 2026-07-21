//! # Agent Chat Handler — حلقة أدوات حقيقية
//!
//! `POST /api/agent/chat`
//!
//! Implements a multi-step agentic tool-use loop (like Claude Code):
//! 1. Build system prompt + workspace context
//! 2. Call LLM with workspace tools enabled
//! 3. If the response has tool_calls → execute them, stream results, loop
//! 4. Repeat up to MAX_AGENT_STEPS until the model produces a final text reply
//! 5. Emit SSE events for each step; close with `{"type":"done"}`

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
use tracing::{debug, warn, error};
use crate::routes::AuthUser;
use crate::agent::tools::workspace::{workspace_tools_schema, execute_workspace_tool};
use crate::storage::workspace as ws;

// ─── Constants ────────────────────────────────────────────────────────────────

const MAX_AGENT_STEPS: usize = 10;
const ZEN_TIMEOUT_SECS: u64 = 90;
const ZEN_ENDPOINT: &str = "https://opencode.ai/zen/v1/chat/completions";

// ─── Request / Response Types ─────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct AgentChatRequest {
    pub message: String,
    pub session_id: Option<String>,
    /// If set, workspace filesystem tools are enabled
    pub workspace_id: Option<String>,
    pub mode: Option<String>,
    pub effort: Option<String>,
    pub model: Option<String>,
}

/// SSE event variants streamed to the client.
#[derive(Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum AgentEvent {
    Thinking    { content: String },
    ToolUse     { tool: String, input: Value, tool_call_id: String },
    ToolResult  { tool_call_id: String, result: Value },
    Text        { content: String },
    Error       { message: String },
    Done        { usage: Value },
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

// ─── Workspace context builder ────────────────────────────────────────────────

async fn build_workspace_context(user_id: &str, workspace_id: &str) -> String {
    match ws::workspace_tree(user_id, workspace_id).await {
        Ok(tree_json) => {
            let mut files: Vec<String> = Vec::new();
            collect_paths(&tree_json, &mut files);
            if files.is_empty() {
                format!("Workspace `{workspace_id}` is empty.\n")
            } else {
                let list = files.iter().map(|f| format!("- {f}")).collect::<Vec<_>>().join("\n");
                format!("## Workspace: `{workspace_id}`\n\nFiles ({}):\n{list}\n", files.len())
            }
        }
        Err(_) => String::new(),
    }
}

fn collect_paths(node: &Value, out: &mut Vec<String>) {
    let arr = match node.as_array() {
        Some(a) => a,
        None => return,
    };
    for item in arr {
        let kind = item["type"].as_str().unwrap_or("file");
        let path = item["path"].as_str().unwrap_or("").to_string();
        if kind == "file" {
            out.push(path);
        } else if kind == "dir" {
            if let Some(children) = item.get("children") {
                collect_paths(children, out);
            }
        }
    }
}

// ─── Main handler ─────────────────────────────────────────────────────────────

pub async fn agent_chat_handler(
    State(_state): State<Arc<crate::AppState>>,
    Extension(auth_user): Extension<AuthUser>,
    Json(body): Json<AgentChatRequest>,
) -> Response {
    let user_id      = auth_user.user_id.clone();
    let workspace_id = body.workspace_id.clone();
    let model        = body.model.clone()
        .unwrap_or_else(|| "deepseek-v4-flash-free".to_string());
    let mode         = body.mode.as_deref().unwrap_or("coder").to_string();
    let _session_id  = body.session_id.as_deref().unwrap_or("default").to_string();
    let message      = body.message.clone();

    debug!("agent_chat: user={user_id} model={model} workspace={workspace_id:?}");

    // Use a futures mpsc channel (available via `futures` crate already in Cargo.toml)
    let (tx, rx) = futures::channel::mpsc::channel::<bytes::Bytes>(64);

    let user_id_c      = user_id.clone();
    let workspace_id_c = workspace_id.clone();
    let model_c        = model.clone();

    tokio::spawn(async move {
        run_agent_loop(
            &user_id_c,
            workspace_id_c.as_deref(),
            &model_c,
            &mode,
            &message,
            tx,
        ).await;
    });

    // futures::channel::mpsc::Receiver implements Stream directly
    let stream = rx.map(|b| Ok::<_, std::io::Error>(b));
    let body = Body::from_stream(stream);

    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "text/event-stream; charset=utf-8")
        .header(header::CACHE_CONTROL, "no-cache, no-store")
        .header("X-Accel-Buffering", "no")
        .body(body)
        .unwrap_or_else(|e| {
            error!("agent_chat: failed to build response: {e}");
            (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "stream build failed"}))).into_response()
        })
}

// ─── Agent loop ──────────────────────────────────────────────────────────────

async fn run_agent_loop(
    user_id: &str,
    workspace_id: Option<&str>,
    model: &str,
    mode: &str,
    user_message: &str,
    mut tx: futures::channel::mpsc::Sender<bytes::Bytes>,
) {
    /// Helper: send an SSE event, ignore send errors (client disconnect).
    macro_rules! emit {
        ($event:expr) => {
            let _ = tx.try_send(bytes::Bytes::from($event.to_sse_line()));
        };
    }

    // ── Build initial messages ──────────────────────────────────────────────
    let mut system_content = format!(
        "You are Requiem Agent 1 — an autonomous AI coding assistant.\n\
         Mode: {mode}\n\
         You have workspace filesystem tools available. Use them to read, write, and \
         edit files. Always use ws_read before editing. Think step by step.\n"
    );

    // Workspace context
    if let Some(wid) = workspace_id {
        let ctx = build_workspace_context(user_id, wid).await;
        if !ctx.is_empty() {
            system_content.push_str("\n\n");
            system_content.push_str(&ctx);
        }
    }

    let mut messages: Vec<Value> = vec![
        json!({"role": "system", "content": system_content}),
        json!({"role": "user",   "content": user_message}),
    ];

    // ── Tools list ──────────────────────────────────────────────────────────
    let tools: Vec<Value> = if workspace_id.is_some() {
        workspace_tools_schema()
    } else {
        vec![]
    };

    // ── Agentic loop ────────────────────────────────────────────────────────
    let mut steps = 0usize;

    loop {
        steps += 1;
        if steps > MAX_AGENT_STEPS {
            emit!(AgentEvent::Error {
                message: "Reached maximum agent steps limit".to_string()
            });
            break;
        }

        emit!(AgentEvent::Thinking {
            content: format!("Step {steps}/{MAX_AGENT_STEPS} — calling model…")
        });

        let tools_ref: Option<&[Value]> = if tools.is_empty() { None } else { Some(&tools) };

        let llm_response = match call_llm(user_id, model, &messages, tools_ref).await {
            Ok(r)  => r,
            Err(e) => {
                emit!(AgentEvent::Error { message: format!("LLM error: {e}") });
                break;
            }
        };

        // Extract the assistant message from the response
        let choice   = &llm_response["choices"][0];
        let message  = &choice["message"];

        // Push assistant turn into conversation
        messages.push(message.clone());

        // ── Check for tool_calls ────────────────────────────────────────────
        let tool_calls = message["tool_calls"].as_array().cloned().unwrap_or_default();

        if !tool_calls.is_empty() {
            // Execute each tool call
            let mut tool_results: Vec<Value> = Vec::new();

            for call in &tool_calls {
                let call_id = call["id"].as_str().unwrap_or("").to_string();
                let fn_name = call["function"]["name"].as_str().unwrap_or("").to_string();
                let args_raw = call["function"]["arguments"].as_str().unwrap_or("{}");
                let input: Value = serde_json::from_str(args_raw).unwrap_or(json!({}));

                emit!(AgentEvent::ToolUse {
                    tool: fn_name.clone(),
                    input: input.clone(),
                    tool_call_id: call_id.clone()
                });

                let result = if let Some(wid) = workspace_id {
                    execute_workspace_tool(&fn_name, &input, user_id, wid)
                        .await
                        .unwrap_or_else(|e| json!({"error": e}))
                } else {
                    json!({"error": "no workspace_id — tool unavailable"})
                };

                emit!(AgentEvent::ToolResult {
                    tool_call_id: call_id.clone(),
                    result: result.clone()
                });

                tool_results.push(json!({
                    "role":         "tool",
                    "tool_call_id": call_id,
                    "content":      serde_json::to_string(&result).unwrap_or_default()
                }));
            }

            // Append all tool results to messages for next round
            messages.extend(tool_results);
            continue; // loop again with tool results
        }

        // ── No tool calls — extract final text ─────────────────────────────
        let text = message["content"].as_str().unwrap_or("").to_string();
        if !text.is_empty() {
            emit!(AgentEvent::Text { content: text });
        }

        // Done
        let usage = llm_response.get("usage").cloned().unwrap_or(json!({}));
        emit!(AgentEvent::Done {
            usage: json!({ "steps": steps, "model_usage": usage })
        });
        break;
    }
}
