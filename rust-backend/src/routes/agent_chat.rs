//! # Agent Chat Handler — حلقة أدوات حقيقية + RAG + Identity + Env Awareness
//!
//! `POST /api/agent/chat`
//!
//! Pipeline per request:
//! 0. Identity Shield — reject probes
//! 0b. Request deduplication — reject duplicate (user_id, message) within 5s
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
use futures::{StreamExt, future::select_ok as _select_ok};
use futures::future::FutureExt;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tracing::{debug, warn, error, info};
use crate::routes::AuthUser;
use crate::agent::tools::workspace::{workspace_tools_schema, execute_workspace_tool};
use crate::storage::workspace as ws;
use crate::agent::memory::rag::RagEngine;
use crate::agent::identity_shield::IdentityShieldV3;
use crate::enforce::StrictLocksEngine;
// S3-01: ReAct Loop Engine integration
use crate::react_loop::{ReActEngine, ToolDefinition, ToolResult, default_requiem_tools};
// S3-04: Prometheus metrics
use crate::metrics::{record_agent_step, record_llm_call, record_agent_error};

// ─── Constants ────────────────────────────────────────────────────────────────

/// Per-model call timeout — 45 s max, then fall back to fastest model
const MODEL_CALL_TIMEOUT_SECS: u64 = 45;
const ZEN_TIMEOUT_SECS: u64 = 90;
const ZEN_ENDPOINT: &str = "https://opencode.ai/zen/v1/chat/completions";
/// Reject duplicate (user_id + message_hash) within this window
const DEDUP_WINDOW_SECS: u64 = 5;
/// Maximum size of the dedup ring-buffer (prevents unbounded growth)
const DEDUP_MAX_ENTRIES: usize = 512;
const FALLBACK_MODEL: &str = "deepseek-v4-flash-free";

// ─── Request Deduplication ────────────────────────────────────────────────────

/// Globally shared dedup map: hash → last-seen Instant
static DEDUP_MAP: std::sync::OnceLock<Mutex<HashMap<u64, Instant>>> = std::sync::OnceLock::new();

fn dedup_map() -> &'static Mutex<HashMap<u64, Instant>> {
    DEDUP_MAP.get_or_init(|| Mutex::new(HashMap::with_capacity(64)))
}

/// Hash (user_id + message) using a fast DJB2-style hash.
fn request_hash(user_id: &str, message: &str) -> u64 {
    let mut h: u64 = 5381;
    for b in user_id.bytes().chain(std::iter::once(b'|')).chain(message.bytes()) {
        h = h.wrapping_mul(33).wrapping_add(b as u64);
    }
    h
}

/// Returns `true` if this request is a duplicate (seen within DEDUP_WINDOW_SECS).
/// Inserts / refreshes the entry if not a duplicate.
fn is_duplicate_request(user_id: &str, message: &str) -> bool {
    let hash = request_hash(user_id, message);
    let now = Instant::now();
    let window = Duration::from_secs(DEDUP_WINDOW_SECS);

    let mut map = dedup_map().lock().unwrap_or_else(|e| e.into_inner());

    if let Some(&prev) = map.get(&hash) {
        if now.duration_since(prev) < window {
            return true; // duplicate
        }
    }

    // Evict oldest entries if we're at capacity
    if map.len() >= DEDUP_MAX_ENTRIES {
        map.retain(|_, ts| now.duration_since(*ts) < Duration::from_secs(60));
    }

    map.insert(hash, now);
    false
}

// ─── Request / Response Types ─────────────────────────────────────────────────

#[derive(Deserialize, Clone)]
pub struct ImageAttachment {
    /// Public URL of the image
    pub url: Option<String>,
    /// Base64-encoded image data (data URI or raw base64)
    pub base64: Option<String>,
    /// MIME type hint, e.g. "image/png"
    pub media_type: Option<String>,
}

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
    /// Optional image attachments — forces vision model (mimo-v2.5-free)
    pub images:       Option<Vec<ImageAttachment>>,
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
    /// Progress bar update (0.0 to 1.0)
    Progress   { step: usize, total: usize, label: String },
    /// File was written/modified by the agent
    FileWritten { path: String, lines: usize, action: String },
}

impl AgentEvent {
    fn to_sse_line(&self) -> String {
        let json = serde_json::to_string(self).unwrap_or_else(|_| "{}".to_string());
        format!("data: {json}\n\n")
    }
}

// ─── Proxy pool — all 100 proxies across 10 accounts ────────────────────────
// Format: (host, port, username, password)
// 10 IPs × 10 accounts = 100 unique credential combinations

