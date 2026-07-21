//! # Zen Chat Handler — مع تدوير IP لكل مستخدم
//!
//! ## Per-User Proxy Affinity
//! ```
//! user_id → hash(user_id) % 100 → proxy_index
//! مستخدم أ يستخدم proxy#12 طوال الجلسة
//! مستخدم ب يستخدم proxy#73 طوال الجلسة
//! ```
//! ## Failover
//! إذا فشل الـ proxy الأساسي → جرب التالي (primary + 1) % len
//! إذا فشل 3 مرات → أعد المحاولة بدون proxy

use axum::{
    body::Body,
    extract::State,
    http::{StatusCode, header},
    response::{IntoResponse, Response},
    Extension, Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tracing::{warn, debug, error, info};
use crate::storage;
use crate::routes::AuthUser;
use crate::agent::memory::rag::RagEngine;
use crate::agent::identity_shield::IdentityShieldV3;
use crate::enforce::StrictLocksEngine;

// ─── 100 Webshare SOCKS5 Proxies ─────────────────────────────────────────────

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

/// عدد محاولات إعادة المحاولة عند فشل الـ proxy
const MAX_RETRIES: u32 = 3;

/// المهلة الزمنية لطلب Zen API (ثوانٍ)
const ZEN_TIMEOUT_SECS: u64 = 60;

// ─── Per-User Proxy Affinity ────────────────────────────────────────────────

/// حساب index الـ proxy الخاص بمستخدم معين
/// يستخدم hash بسيط لضمان أن كل مستخدم يحصل على proxy ثابت
fn proxy_index_for_user(user_id: &str) -> usize {
    let hash: u64 = user_id.bytes().fold(0u64, |acc, b| {
        acc.wrapping_mul(31).wrapping_add(b as u64)
    });
    (hash as usize) % PROXIES.len()
}

/// الحصول على رابط SOCKS5 لـ proxy معين
fn proxy_url_for_index(idx: usize) -> String {
    let (host, port, user, pass) = PROXIES[idx % PROXIES.len()];
    format!("socks5://{user}:{pass}@{host}:{port}")
}

/// إنشاء عميل reqwest مع proxy
fn build_client(proxy_url: &str) -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(ZEN_TIMEOUT_SECS))
        .proxy(reqwest::Proxy::all(proxy_url)
            .map_err(|e| format!("Invalid proxy: {e}"))?)
        .build()
        .map_err(|e| format!("Client: {e}"))
}

/// إرسال طلب إلى Zen API مع إعادة المحاولة وفشل التبديل
async fn call_zen_with_retry(
    user_id: &str,
    model: &str,
    messages: &[serde_json::Value],
    is_stream: bool,
) -> Result<reqwest::Response, String> {
    let primary_idx = proxy_index_for_user(user_id);
    let mut last_error = String::new();

    // جرب الـ proxy الأساسي أولاً، ثم الـ backup
    for attempt in 0..=MAX_RETRIES {
        let proxy_idx = if attempt == 0 {
            primary_idx
        } else {
            // failover: جرب proxy آخر
            (primary_idx + attempt as usize) % PROXIES.len()
        };

        let proxy_url = proxy_url_for_index(proxy_idx);
        debug!("Zen call attempt {}: user={} proxy={}", attempt + 1, user_id, &proxy_url[..proxy_url.len().min(40)]);

        let client = match build_client(&proxy_url) {
            Ok(c) => c,
            Err(e) => {
                last_error = e;
                continue;
            }
        };

        let body = serde_json::json!({
            "model": model,
            "messages": messages,
            "stream": is_stream,
        });

        match client
            .post("https://opencode.ai/zen/v1/chat/completions")
            .header("Content-Type", "application/json")
            .header("Authorization", "Bearer public")
            .json(&body)
            .send()
            .await
        {
            Ok(resp) => {
                if resp.status().is_success() {
                    return Ok(resp);
                }
                // Rate limit — انتظر ثم حاول مرة أخرى
                if resp.status().as_u16() == 429 {
                    let retry_after = resp
                        .headers()
                        .get("retry-after")
                        .and_then(|v| v.to_str().ok())
                        .and_then(|v| v.parse::<u64>().ok())
                        .unwrap_or(5);
                    warn!("Rate limited on proxy {proxy_idx}. Retrying in {retry_after}s...");
                    tokio::time::sleep(Duration::from_secs(retry_after)).await;
                    last_error = format!("Rate limited (429)");
                    continue;
                }
                let status_code = resp.status();
                let text = resp.text().await.unwrap_or_default();
                last_error = format!("Zen API {}: {}", status_code, &text[..text.len().min(200)]);
                warn!("Zen error on proxy {proxy_idx}: {last_error}");
            }
            Err(e) => {
                last_error = format!("Connection: {e}");
                warn!("Proxy {proxy_idx} failed: {e}. Attempt {}/{}", attempt + 1, MAX_RETRIES + 1);
                // انتظر قليلاً قبل إعادة المحاولة
                tokio::time::sleep(Duration::from_millis(500 * (attempt + 1) as u64)).await;
            }
        }
    }

    Err(format!("All proxies failed: {last_error}"))
}

