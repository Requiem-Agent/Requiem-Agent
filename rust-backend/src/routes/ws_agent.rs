// ws_agent.rs — WebSocket handler for real-time agent streaming
// S4-03: Replaces/complements SSE with a full-duplex WebSocket channel
//
// Protocol (JSON messages):
//   Client → Server:
//     { "type": "start",   "message": "...", "workspace_id": "...", "mode": "chat|orchestrator" }
//     { "type": "cancel" }
//     { "type": "ping" }
//
//   Server → Client:
//     { "type": "token",    "content": "..." }          ← streaming token
//     { "type": "step",     "step": N, "thought": "..." } ← ReAct step
//     { "type": "tool_call","name": "...", "args": {...} }
//     { "type": "tool_result","name": "...", "output": "..." }
//     { "type": "done",     "content": "...", "steps": N }
//     { "type": "error",    "message": "..." }
//     { "type": "pong" }

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::Response,
};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};
use std::sync::atomic::{AtomicBool, Ordering};

use crate::{
    error::AppError,
    llm_stream::{stream_anthropic_to_ws, LlmStreamConfig},
    metrics::{record_agent_step, record_llm_call},
    react_loop::{default_requiem_tools, ReActEngine, StopReason},
};

// ─────────────────────────────────────────────
// Message types
// ─────────────────────────────────────────────

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ClientMessage {
    Start {
        message: String,
        #[serde(default)]
        workspace_id: Option<String>,
        #[serde(default = "default_mode")]
        mode: String,
        #[serde(default = "default_max_steps")]
        max_steps: usize,
    },
    Cancel,
    Ping,
}

fn default_mode() -> String { "chat".into() }
fn default_max_steps() -> usize { 10 }

#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMessage {
    Token { content: String },
    Step { step: usize, thought: String },
    ToolCall { name: String, args: serde_json::Value },
    ToolResult { name: String, output: String },
    Done { content: String, steps: usize },
    Error { message: String },
    Pong,
}

impl ServerMessage {
    fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| r#"{"type":"error","message":"serialization failed"}"#.into())
    }
}

// ─────────────────────────────────────────────
// AppState trait (same pattern as rate_limit.rs)
// ─────────────────────────────────────────────

pub trait HasWsConfig {
    fn ws_max_message_size(&self) -> usize { 64 * 1024 } // 64 KB default
    fn ws_timeout_secs(&self) -> u64 { 300 }             // 5 min default
}

// ─────────────────────────────────────────────
// Upgrade handler — registered as GET /ws/agent
// ─────────────────────────────────────────────

/// Axum handler: upgrades HTTP → WebSocket and spawns the session task.
///
/// Route registration example:
/// ```rust
/// router.route("/ws/agent", get(ws_agent::ws_handler::<AppState>))
/// ```
pub async fn ws_handler<S>(
    ws: WebSocketUpgrade,
    State(state): State<Arc<S>>,
) -> Response
where
    S: HasWsConfig + Send + Sync + 'static,
{
    let max_size = state.ws_max_message_size();
    ws.max_message_size(max_size)
        .on_upgrade(move |socket| handle_socket(socket, state))
}

// ─────────────────────────────────────────────
// Session handler
// ─────────────────────────────────────────────