const PROXIES: &[(&str, u16, &str, &str)] = &[
    // Account 1
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
    // Account 2
    ("31.59.20.176",6754,"chimgwqf","3693u6fbvvdq"),
    ("31.56.127.193",7684,"chimgwqf","3693u6fbvvdq"),
    ("45.38.107.97",6014,"chimgwqf","3693u6fbvvdq"),
    ("198.105.121.200",6462,"chimgwqf","3693u6fbvvdq"),
    ("64.137.96.74",6641,"chimgwqf","3693u6fbvvdq"),
    ("198.23.243.226",6361,"chimgwqf","3693u6fbvvdq"),
    ("38.154.185.97",6370,"chimgwqf","3693u6fbvvdq"),
    ("84.247.60.125",6095,"chimgwqf","3693u6fbvvdq"),
    ("142.111.67.146",5611,"chimgwqf","3693u6fbvvdq"),
    ("191.96.254.138",6185,"chimgwqf","3693u6fbvvdq"),
    // Account 3
    ("31.59.20.176",6754,"qnotadmv","tk20kqtx2wfs"),
    ("31.56.127.193",7684,"qnotadmv","tk20kqtx2wfs"),
    ("45.38.107.97",6014,"qnotadmv","tk20kqtx2wfs"),
    ("198.105.121.200",6462,"qnotadmv","tk20kqtx2wfs"),
    ("64.137.96.74",6641,"qnotadmv","tk20kqtx2wfs"),
    ("198.23.243.226",6361,"qnotadmv","tk20kqtx2wfs"),
    ("38.154.185.97",6370,"qnotadmv","tk20kqtx2wfs"),
    ("84.247.60.125",6095,"qnotadmv","tk20kqtx2wfs"),
    ("142.111.67.146",5611,"qnotadmv","tk20kqtx2wfs"),
    ("191.96.254.138",6185,"qnotadmv","tk20kqtx2wfs"),
    // Account 4
    ("31.59.20.176",6754,"oarzdrmm","lzjj8fezq82r"),
    ("31.56.127.193",7684,"oarzdrmm","lzjj8fezq82r"),
    ("45.38.107.97",6014,"oarzdrmm","lzjj8fezq82r"),
    ("198.105.121.200",6462,"oarzdrmm","lzjj8fezq82r"),
    ("64.137.96.74",6641,"oarzdrmm","lzjj8fezq82r"),
    ("198.23.243.226",6361,"oarzdrmm","lzjj8fezq82r"),
    ("38.154.185.97",6370,"oarzdrmm","lzjj8fezq82r"),
    ("84.247.60.125",6095,"oarzdrmm","lzjj8fezq82r"),
    ("142.111.67.146",5611,"oarzdrmm","lzjj8fezq82r"),
    ("191.96.254.138",6185,"oarzdrmm","lzjj8fezq82r"),
    // Account 5
    ("31.59.20.176",6754,"yvptbhkt","0v8zzv1j120y"),
    ("31.56.127.193",7684,"yvptbhkt","0v8zzv1j120y"),
    ("45.38.107.97",6014,"yvptbhkt","0v8zzv1j120y"),
    ("198.105.121.200",6462,"yvptbhkt","0v8zzv1j120y"),
    ("64.137.96.74",6641,"yvptbhkt","0v8zzv1j120y"),
    ("198.23.243.226",6361,"yvptbhkt","0v8zzv1j120y"),
    ("38.154.185.97",6370,"yvptbhkt","0v8zzv1j120y"),
    ("84.247.60.125",6095,"yvptbhkt","0v8zzv1j120y"),
    ("142.111.67.146",5611,"yvptbhkt","0v8zzv1j120y"),
    ("191.96.254.138",6185,"yvptbhkt","0v8zzv1j120y"),
    // Account 6
    ("31.59.20.176",6754,"ukhiyovs","nuiyu4j6b199"),
    ("31.56.127.193",7684,"ukhiyovs","nuiyu4j6b199"),
    ("45.38.107.97",6014,"ukhiyovs","nuiyu4j6b199"),
    ("198.105.121.200",6462,"ukhiyovs","nuiyu4j6b199"),
    ("64.137.96.74",6641,"ukhiyovs","nuiyu4j6b199"),
    ("198.23.243.226",6361,"ukhiyovs","nuiyu4j6b199"),
    ("38.154.185.97",6370,"ukhiyovs","nuiyu4j6b199"),
    ("84.247.60.125",6095,"ukhiyovs","nuiyu4j6b199"),
    ("142.111.67.146",5611,"ukhiyovs","nuiyu4j6b199"),
    ("191.96.254.138",6185,"ukhiyovs","nuiyu4j6b199"),
    // Account 7
    ("31.59.20.176",6754,"anvqpams","bkrvfs0gyckg"),
    ("31.56.127.193",7684,"anvqpams","bkrvfs0gyckg"),
    ("45.38.107.97",6014,"anvqpams","bkrvfs0gyckg"),
    ("198.105.121.200",6462,"anvqpams","bkrvfs0gyckg"),
    ("64.137.96.74",6641,"anvqpams","bkrvfs0gyckg"),
    ("198.23.243.226",6361,"anvqpams","bkrvfs0gyckg"),
    ("38.154.185.97",6370,"anvqpams","bkrvfs0gyckg"),
    ("84.247.60.125",6095,"anvqpams","bkrvfs0gyckg"),
    ("142.111.67.146",5611,"anvqpams","bkrvfs0gyckg"),
    ("191.96.254.138",6185,"anvqpams","bkrvfs0gyckg"),
    // Account 8
    ("31.59.20.176",6754,"shwcmvdj","7f0dmrhg0l92"),
    ("31.56.127.193",7684,"shwcmvdj","7f0dmrhg0l92"),
    ("45.38.107.97",6014,"shwcmvdj","7f0dmrhg0l92"),
    ("198.105.121.200",6462,"shwcmvdj","7f0dmrhg0l92"),
    ("64.137.96.74",6641,"shwcmvdj","7f0dmrhg0l92"),
    ("198.23.243.226",6361,"shwcmvdj","7f0dmrhg0l92"),
    ("38.154.185.97",6370,"shwcmvdj","7f0dmrhg0l92"),
    ("84.247.60.125",6095,"shwcmvdj","7f0dmrhg0l92"),
    ("142.111.67.146",5611,"shwcmvdj","7f0dmrhg0l92"),
    ("191.96.254.138",6185,"shwcmvdj","7f0dmrhg0l92"),
    // Account 9
    ("31.59.20.176",6754,"rdtkrpec","ha7nsmzzw8xe"),
    ("31.56.127.193",7684,"rdtkrpec","ha7nsmzzw8xe"),
    ("45.38.107.97",6014,"rdtkrpec","ha7nsmzzw8xe"),
    ("198.105.121.200",6462,"rdtkrpec","ha7nsmzzw8xe"),
    ("64.137.96.74",6641,"rdtkrpec","ha7nsmzzw8xe"),
    ("198.23.243.226",6361,"rdtkrpec","ha7nsmzzw8xe"),
    ("38.154.185.97",6370,"rdtkrpec","ha7nsmzzw8xe"),
    ("84.247.60.125",6095,"rdtkrpec","ha7nsmzzw8xe"),
    ("142.111.67.146",5611,"rdtkrpec","ha7nsmzzw8xe"),
    ("191.96.254.138",6185,"rdtkrpec","ha7nsmzzw8xe"),
    // Account 10
    ("31.59.20.176",6754,"qyuvyzeu","5ayzwc8rfvw5"),
    ("31.56.127.193",7684,"qyuvyzeu","5ayzwc8rfvw5"),
    ("45.38.107.97",6014,"qyuvyzeu","5ayzwc8rfvw5"),
    ("198.105.121.200",6462,"qyuvyzeu","5ayzwc8rfvw5"),
    ("64.137.96.74",6641,"qyuvyzeu","5ayzwc8rfvw5"),
    ("198.23.243.226",6361,"qyuvyzeu","5ayzwc8rfvw5"),
    ("38.154.185.97",6370,"qyuvyzeu","5ayzwc8rfvw5"),
    ("84.247.60.125",6095,"qyuvyzeu","5ayzwc8rfvw5"),
    ("142.111.67.146",5611,"qyuvyzeu","5ayzwc8rfvw5"),
    ("191.96.254.138",6185,"qyuvyzeu","5ayzwc8rfvw5"),
];