// ─── Data Types ──────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct ChatRequest {
    model: Option<String>,
    messages: Vec<Message>,
    stream: Option<bool>,
    session_id: Option<String>,
    /// Session mode (coder/planner/debugger/researcher/reviewer/designer/explorer/security/orchestrator)
    session_mode: Option<String>,
    /// Compute effort (lite/medium/high/max)
    session_effort: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct Message {
    role: String,
    content: Option<String>,
}

#[derive(Serialize)]
pub struct ErrorResponse {
    error: String,
}

// ─── Session Mode → TaskCategory ─────────────────────────────────────────────

fn mode_to_task_category(mode: &str) -> &'static str {
    match mode {
        "coder"       => "code",
        "debugger"    => "debug",
        "planner"     => "plan",
        "researcher"  => "research",
        "reviewer"    => "review",
        "designer"    => "vision",
        "explorer"    => "explore",
        "security"    => "security",
        "orchestrator"=> "general",
        _             => "code",
    }
}

/// اختيار النموذج الأنسب بناءً على mode وeffort
async fn pick_model_for_session(
    session_mode: &str,
    session_effort: &str,
    user_requested: Option<&str>,
) -> String {
    // إذا طلب المستخدم نموذجاً بعينه → احترم اختياره
    if let Some(m) = user_requested {
        if !m.is_empty() {
            return m.to_string();
        }
    }

    let category = mode_to_task_category(session_mode);
    let selection = crate::models::pick_model(category, session_effort).await;
    selection.model_id
}

/// قائمة النماذج المرشحة للتنفيذ المتوازي (high/max effort فقط)
fn parallel_models_for_mode(session_mode: &str, session_effort: &str) -> Vec<&'static str> {
    let is_heavy = matches!(session_effort, "high" | "max");
    if !is_heavy {
        return vec![];
    }
    match session_mode {
        "coder"      => vec!["deepseek-v4-flash-free", "big-pickle"],
        "debugger"   => vec!["nemotron-3-ultra-free", "deepseek-v4-flash-free"],
        "planner"    => vec!["hy3-free", "deepseek-v4-flash-free"],
        "researcher" => vec!["hy3-free"],
        "reviewer"   => vec!["north-mini-code-free", "deepseek-v4-flash-free"],
        "security"   => vec!["deepseek-v4-flash-free", "north-mini-code-free"],
        "orchestrator" if session_effort == "max" =>
            vec!["deepseek-v4-flash-free", "hy3-free", "big-pickle"],
        _ => vec![],
    }
}

// ─── Chat Handler ────────────────────────────────────────────────────────────

