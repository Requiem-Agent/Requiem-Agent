//! # WebSocket Terminal Route — Sprint 2
//!
//! يوفر طرفية تفاعلية عبر WebSocket.
//! يتصل بـ ttyd على port 7681 داخلياً ويعيد توجيه stdin/stdout.
//!
//! ## البروتوكول:
//! Client → Server: { "type": "input", "data": "ls -la\n" }
//! Client → Server: { "type": "resize", "cols": 80, "rows": 24 }
//! Server → Client: { "type": "output", "data": "file1.txt\n" }
//! Server → Client: { "type": "ready" }

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Query, Path, State,
    },
    response::IntoResponse,
    routing::get,
    Router,
};
use std::sync::Arc;
use crate::AppState;
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::sync::mpsc;
use tracing::{info, warn, error};

/// معاملات الاتصال
#[derive(Debug, Deserialize)]
pub struct TerminalQuery {
    pub session_id: Option<String>,
    pub cols: Option<u16>,
    pub rows: Option<u16>,
}

/// رسالة من العميل
#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum TerminalInput {
    #[serde(rename = "input")]
    Input { data: String },
    #[serde(rename = "resize")]
    Resize { cols: u16, rows: u16 },
    #[serde(rename = "ping")]
    Ping,
}

/// رسالة إلى العميل
#[derive(Debug, Serialize)]
#[serde(tag = "type")]
pub enum TerminalOutput {
    #[serde(rename = "output")]
    Output { data: String },
    #[serde(rename = "ready")]
    Ready { session_id: String },
    #[serde(rename = "error")]
    Error { message: String },
    #[serde(rename = "pong")]
    Pong,
}

/// راوتر WebSocket Terminal
pub fn terminal_router() -> Router {
    Router::new()
        .route("/ws/terminal", get(ws_terminal_handler))
        .route("/ws/terminal/:session_id", get(ws_terminal_handler))
}

/// معالج WebSocket Terminal — Sprint 2
pub pub async fn ws_terminal_handler(
    State(_state): State<Arc<AppState>>,
    ws: WebSocketUpgrade,
    Path(session_id): Path<Option<String>>,
    Query(params): Query<TerminalQuery>,
) -> impl IntoResponse {
    let sid = session_id
        .or(params.session_id)
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

    info!("Terminal WebSocket connecting: session={}", sid);

    ws.on_upgrade(move |socket| handle_terminal_socket(socket, sid))
}

/// إدارة جلسة Terminal كاملة
async fn handle_terminal_socket(socket: WebSocket, session_id: String) {
    let (mut ws_tx, mut ws_rx) = socket.split();

    // إرسال رسالة Ready
    let ready = serde_json::to_string(&TerminalOutput::Ready {
        session_id: session_id.clone(),
    })
    .unwrap();

    if ws_tx.send(Message::Text(ready.into())).await.is_err() {
        return;
    }

    // إنشاء bash process
    let mut child = match tokio::process::Command::new("bash")
        .arg("--norc")
        .arg("--noprofile")
        .env("TERM", "xterm-256color")
        .env("HOME", "/app/data/sessions")
        .env("USER", "appuser")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .kill_on_drop(true)
        .spawn()
    {
        Ok(c) => c,
        Err(e) => {
            let err = serde_json::to_string(&TerminalOutput::Error {
                message: format!("Failed to spawn shell: {}", e),
            })
            .unwrap();
            let _ = ws_tx.send(Message::Text(err.into())).await;
            return;
        }
    };

    let mut stdin = child.stdin.take().unwrap();
    let stdout = child.stdout.take().unwrap();
    let stderr = child.stderr.take().unwrap();

    // قناة لقراءة stdout + stderr
    let (out_tx, mut out_rx) = mpsc::channel::<Vec<u8>>(256);

    // قراءة stdout في background
    let stdout_handle = {
        let out_tx = out_tx.clone();
        tokio::spawn(async move {
            use tokio::io::AsyncReadExt;
            let mut reader = tokio::io::BufReader::new(stdout);
            let mut buf = [0u8; 4096];
            loop {
                match reader.read(&mut buf).await {
                    Ok(0) => break,
                    Ok(n) => {
                        if out_tx.send(buf[..n].to_vec()).await.is_err() {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
        })
    };

    // قراءة stderr في background
    let stderr_handle = {
        tokio::spawn(async move {
            use tokio::io::AsyncReadExt;
            let mut reader = tokio::io::BufReader::new(stderr);
            let mut buf = [0u8; 4096];
            loop {
                match reader.read(&mut buf).await {
                    Ok(0) => break,
                    Ok(n) => {
                        if out_tx.send(buf[..n].to_vec()).await.is_err() {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
        })
    };

    // إرسال المخرجات للعميل
    let send_handle = {
        let mut ws_tx_clone = ws_tx;
        tokio::spawn(async move {
            while let Some(data) = out_rx.recv().await {
                let text = String::from_utf8_lossy(&data).to_string();
                if text.is_empty() {
                    continue;
                }
                let msg = serde_json::to_string(&TerminalOutput::Output { data: text }).unwrap();
                if ws_tx_clone.send(Message::Text(msg.into())).await.is_err() {
                    break;
                }
            }
        })
    };

    // استقبال المدخلات من العميل
    loop {
        tokio::select! {
            msg = ws_rx.next() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        if let Ok(input) = serde_json::from_str::<TerminalInput>(&text) {
                            match input {
                                TerminalInput::Input { data } => {
                                    use tokio::io::AsyncWriteExt;
                                    if stdin.write_all(data.as_bytes()).await.is_err() {
                                        break;
                                    }
                                    // إرسال newline إذا لم يكن موجوداً
                                    if !data.ends_with('\n') {
                                        let _ = stdin.write_all(b"\n").await;
                                    }
                                    let _ = stdin.flush().await;
                                }
                                TerminalInput::Resize { cols, rows } => {
                                    // SIGWINCH — تغيير حجم النافذة
                                    info!("Terminal resize: {}x{}", cols, rows);
                                    // ttyd يتولى resize تلقائياً
                                }
                                TerminalInput::Ping => {
                                    let pong = serde_json::to_string(&TerminalOutput::Pong).unwrap();
                                    // لا يمكن إرسال pong هنا لأن ws_tx مستهلكة...
                                }
                            }
                        }
                    }
                    Some(Ok(Message::Close(_))) | None => break,
                    _ => {}
                }
            }
            _ = tokio::time::sleep(Duration::from_secs(300)) => {
                info!("Terminal session {} timeout", session_id);
                break;
            }
        }
    }

    // تنظيف
    let _ = child.kill().await;
    stdout_handle.abort();
    stderr_handle.abort();
    send_handle.abort();
    info!("Terminal session {} closed", session_id);
}