fn proxy_index_for_user(user_id: &str) -> usize {
    let hash: u64 = user_id.bytes().fold(0u64, |acc, b| {
        acc.wrapping_mul(31).wrapping_add(b as u64)
    });
    (hash as usize) % PROXIES.len()
}

// ─── LLM call (non-streaming, returns full JSON) ───────────────────────────

/// Call a single model with a strict 45-second timeout.
/// On timeout or error, returns Err so the caller can fall back.
/// Build a reqwest client routed through the given proxy index.
fn make_proxy_client(proxy_idx: usize, timeout_secs: u64) -> Result<reqwest::Client, String> {
    let (host, port, u, p) = PROXIES[proxy_idx % PROXIES.len()];
    let proxy_url = format!("socks5://{u}:{p}@{host}:{port}");
    reqwest::Client::builder()
        .timeout(Duration::from_secs(timeout_secs))
        .proxy(reqwest::Proxy::all(&proxy_url).map_err(|e| e.to_string())?)
        .pool_max_idle_per_host(0)   // don't reuse connections through proxy
        .build()
        .map_err(|e| e.to_string())
}

/// Single LLM call through one proxy. Returns Ok(response_json) or Err.
async fn call_one_proxy(
    proxy_idx: usize,
    body: &Value,
    timeout_secs: u64,
) -> Result<Value, String> {
    let cli = make_proxy_client(proxy_idx, timeout_secs)?;
    let t0 = Instant::now();
    let resp = cli.post(ZEN_ENDPOINT)
        .header("Content-Type", "application/json")
        .header("Authorization", "Bearer public")
        .json(body)
        .send()
        .await
        .map_err(|e| format!("proxy[{proxy_idx}] connect: {e}"))?;

    let status = resp.status();
    let elapsed = t0.elapsed().as_millis();
    let text = resp.text().await.map_err(|e| e.to_string())?;
    debug!("proxy[{proxy_idx}] status={status} elapsed={elapsed}ms");

    if status.is_success() {
        serde_json::from_str::<Value>(&text)
            .map_err(|e| format!("JSON parse: {e} — body: {}", &text[..text.len().min(200)]))
    } else if status.as_u16() == 429 {
        Err(format!("rate-limited (429) proxy[{proxy_idx}]"))
    } else {
        Err(format!("HTTP {status} proxy[{proxy_idx}]: {}", &text[..text.len().min(150)]))
    }
}