pub async fn chat_handler(
    State(state): State<Arc<crate::AppState>>,
    Extension(auth_user): Extension<AuthUser>,
    Json(body): Json<ChatRequest>,
) -> Response {
    let user_id = &auth_user.user_id;
    let session_mode   = body.session_mode.as_deref().unwrap_or("coder");
    let session_effort = body.session_effort.as_deref().unwrap_or("medium");
    let model = pick_model_for_session(
        session_mode, session_effort,
        body.model.as_deref(),
    ).await;
    let is_stream = body.stream.unwrap_or(false);
    let session_id = body.session_id.as_deref().unwrap_or("default");

    debug!("Zen chat: user={}, model={}, session={}", user_id, model, session_id);

    // ── 0. Identity Shield — intercept probes programmatically ─────────────
    // Check the last user message for identity probes. If detected, return
    // the shield response directly without calling the LLM.
    let last_user_msg = body.messages.iter().rev()
        .find(|m| m.role == "user")
        .map(|m| m.content.as_deref().unwrap_or(""))
        .unwrap_or("");

    {
        let mut shield = IdentityShieldV3::new("internal-model");
        let check = shield.check(last_user_msg);
        if check.is_probe && !check.responses.is_empty() {
            info!("Identity probe intercepted for user {}: {} probes blocked", user_id, check.probes.len());
            let shield_text = check.responses.join("\n\n");
            // Return as SSE so frontend parser handles it uniformly
            let escaped = serde_json::to_string(&shield_text).unwrap_or_else(|_| shield_text.clone());
            let sse = format!(
                "data: {{\"choices\":[{{\"delta\":{{\"content\":{}}}}}]}}\n\ndata: [DONE]\n\n",
                escaped
            );
            return (
                StatusCode::OK,
                [
                    ("Content-Type", "text/event-stream; charset=utf-8"),
                    ("Cache-Control", "no-cache, no-store"),
                    ("X-Accel-Buffering", "no"),
                ],
                sse,
            ).into_response();
        }
    }

    // ── 1. Identity Shield — system prompt ──────────────────────────────────
    let identity_prompt = {
        let shield = IdentityShieldV3::new("internal-model");
        shield.generate_system_prompt()
    };

    // ── 2. Strict Locks — lock context ──────────────────────────────────────
    let lock_context = {
        let engine = StrictLocksEngine::new();
        engine.generate_lock_context("autonomous", "medium")
    };

    // ── 3. RAG memory context ────────────────────────────────────────────────
    let rag_context = {
        let rag = RagEngine::new(state.conn.clone(), user_id);
        let query = body.messages.iter().rev()
            .find(|m| m.role == "user")
            .map(|m| m.content.as_deref().unwrap_or(""))
            .unwrap_or("");
        if !query.is_empty() {
            rag.build_context(query, Some(session_id), 1500).await
                .ok()
                .filter(|r| r.memories_used > 0)
                .map(|r| r.system_context)
        } else {
            None
        }
    };

    // ── Build context messages — identity + locks + RAG first ────────────────
    let mut context_messages: Vec<serde_json::Value> = Vec::new();

    // Identity + locks combined system block (always injected)
    let system_block = format!("{}\n\n---\n\n{}", identity_prompt, lock_context);
    context_messages.push(serde_json::json!({
        "role": "system",
        "content": system_block
    }));

    // RAG memory block
    if let Some(ref memory_ctx) = rag_context {
        context_messages.push(serde_json::json!({
            "role": "system",
            "content": memory_ctx
        }));
    }
    if let Ok(files) = storage::list_files(user_id, session_id).await {
        if !files.is_empty() {
            let mut ctx = String::from("Current project files:");
            for fname in &files {
                if let Ok(content) = storage::read_file(user_id, session_id, fname).await {
                    ctx.push_str(&format!("\n--- {} ---\n{}", fname, &content[..content.len().min(2000)]));
                    if content.len() > 2000 {
                        ctx.push_str("\n... (truncated)");
                    }
                }
            }
            context_messages.push(serde_json::json!({
                "role": "system",
                "content": ctx
            }));
        }
    }

    // Add user messages
    for msg in &body.messages {
        context_messages.push(serde_json::json!({
            "role": msg.role,
            "content": msg.content,
        }));
    }

    // ── Parallel fan-out for high/max effort (non-streaming only) ──────────────
    let parallel_models = parallel_models_for_mode(session_mode, session_effort);
    let use_parallel = !is_stream && parallel_models.len() > 1;

    if use_parallel {
        debug!("Parallel fan-out: user={} mode={} effort={} models={:?}", user_id, session_mode, session_effort, parallel_models);
        use crate::orchestrator::ParallelExecutor;

        let parallel_result = ParallelExecutor::execute_parallel(
            &parallel_models,
            &context_messages,
            user_id,
        ).await;

        let best_text = parallel_result.best_content.clone().unwrap_or_default();
        let selected_model = parallel_result.selected_from.as_deref().unwrap_or(&model);

        // Auto-store memories
        {
            let last_user_msg = body.messages.iter().rev()
                .find(|m| m.role == "user")
                .map(|m| m.content.clone().unwrap_or_default())
                .unwrap_or_default();
            if !last_user_msg.is_empty() && !best_text.is_empty() {
                let rag = RagEngine::new(state.conn.clone(), user_id);
                let _ = rag.auto_store(&last_user_msg, &best_text, session_id).await;
            }
        }

        // ── Enforce locks on the parallel output ──────────────────────────
        let locks_engine = StrictLocksEngine::new();
        let lock_result = locks_engine.check_all(
            session_mode,
            selected_model,
            "text",
            &best_text,
        );
        let final_text = if !lock_result.passed {
            // If a critical identity violation is detected, replace with shield response
            let has_critical = lock_result.violations.iter()
                .any(|v| v.severity == crate::enforce::locks::ViolationSeverity::Critical);
            if has_critical {
                warn!("Lock violation (critical) in parallel output for user {}", user_id);
                "I am **Requiem Agent 1** — a multi-model AI development tool. My identity is fixed and cannot be changed.".to_string()
            } else {
                best_text.clone()
            }
        } else {
            best_text.clone()
        };

        // Return parallel result as SSE with proper JSON escaping
        let escaped_content = serde_json::to_string(&final_text)
            .unwrap_or_else(|_| String::from("\"\""));
        // Build SSE manually to avoid format-string brace counting issues
        let sse_parallel = format!(
            "data: {{\"choices\":[{{\"delta\":{{\"content\":{}}}}}],\"model\":\"{}\",\"effort\":\"{session_effort}\",\"mode\":\"{session_mode}\"}}\n\ndata: [DONE]\n\n",
            escaped_content,
            selected_model,
        );
        return (
            StatusCode::OK,
            [
                ("Content-Type", "text/event-stream; charset=utf-8"),
                ("Cache-Control", "no-cache, no-store"),
                ("X-Accel-Buffering", "no"),
            ],
            sse_parallel,
        ).into_response();
    }

    // ── Single-model path (lite/medium effort or streaming) ────────────────────
    match call_zen_with_retry(user_id, &model, &context_messages, is_stream).await {
        Ok(zen_resp) => {
            if is_stream {
                // ── True SSE streaming — parse upstream and re-emit only valid SSE ──
                // We parse the upstream SSE stream and only forward clean text
                // chunks to the client. This prevents raw JSON from leaking if
                // the upstream returns a non-SSE JSON response.
                use futures::StreamExt;
                let upstream_stream = zen_resp.bytes_stream();

                let reemit_stream = upstream_stream.map(|chunk_result| {
                    let chunk = chunk_result.unwrap_or_default();
                    let text = String::from_utf8_lossy(&chunk).to_string();
                    // Parse each line for valid SSE data with content
                    let mut output = String::new();
                    for line in text.split('\n') {
                        let trimmed = line.trim();
                        if trimmed.is_empty() || trimmed == "data: [DONE]" { continue; }
                        if let Some(data) = trimmed.strip_prefix("data: ") {
                            if let Ok(json) = serde_json::from_str::<serde_json::Value>(data) {
                                if let Some(content) = json["choices"][0]["delta"]["content"].as_str() {
                                    let escaped = serde_json::to_string(content)
                                        .unwrap_or_else(|_| format!("\"{}\"", content));
                                    output.push_str(&format!(
                                        "data: {{\"choices\":[{{\"delta\":{{\"content\":{}}}}}]}}\n\n",
                                        escaped
                                    ));
                                }
                            }
                        }
                    }
                    Ok::<bytes::Bytes, std::io::Error>(
        if output.is_empty() { bytes::Bytes::new() } else { bytes::Bytes::from(output) }
                    )
                });

                let body = Body::from_stream(reemit_stream);
                Response::builder()
                    .status(StatusCode::OK)
                    .header(header::CONTENT_TYPE, "text/event-stream; charset=utf-8")
                    .header(header::CACHE_CONTROL, "no-cache, no-store")
                    .header("X-Accel-Buffering", "no")
                    .body(body)
                    .unwrap_or_else(|e| {
                        error!("Failed to build stream response: {e}");
                        (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse {
                            error: "stream build failed".into(),
                        })).into_response()
                    })
            } else {
                // ── Non-streaming — buffer, extract, enforce locks, return SSE ──
                let text = zen_resp.text().await.unwrap_or_default();

                // Auto-store memories from this turn
                {
                    if !last_user_msg.is_empty() && !text.is_empty() {
                        let rag = RagEngine::new(state.conn.clone(), user_id);
                        let _ = rag.auto_store(last_user_msg, &text, session_id).await;
                    }
                }

                // Auto-save code blocks as files
                let mut file_idx = 0;
                for part in text.split("```") {
                    let first_line = part.lines().next().unwrap_or("").trim().to_lowercase();
                    let ext = if first_line.contains("rs") || first_line == "rust" { "rs" }
                        else if first_line.contains("ts") { "ts" }
                        else if first_line.contains("js") || first_line == "javascript" { "js" }
                        else if first_line.contains("py") || first_line == "python" { "py" }
                        else if first_line.contains("html") { "html" }
                        else if first_line.contains("css") { "css" }
                        else if first_line.contains("json") { "json" }
                        else if first_line.contains("yaml") || first_line.contains("yml") { "yml" }
                        else if first_line.contains("toml") { "toml" }
                        else if first_line.contains("sh") || first_line.contains("bash") { "sh" }
                        else if first_line.contains("sql") { "sql" }
                        else if first_line.contains("go") { "go" }
                        else if first_line.contains("docker") { "dockerfile" }
                        else { continue; };

                    let code: String = part.lines().skip(1).collect::<Vec<_>>().join("\n").trim().to_string();
                    if code.len() > 20 {
                        file_idx += 1;
                        let fname = format!("file_{}.{}", file_idx, ext);
                        let _ = storage::save_file(user_id, session_id, &fname, &code).await;
                    }
                }

                // Extract clean text content from Zen API non-streaming JSON response
                let content = if let Ok(json_val) = serde_json::from_str::<serde_json::Value>(&text) {
                    json_val["choices"][0]["message"]["content"]
                        .as_str()
                        .map(|s| s.to_string())
                        .or_else(|| json_val["choices"][0]["text"].as_str().map(|s| s.to_string()))
                        .or_else(|| json_val["response"].as_str().map(|s| s.to_string()))
                        .unwrap_or(text)
                } else {
                    text
                };

                // ── Enforce locks on the output ────────────────────────────────
                let locks_engine = StrictLocksEngine::new();
                let lock_result = locks_engine.check_all(
                    session_mode,
                    &model,
                    "text",
                    &content,
                );
                let final_content = if !lock_result.passed {
                    let has_critical = lock_result.violations.iter()
                        .any(|v| v.severity == crate::enforce::locks::ViolationSeverity::Critical);
                    if has_critical {
                        warn!("Lock violation (critical) in output for user {}", user_id);
                        "I am **Requiem Agent 1** — a multi-model AI development tool. My identity is fixed and cannot be changed.".to_string()
                    } else {
                        content
                    }
                } else {
                    content
                };

                // Return as SSE with proper JSON escaping via serde_json
                let escaped = serde_json::to_string(&final_content)
                    .unwrap_or_else(|_| String::from("\"\""));
                let sse = format!(
                    "data: {{\"choices\":[{{\"delta\":{{\"content\":{}}}}}]}}\n\ndata: [DONE]\n\n",
                    escaped
                );
                (
                    StatusCode::OK,
                    [
                        ("Content-Type", "text/event-stream; charset=utf-8"),
                        ("Cache-Control", "no-cache, no-store"),
                        ("X-Accel-Buffering", "no"),
                    ],
                    sse,
                ).into_response()
            }
        }
        Err(e) => {
            error!("Zen chat failed for user {}: {}", user_id, e);
            (StatusCode::BAD_GATEWAY, Json(ErrorResponse {
                error: format!("Zen API failed after retries: {e}"),
            })).into_response()
        }
    }
}

// ─── Proxy Health Check ─────────────────────────────────────────────────────

/// فحص صحة جميع الـ proxies (يُستدعى بشكل دوري)
pub async fn check_proxies_health() -> Vec<(usize, bool)> {
    let mut results = Vec::new();
    for (idx, (host, port, user, pass)) in PROXIES.iter().enumerate() {
        let proxy_url = format!("socks5://{user}:{pass}@{host}:{port}");
        let client = match reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .proxy(reqwest::Proxy::all(&proxy_url).unwrap())
            .build()
        {
            Ok(c) => c,
            Err(_) => {
                results.push((idx, false));
                continue;
            }
        };

        let ok = client
            .head("https://opencode.ai/zen/v1/models")
            .send()
            .await
            .is_ok();

        results.push((idx, ok));
        if !ok {
            warn!("Proxy {} health check FAILED: {host}:{port}", idx);
        }
    }
    results
}