async fn handle_socket<S>(socket: WebSocket, state: Arc<S>)
where
    S: HasWsConfig + Send + Sync + 'static,
{
    let timeout_secs = state.ws_timeout_secs();
    let (mut sender, mut receiver) = socket.split();

    // Channel for sending server messages from the agent task back to the WS sender
    let (tx, mut rx) = mpsc::channel::<ServerMessage>(64);

    // Spawn a task that forwards ServerMessages → WebSocket frames
    let send_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            let json = msg.to_json();
            if sender.send(Message::Text(json.into())).await.is_err() {
                break; // client disconnected
            }
        }
        // Graceful close
        let _ = sender.close().await;
    });

    // Cancellation flag
    let cancelled = Arc::new(AtomicBool::new(false));

    // Session loop — wait for a "start" message, then run the agent
    let session_timeout = tokio::time::Duration::from_secs(timeout_secs);

    let result = tokio::time::timeout(session_timeout, async {
        while let Some(raw) = receiver.next().await {
            let raw = match raw {
                Ok(r) => r,
                Err(e) => {
                    warn!("WS receive error: {}", e);
                    break;
                }
            };

            let text = match raw {
                Message::Text(t) => t,
                Message::Close(_) => break,
                Message::Ping(data) => {
                    // Axum handles Pong automatically, but we log it
                    debug!("WS ping received ({} bytes)", data.len());
                    continue;
                }
                _ => continue,
            };

            let client_msg: ClientMessage = match serde_json::from_str(&text) {
                Ok(m) => m,
                Err(e) => {
                    let _ = tx.send(ServerMessage::Error {
                        message: format!("Invalid message: {}", e),
                    }).await;
                    continue;
                }
            };

            match client_msg {
                ClientMessage::Ping => {
                    let _ = tx.send(ServerMessage::Pong).await;
                }

                ClientMessage::Cancel => {
                    cancelled.store(true, Ordering::Relaxed);
                    info!("WS agent session cancelled by client");
                    break;
                }

                ClientMessage::Start { message, workspace_id, mode, max_steps } => {
                    info!(
                        mode = %mode,
                        workspace_id = ?workspace_id,
                        max_steps,
                        "WS agent session started"
                    );

                    let tx2 = tx.clone();
                    let cancelled2 = cancelled.clone();

                    // Run the agent in a separate task so we can still receive Cancel
                    let agent_task = tokio::spawn(async move {
                        run_agent_session(
                            message,
                            workspace_id,
                            mode,
                            max_steps,
                            tx2,
                            cancelled2,
                        ).await;
                    });

                    // Wait for agent to finish (or cancellation)
                    let _ = agent_task.await;
                    // After one "start" session, wait for the next or close
                }
            }
        }
    }).await;

    if result.is_err() {
        warn!("WS session timed out after {} seconds", timeout_secs);
        let _ = tx.send(ServerMessage::Error {
            message: "Session timed out".into(),
        }).await;
    }

    // Drop tx so the send_task exits
    drop(tx);
    let _ = send_task.await;
}

// ─────────────────────────────────────────────
// Agent execution within a WS session
// ─────────────────────────────────────────────

async fn run_agent_session(
    user_message: String,
    _workspace_id: Option<String>,
    mode: String,
    max_steps: usize,
    tx: mpsc::Sender<ServerMessage>,
    cancelled: Arc<AtomicBool>,
) {
    if mode == "orchestrator" {
        run_react_session(user_message, max_steps, tx, cancelled).await;
    } else {
        run_chat_session(user_message, tx, cancelled).await;
    }
}

/// ReAct orchestrator mode — streams each step over WebSocket
async fn run_react_session(
    user_message: String,
    max_steps: usize,
    tx: mpsc::Sender<ServerMessage>,
    cancelled: Arc<AtomicBool>,
) {
    let tools = default_requiem_tools();
    let engine = ReActEngine::with_config(tools, max_steps, 45);

    // We run the engine and stream steps as they complete.
    // Since ReActEngine::run() is not yet streaming-native, we wrap it
    // and emit a "step" event after each iteration via a callback channel.
    //
    // Future improvement: refactor ReActEngine to accept a step_callback: Fn(Step) -> Future
    // For now we run it to completion and stream the steps from the result.

    record_llm_call("react", true);

    let result = engine.run(
        &user_message,
        None,
        |tool_name, args| {
            // Emit tool_call event (best-effort; ignore send errors)
            let tx_inner = tx.clone();
            let name = tool_name.clone();
            // args is already a serde_json::Value from the engine
            let args_val = args.clone();
            tokio::spawn(async move {
                let _ = tx_inner.send(ServerMessage::ToolCall {
                    name,
                    args: args_val,
                }).await;
            });

            // Return a ToolResult (real impl would call workspace tools)
            let tool_name_owned = tool_name.clone();
            async move {
                crate::react_loop::ToolResult {
                    tool_name: tool_name_owned,
                    success: true,
                    output: Some(serde_json::json!({
                        "result": format!("Tool '{}' executed", tool_name)
                    })),
                    error: None,
                    duration_ms: 0,
                }
            }
        },
    ).await;

    if cancelled.load(Ordering::Relaxed) {
        let _ = tx.send(ServerMessage::Error { message: "Cancelled".into() }).await;
        return;
    }

    // Stream steps from result
    for (i, step) in result.steps.iter().enumerate() {
        record_agent_step();
        // ReActStep uses `content` field (not `thought`)
        let _ = tx.send(ServerMessage::Step {
            step: i + 1,
            thought: step.content.clone(),
        }).await;

        if let Some(ref tool_name) = step.tool_name {
            // tool_input is the observation/output for tool steps
            let output = step.tool_input
                .as_ref()
                .map(|v| v.to_string())
                .unwrap_or_default();
            let _ = tx.send(ServerMessage::ToolResult {
                name: tool_name.clone(),
                output,
            }).await;
        }
    }

    match result.stop_reason {
        StopReason::Completed => {
            let content = result.final_answer.unwrap_or_default();
            let _ = tx.send(ServerMessage::Done {
                content,
                steps: result.steps.len(),
            }).await;
        }
        reason => {
            let _ = tx.send(ServerMessage::Error {
                message: format!("ReAct stopped: {:?}", reason),
            }).await;
        }
    }
}