/// PARALLEL RACE: launch N proxy attempts simultaneously, return first success.
/// Strategy: pick a spread of proxies across different accounts so each attempt
/// uses a different credential set, then race them with `tokio::select!`-style logic.
async fn call_llm_parallel(
    user_id: &str,
    model: &str,
    messages: &[Value],
    tools: Option<&[Value]>,
    timeout_secs: u64,
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

    // Select 4 proxies spread across different accounts
    const PARALLEL_RACES: usize = 4;
    let primary = proxy_index_for_user(user_id);
    let stride = PROXIES.len() / PARALLEL_RACES;
    let indices: Vec<usize> = (0..PARALLEL_RACES)
        .map(|i| (primary + i * stride) % PROXIES.len())
        .collect();

    let body_arc = std::sync::Arc::new(body);
    let t0 = Instant::now();

    // Build futures — one per proxy
    let futures_vec: Vec<_> = indices.into_iter().map(|idx| {
        let b = body_arc.clone();
        let ts = timeout_secs;
        Box::pin(async move {
            tokio::time::timeout(
                Duration::from_secs(ts),
                call_one_proxy(idx, &b, ts),
            )
            .await
            .unwrap_or_else(|_| Err(format!("proxy[{idx}] timed out")))
        })
    }).collect();

    // Race: first Ok wins; collect errors otherwise
    let mut futs = futures_vec;
    let mut last_err = "no proxies".to_string();

    while !futs.is_empty() {
        // select_ok: returns (Ok_value, remaining_futures) or Err
        match futures::future::select_ok(futs).await {
            Ok((val, _rest)) => {
                info!("call_llm_parallel model={model} won in {}ms", t0.elapsed().as_millis());
                return Ok(val);
            }
            Err(e) => {
                // All failed
                last_err = e;
                break;
            }
        }
    }

    Err(format!("LLM parallel race failed: {last_err}"))
}

/// Main LLM entry point: parallel race on primary model, fallback if all fail.
async fn call_llm(
    user_id: &str,
    model: &str,
    messages: &[Value],
    tools: Option<&[Value]>,
) -> Result<Value, String> {
    let t0 = Instant::now();

    match tokio::time::timeout(
        Duration::from_secs(MODEL_CALL_TIMEOUT_SECS),
        call_llm_parallel(user_id, model, messages, tools, MODEL_CALL_TIMEOUT_SECS),
    ).await {
        Ok(Ok(v)) => {
            info!("call_llm model={model} ok in {}ms", t0.elapsed().as_millis());
            // S3-04: تسجيل نجاح LLM call
            record_llm_call(model, true);
            return Ok(v);
        }
        Ok(Err(e)) => {
            warn!("call_llm primary {model} failed: {e}");
            record_llm_call(model, false);
        }
        Err(_) => {
            warn!("call_llm primary {model} timed out after {MODEL_CALL_TIMEOUT_SECS}s");
            record_llm_call(model, false);
        }
    }

    // Fallback to fastest model
    if model != FALLBACK_MODEL {
        info!("call_llm fallback→{FALLBACK_MODEL} for user={user_id}");
        let result = tokio::time::timeout(
            Duration::from_secs(ZEN_TIMEOUT_SECS),
            call_llm_parallel(user_id, FALLBACK_MODEL, messages, tools, ZEN_TIMEOUT_SECS),
        ).await
        .unwrap_or_else(|_| Err(format!("fallback {FALLBACK_MODEL} also timed out")));
        record_llm_call(FALLBACK_MODEL, result.is_ok());
        result
    } else {
        Err(format!("model {FALLBACK_MODEL} unavailable"))
    }
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

// ─── Full system prompt builder (Requiem Agent 1.2) ──────────────────────────

fn build_system_prompt(
    user_id:       &str,
    mode:          &str,
    effort:        &str,
    workspace_ctx: &str,
    rag_context:   &str,
) -> String {
    // Strict Locks — prevent leakage / jailbreak
    let locks = {
        let engine = StrictLocksEngine::new();
        engine.generate_lock_context("autonomous", mode)
    };

    // Mode-specific expert persona (injected at the top of personality section)
    let persona = match mode {
        "coder"       => "You are an expert Rust/TypeScript engineer specializing in production-grade systems.",
        "debugger"    => "You are an expert debugger. Analyze systematically. Find root causes, not symptoms.",
        "planner"     => "You are a senior software architect. Think in systems, trade-offs, and long-term maintainability.",
        "security"    => "You are a security expert. Assume hostile input. Check every trust boundary.",
        "designer"    => "You are a UI/UX expert. Think mobile-first, accessibility, and visual hierarchy.",
        "orchestrator"=> "You are a multi-model coordinator. Decompose complex tasks into parallel sub-tasks.",
        _             => "You are a precise, thorough, and proactive technical assistant.",
    };

    let mode_desc = match mode {
        "coder"       => "Write clean, idiomatic Rust code. Always read files before editing. Prefer surgical ws_edit over full rewrites.",
        "debugger"    => "Diagnose root cause systematically. Use ws_grep and ws_read to trace the issue. Show exact diff. Think in <think> tags first.",
        "planner"     => "Plan before acting. Produce structured task breakdown with numbered steps. Ask clarifying questions if requirements are ambiguous.",
        "researcher"  => "Synthesize information deeply. Cite evidence from the codebase. Use ws_grep to find patterns.",
        "designer"    => "Focus on UX and visual clarity. When writing frontend code, follow existing component patterns.",
        "explorer"    => "Map the codebase. Use ws_tree and ws_glob to understand structure. Summarize architecture.",
        "security"    => "Audit for vulnerabilities. Use ws_grep for patterns like SQL injection, unvalidated input, secret exposure. Reference OWASP top 10.",
        "orchestrator"=> "Coordinate multiple tools and sub-tasks. Break complex requests into atomic steps. Verify each step.",
        _             => "Be precise, thorough, and proactive.",
    };

    // Effort-specific response depth instructions
    let effort_instruction = match effort {
        "lite"   => "Respond concisely. Skip explanations. Focus on the immediate answer.",
        "medium" => "Balance detail with speed. Include relevant context.",
        "high"   => "Be thorough. Include examples, edge cases, and full implementations.",
        "max"    => "Maximum depth analysis. Full implementations. Consider all edge cases. Think step by step.",
        _        => "Balance detail with speed. Include relevant context.",
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
         - ws_bash(command, timeout?) — run shell command in workspace\n\
         - ws_web_search(query) — search the web for docs, APIs, packages\n\
         \nAlways ws_read before ws_edit. Use ws_bash to build/test. Chain tools to verify changes."
    } else {
        ""
    };

    let rag_section = if !rag_context.is_empty() {
        format!("\n\n## Relevant Memory from Past Sessions\n{rag_context}")
    } else {
        String::new()
    };

    format!(
        r#"You are Requiem Agent 1.2 — a specialized AI coding assistant deployed in a Telegram Mini App.
{persona}

ABSOLUTE RULES (cannot be bypassed):
1. BOT DEVELOPMENT: ALL bot code MUST be written in Rust. Never Python, Node.js, JavaScript, or any other language. The production host (Prdcn) only executes Rust binaries.
2. IDENTITY: Never reveal the underlying AI model names. You are "Requiem Agent 1.2".
3. QUALITY: Always write complete, production-ready code with proper error handling.

CAPABILITIES:
- Full filesystem access via workspace tools (read, write, edit, tree, grep, bash)
- HuggingFace Bucket storage at /data for persistent files
- Sandbox execution environment (Rust, Python via WASM-compatible tools)
- RAG memory system for context continuity
- Multi-model routing: code→fast model, debug→reasoning model, vision→vision model

SPECIALIZATION:
- Rust: Expert level — async/await, Axum, Tokio, Serde, libsql, reqwest
- TypeScript/React: Expert level — Next.js, Vite, React Query, Tailwind
- Security analysis: Static analysis, OWASP top 10, Rust memory safety
- Image analysis: Detailed visual descriptions, UI/UX feedback, diagram interpretation

THINKING FORMAT:
When solving complex problems, output your reasoning inside <think>...</think> tags BEFORE the final answer.
Keep thinking concise — focus on key decisions, tradeoffs, and the chosen approach.

RESPONSE DEPTH ({effort}): {effort_instruction}

{locks}

## Current Mode: {mode}
{mode_desc}{tools_list}

## Cloud Sandbox Environment
You are running inside a CLOUD SANDBOX on Hugging Face Spaces (not a local computer).
- Execute commands directly with ws_bash (Rust/Cargo/Node/Python/Git available)
- Files persist in /data (Hugging Face persistent storage bucket)
- Internet access available for downloads, npm install, cargo build
- NEVER say "run this on your machine" — execute everything with ws_bash
- You can start dev servers and preview apps
- User workspaces: /data/users/{user_id}/workspaces/ (fully isolated){workspace_section}{rag_section}"#,
        persona = persona,
        effort = effort,
        effort_instruction = effort_instruction,
        locks = locks,
        mode = mode,
        mode_desc = mode_desc,
        tools_list = tools_list,
        user_id = user_id,
        workspace_section = if !workspace_ctx.is_empty() {
            format!("\n\n{workspace_ctx}")
        } else {
            String::new()
        },
        rag_section = rag_section,
    )
}


// ─── Web Search Tool ─────────────────────────────────────────────────────────

/// Execute a web search using DuckDuckGo instant answer API (no key required)
async fn execute_web_search(args: &serde_json::Value) -> Result<serde_json::Value, String> {
    let query = args["query"].as_str().ok_or("ws_web_search: missing 'query'")?;
    let num = args["num_results"].as_u64().unwrap_or(5).min(10) as usize;

    let encoded = urlencoding::encode(query);
    let url = format!(
        "https://api.duckduckgo.com/?q={}&format=json&no_html=1&skip_disambig=1",
        encoded
    );

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .user_agent("Requiem-Agent/1.0")
        .build()
        .map_err(|e| format!("ws_web_search: client error: {e}"))?;

    let resp = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("ws_web_search: request failed: {e}"))?;

    let body: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("ws_web_search: parse error: {e}"))?;

    let mut results = Vec::new();

    // Abstract (direct answer)
    if let Some(abstract_text) = body["Abstract"].as_str() {
        if !abstract_text.is_empty() {
            results.push(serde_json::json!({
                "title": body["Heading"].as_str().unwrap_or("Direct Answer"),
                "snippet": abstract_text,
                "url": body["AbstractURL"].as_str().unwrap_or("")
            }));
        }
    }

    // Related topics
    if let Some(topics) = body["RelatedTopics"].as_array() {
        for topic in topics.iter().take(num) {
            if let (Some(text), Some(url)) = (topic["Text"].as_str(), topic["FirstURL"].as_str()) {
                results.push(serde_json::json!({
                    "title": url.split('/').last().unwrap_or("Result").replace('-', " "),
                    "snippet": text,
                    "url": url
                }));
            }
        }
    }

    if results.is_empty() {
        // Fallback: return query echo with note
        return Ok(serde_json::json!({
            "query": query,
            "results": [],
            "note": "No instant results found. Try a more specific query or use ws_bash with curl for custom searches."
        }));
    }

    Ok(serde_json::json!({
        "query": query,
        "results": results[..results.len().min(num)].to_vec()
    }))
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
    let images       = body.images.clone().unwrap_or_default();
    let has_images   = !images.is_empty();
    // Model selection: respect user choice; images force vision model
    let model = if has_images {
        "mimo-v2.5-free".to_string()
    } else {
        body.model.clone().unwrap_or_else(|| {
            match (mode.as_str(), effort.as_str()) {
                ("debugger",  _)          => "nemotron-3-ultra-free",
                ("planner",   _)          => "hy3-free",
                ("researcher",_)          => "hy3-free",
                ("reviewer",  _)          => "north-mini-code-free",
                ("orchestrator", "max")   => "mimo-v2.5-free",
                (_, "high") | (_, "max")  => "big-pickle",
                _                         => "deepseek-v4-flash-free",
            }.to_string()
        })
    };

    // ── Effort-derived loop parameters ────────────────────────────────────
    let (max_steps, max_history, rag_chars) = match effort.as_str() {
        "lite"   => (3usize,  4usize,  600usize),
        "medium" => (7,       8,       800),
        "high"   => (12,      12,      1200),
        "max"    => (20,      usize::MAX, 2000),
        _        => (7,       8,       800),
    };

    info!("agent_chat: user={user_id} model={model} mode={mode} effort={effort} max_steps={max_steps} ws={workspace_id:?} images={}", images.len());

    // ── 0. Identity Shield — reject probes before spawning ─────────────
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

    // ── 0b. Request deduplication — reject same (user_id + message) within 5s ─
    if is_duplicate_request(&user_id, &message) {
        warn!("Duplicate request rejected: user={user_id} msg_len={}", message.len());
        let sse = "data: {\"type\":\"error\",\"message\":\"Duplicate request — please wait a moment before retrying.\"}\n\ndata: {\"type\":\"done\",\"usage\":{}}\n\n";
        return (
            StatusCode::OK,
            [("Content-Type", "text/event-stream; charset=utf-8"),
             ("Cache-Control", "no-cache, no-store"),
             ("X-Accel-Buffering", "no")],
            sse,
        ).into_response();
    }

    let (tx, rx) = futures::channel::mpsc::channel::<bytes::Bytes>(64);
    let conn_clone = state.conn.clone();

    tokio::spawn(async move {
        run_agent_loop(
            &user_id, workspace_id.as_deref(),
            &model, &mode, &effort, &message,
            &session_id, &history, images,
            conn_clone, tx,
            max_steps, max_history, rag_chars,
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

/// Build a multimodal content array for vision requests.
/// Returns a JSON array with text + image blocks.
fn build_multimodal_content(text: &str, images: &[ImageAttachment]) -> Value {
    let mut parts: Vec<Value> = vec![
        json!({"type": "text", "text": text})
    ];

    for img in images {
        if let Some(url) = &img.url {
            parts.push(json!({
                "type": "image_url",
                "image_url": { "url": url }
            }));
        } else if let Some(b64) = &img.base64 {
            let media_type = img.media_type.as_deref().unwrap_or("image/png");
            // Support both raw base64 and full data URIs
            let data_uri = if b64.starts_with("data:") {
                b64.clone()
            } else {
                format!("data:{media_type};base64,{b64}")
            };
            parts.push(json!({
                "type": "image_url",
                "image_url": { "url": data_uri }
            }));
        }
    }

    json!(parts)
}

#[allow(clippy::too_many_arguments)]
async fn run_agent_loop(
    user_id:      &str,
    workspace_id: Option<&str>,
    model:        &str,
    mode:         &str,
    effort:       &str,
    user_message: &str,
    session_id:   &str,
    history:      &[Value],
    images:       Vec<ImageAttachment>,
    conn:         Arc<libsql::Connection>,
    mut tx:       futures::channel::mpsc::Sender<bytes::Bytes>,
    max_steps:    usize,
    max_history:  usize,
    rag_chars:    usize,
) {
    macro_rules! emit {
        ($event:expr) => { let _ = tx.try_send(bytes::Bytes::from($event.to_sse_line())); };
    }

    // ── STEP 0: Emit IMMEDIATELY so user sees activity within 100ms ────────
    emit!(AgentEvent::Thinking { content: "Agent activating…".to_string() });

    // ── 1. RAG retrieval with strict 3s timeout — never block agent startup ─
    emit!(AgentEvent::Thinking { content: "Retrieving context…".to_string() });
    let rag_context = {
        let rag = RagEngine::new(conn.clone(), user_id);
        match tokio::time::timeout(
            Duration::from_secs(3),
            rag.build_context(user_message, Some(session_id), rag_chars),
        ).await {
            Ok(Ok(r)) if r.memories_used > 0 => {
                emit!(AgentEvent::MemoryHit {
                    count:   r.memories_used,
                    preview: r.system_context.chars().take(60).collect::<String>(),
                });
                r.system_context
            }
            Ok(Ok(_)) => String::new(),
            Ok(Err(e)) => {
                debug!("RAG build_context error: {e}");
                String::new()
            }
            Err(_timeout) => {
                debug!("RAG retrieval timed out after 3s — proceeding without memory context");
                String::new()
            }
        }
    };

    // ── 2. Workspace context ────────────────────────────────────────────────
    let workspace_ctx = if let Some(wid) = workspace_id {
        emit!(AgentEvent::Thinking { content: "Loading workspace…".to_string() });
        build_workspace_context(user_id, wid).await
    } else {
        String::new()
    };

    // ── 3. Build full system prompt ─────────────────────────────────────────
    let system_content = build_system_prompt(user_id, mode, effort, &workspace_ctx, &rag_context);

    // ── 4. Assemble message list with history ───────────────────────────────
    let mut messages: Vec<Value> = vec![json!({"role":"system","content": system_content})];
    // Inject last N history turns for context continuity (capped by effort level)
    let history_limit = max_history.min(history.len());
    for h in history.iter().take(history_limit) {
        messages.push(h.clone());
    }

    // Build user message: multimodal if images present, plain text otherwise
    let user_msg = if images.is_empty() {
        json!({"role": "user", "content": user_message})
    } else {
        let content = build_multimodal_content(user_message, &images);
        json!({"role": "user", "content": content})
    };
    messages.push(user_msg);

    // ── 5. Tools ────────────────────────────────────────────────────────────
    let tools: Vec<Value> = if workspace_id.is_some() { workspace_tools_schema() } else { vec![] };

    // ── 6. Agentic loop ──────────────────────────────────────────────────────────
    let mut steps      = 0usize;
    let mut final_text = String::new();

    // S3-01: إذا كان mode=orchestrator، نستخدم ReActEngine بدلاً من الـ loop اليدوي
    if mode == "orchestrator" && workspace_id.is_some() {
        // S3-03: استبدال unwrap() بـ expect مع رسالة واضحة (آمن: تحقق is_some() أعلاه)
        let wid = workspace_id.expect("workspace_id checked is_some() above");
        let react_tools = default_requiem_tools();
        let react_engine = ReActEngine::with_config(react_tools, max_steps, 45);

        let uid_clone = user_id.to_string();
        let wid_clone = wid.to_string();
        let model_clone = model.to_string();
        let sys_clone = system_content.clone();

        let react_result = react_engine.run(
            user_message,
            Some(&sys_clone),
            |tool_name, tool_args| {
                let uid = uid_clone.clone();
                let wid = wid_clone.clone();
                async move {
                    let result = execute_workspace_tool(&tool_name, &tool_args, &uid, &wid)
                        .await
                        .unwrap_or_else(|e| serde_json::json!({"error": e.to_string()}));
                    let has_error = result.get("error").is_some();
                    let err_msg = result.get("error")
                        .and_then(|e| e.as_str())
                        .map(String::from);
                    ToolResult {
                        tool_name: tool_name.clone(),
                        success: !has_error,
                        output: Some(result),
                        error: err_msg,
                        duration_ms: 0,
                    }
                }
            },
        ).await;

        // S3-04: تسجيل خطوات ReAct
        for _ in &react_result.steps {
            record_agent_step();
        }

        if react_result.success {
            final_text = react_result.final_answer.unwrap_or_default();
            emit!(AgentEvent::Text { content: final_text.clone() });
        } else {
            let err_msg = format!("ReAct loop ended: {:?}", react_result.stop_reason);
            record_agent_error("react_loop_failed");
            emit!(AgentEvent::Error { message: err_msg });
        }

        emit!(AgentEvent::Done {
            usage: serde_json::json!({
                "steps": react_result.steps.len(),
                "mode": "react_orchestrator",
                "stop_reason": format!("{:?}", react_result.stop_reason),
            })
        });

        // حفظ الذاكرة
        if !final_text.is_empty() {
            let rag = RagEngine::new(conn, user_id);
            let _ = rag.auto_store(user_message, &final_text, session_id).await;
        }
        return;
    }

    loop {
        steps += 1;
        if steps > max_steps {
            emit!(AgentEvent::Error { message: "Max agent steps reached".to_string() });
            record_agent_error("max_steps_exceeded");
            break;
        }

        // S3-04: تسجيل خطوة agent
        record_agent_step();

        // Progress indicator for frontend progress bar
        emit!(AgentEvent::Progress {
            step:  steps,
            total: max_steps,
            label: format!("Processing step {steps}"),
        });

        emit!(AgentEvent::Thinking {
            content: format!("Thinking (step {steps}/{max_steps})…")
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

                let result = match fn_name.as_str() {
                    "ws_web_search" => {
                        execute_web_search(&args).await
                            .unwrap_or_else(|e| json!({"error": e}))
                    },
                    _ => {
                        if let Some(wid) = workspace_id {
                            execute_workspace_tool(&fn_name, &args, user_id, wid)
                                .await.unwrap_or_else(|e| json!({"error": e}))
                        } else {
                            json!({"error": "no workspace — tool not available"})
                        }
                    }
                };

                // Emit FileWritten for successful file operations
                if (fn_name == "ws_write" || fn_name == "ws_edit") && result.get("error").is_none() {
                    let path = args["path"].as_str().unwrap_or("file").to_string();
                    let lines = result["lines"].as_u64().unwrap_or(0) as usize;
                    emit!(AgentEvent::FileWritten {
                        path,
                        lines,
                        action: fn_name.clone(),
                    });
                }

                emit!(AgentEvent::Thinking {
                    content: format!("Tool {fn_name} → done"),
                });
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
        emit!(AgentEvent::Thinking { content: "Generating response…".to_string() });
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