/// Simple chat mode — streams real Anthropic SSE tokens over WebSocket.
/// Falls back to echo mode if ANTHROPIC_API_KEY is not set.
async fn run_chat_session(
    user_message: String,
    tx: mpsc::Sender<ServerMessage>,
    cancelled: Arc<AtomicBool>,
) {
    record_llm_call("chat", true);

    let config = LlmStreamConfig::default();

    if config.api_key.is_empty() {
        // Fallback: echo mode (no API key configured)
        let response = format!("Echo (no API key): {}", user_message);
        let words: Vec<&str> = response.split_whitespace().collect();
        for word in &words {
            if cancelled.load(Ordering::Relaxed) {
                let _ = tx.send(ServerMessage::Error { message: "Cancelled".into() }).await;
                return;
            }
            let _ = tx.send(ServerMessage::Token {
                content: format!("{} ", word),
            }).await;
            tokio::time::sleep(tokio::time::Duration::from_millis(30)).await;
        }
        let _ = tx.send(ServerMessage::Done {
            content: response,
            steps: 0,
        }).await;
        return;
    }

    // Real Anthropic SSE → WebSocket bridge
    match stream_anthropic_to_ws(&user_message, &config, &tx, &cancelled).await {
        Ok(tokens) => {
            tracing::info!(output_tokens = tokens, "Anthropic stream completed");
        }
        Err(e) => {
            tracing::error!(error = %e, "Anthropic stream failed");
            // Error already sent to WS by stream_anthropic_to_ws
        }
    }
}

// ─────────────────────────────────────────────
// Unit tests (logic only — no actual WS connection)
// ─────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_message_serialization() {
        let msg = ServerMessage::Token { content: "hello".into() };
        let json = msg.to_json();
        assert!(json.contains("\"type\":\"token\""));
        assert!(json.contains("\"content\":\"hello\""));
    }

    #[test]
    fn test_done_message_serialization() {
        let msg = ServerMessage::Done { content: "result".into(), steps: 3 };
        let json = msg.to_json();
        assert!(json.contains("\"type\":\"done\""));
        assert!(json.contains("\"steps\":3"));
    }

    #[test]
    fn test_error_message_serialization() {
        let msg = ServerMessage::Error { message: "oops".into() };
        let json = msg.to_json();
        assert!(json.contains("\"type\":\"error\""));
    }

    #[test]
    fn test_client_message_deserialization_start() {
        let json = r#"{"type":"start","message":"hello","mode":"chat","max_steps":5}"#;
        let msg: ClientMessage = serde_json::from_str(json).unwrap();
        match msg {
            ClientMessage::Start { message, mode, max_steps, .. } => {
                assert_eq!(message, "hello");
                assert_eq!(mode, "chat");
                assert_eq!(max_steps, 5);
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn test_client_message_deserialization_cancel() {
        let json = r#"{"type":"cancel"}"#;
        let msg: ClientMessage = serde_json::from_str(json).unwrap();
        assert!(matches!(msg, ClientMessage::Cancel));
    }

    #[test]
    fn test_client_message_deserialization_ping() {
        let json = r#"{"type":"ping"}"#;
        let msg: ClientMessage = serde_json::from_str(json).unwrap();
        assert!(matches!(msg, ClientMessage::Ping));
    }

    #[test]
    fn test_default_mode() {
        assert_eq!(default_mode(), "chat");
    }

    #[test]
    fn test_default_max_steps() {
        assert_eq!(default_max_steps(), 10);
    }
}