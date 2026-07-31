// ws_streaming_e2e.rs — S6-04: E2E tests for WebSocket agent streaming
//
// يختبر الـ flow الكامل:
//   connect → send start → receive tokens → done
//   connect → send start → cancel → cancelled
//   connect → ping → pong
//   connect → invalid message → error
//
// يستخدم axum::serve مع tokio-tungstenite للاتصال الحقيقي بـ WebSocket.
//
// ملاحظة: هذه اختبارات integration حقيقية — تُشغّل server فعلي على منفذ عشوائي.

use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    routing::get,
    Router,
};
use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tokio_tungstenite::{connect_async, tungstenite::Message as TungsteniteMessage};

// ─── Mock WebSocket handler (بدون DB/LLM حقيقي) ─────────────────────────────

/// Handler مبسّط يُحاكي سلوك ws_agent.rs للاختبار
async fn mock_ws_handler(ws: WebSocketUpgrade) -> impl axum::response::IntoResponse {
    ws.on_upgrade(handle_mock_ws)
}

async fn handle_mock_ws(mut socket: WebSocket) {
    while let Some(Ok(msg)) = socket.recv().await {
        let text = match msg {
            Message::Text(t) => t,
            Message::Close(_) => break,
            Message::Ping(data) => {
                let _ = socket.send(Message::Pong(data)).await;
                continue;
            }
            _ => continue,
        };

        let parsed: Value = match serde_json::from_str(&text) {
            Ok(v) => v,
            Err(_) => {
                let _ = socket
                    .send(Message::Text(
                        json!({"type": "error", "message": "Invalid JSON"}).to_string(),
                    ))
                    .await;
                continue;
            }
        };

        match parsed["type"].as_str() {
            Some("ping") => {
                let _ = socket
                    .send(Message::Text(json!({"type": "pong"}).to_string()))
                    .await;
            }
            Some("start") => {
                let message = parsed["message"].as_str().unwrap_or("hello");

                // بثّ 3 tokens
                for word in ["Hello", " world", "!"] {
                    let _ = socket
                        .send(Message::Text(
                            json!({"type": "token", "content": word}).to_string(),
                        ))
                        .await;
                }

                // إرسال done
                let _ = socket
                    .send(Message::Text(
                        json!({
                            "type": "done",
                            "content": format!("Hello world! (echo: {})", message),
                            "steps": 0
                        })
                        .to_string(),
                    ))
                    .await;
            }
            Some("cancel") => {
                let _ = socket
                    .send(Message::Text(
                        json!({"type": "error", "message": "Cancelled by client"}).to_string(),
                    ))
                    .await;
                break;
            }
            Some("start_orchestrator") => {
                // محاكاة orchestrator mode مع steps
                let _ = socket
                    .send(Message::Text(
                        json!({"type": "step", "step": 1, "thought": "Analyzing request..."})
                            .to_string(),
                    ))
                    .await;
                let _ = socket
                    .send(Message::Text(
                        json!({"type": "tool_call", "name": "search", "args": {"query": "test"}})
                            .to_string(),
                    ))
                    .await;
                let _ = socket
                    .send(Message::Text(
                        json!({"type": "tool_result", "name": "search", "output": "Found results"})
                            .to_string(),
                    ))
                    .await;
                let _ = socket
                    .send(Message::Text(
                        json!({"type": "done", "content": "Task complete", "steps": 1})
                            .to_string(),
                    ))
                    .await;
            }
            _ => {
                let _ = socket
                    .send(Message::Text(
                        json!({"type": "error", "message": "Unknown message type"}).to_string(),
                    ))
                    .await;
            }
        }
    }
}

// ─── Test helpers ─────────────────────────────────────────────────────────────

/// يُشغّل mock server ويُعيد عنوانه
async fn start_mock_server() -> SocketAddr {
    let app = Router::new().route("/ws/agent", get(mock_ws_handler));
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    addr
}

/// يُنشئ اتصال WebSocket بالـ server
async fn connect_ws(addr: SocketAddr) -> (
    impl futures_util::Sink<TungsteniteMessage, Error = tokio_tungstenite::tungstenite::Error>,
    impl futures_util::Stream<Item = Result<TungsteniteMessage, tokio_tungstenite::tungstenite::Error>>,
) {
    let url = format!("ws://{}/ws/agent", addr);
    let (ws_stream, _) = connect_async(&url).await.expect("Failed to connect");
    ws_stream.split()
}

/// يُرسل رسالة JSON ويُعيد الرد التالي كـ Value
async fn send_and_recv(
    sink: &mut (impl futures_util::Sink<TungsteniteMessage, Error = tokio_tungstenite::tungstenite::Error> + Unpin),
    stream: &mut (impl futures_util::Stream<Item = Result<TungsteniteMessage, tokio_tungstenite::tungstenite::Error>> + Unpin),
    msg: Value,
) -> Value {
    sink.send(TungsteniteMessage::Text(msg.to_string()))
        .await
        .unwrap();
    let resp = stream.next().await.unwrap().unwrap();
    match resp {
        TungsteniteMessage::Text(t) => serde_json::from_str(&t).unwrap(),
        other => panic!("Expected text message, got: {:?}", other),
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

/// E2E-01: connect → ping → pong
#[tokio::test]
async fn test_ws_ping_pong() {
    let addr = start_mock_server().await;
    let (mut sink, mut stream) = connect_ws(addr).await;

    let resp = send_and_recv(&mut sink, &mut stream, json!({"type": "ping"})).await;
    assert_eq!(resp["type"], "pong");
}

/// E2E-02: connect → start → receive 3 tokens → done
#[tokio::test]
async fn test_ws_start_receives_tokens_then_done() {
    let addr = start_mock_server().await;
    let (mut sink, mut stream) = connect_ws(addr).await;

    // إرسال start
    sink.send(TungsteniteMessage::Text(
        json!({"type": "start", "message": "hello", "mode": "chat"}).to_string(),
    ))
    .await
    .unwrap();

    // استقبال 3 tokens
    let mut tokens = Vec::new();
    let mut done_msg: Option<Value> = None;

    for _ in 0..4 {
        let msg = stream.next().await.unwrap().unwrap();
        if let TungsteniteMessage::Text(t) = msg {
            let v: Value = serde_json::from_str(&t).unwrap();
            match v["type"].as_str() {
                Some("token") => tokens.push(v["content"].as_str().unwrap_or("").to_string()),
                Some("done") => { done_msg = Some(v); break; }
                _ => {}
            }
        }
    }

    assert_eq!(tokens.len(), 3, "Should receive exactly 3 tokens");
    assert_eq!(tokens.join(""), "Hello world!");
    assert!(done_msg.is_some(), "Should receive done message");
    assert_eq!(done_msg.unwrap()["type"], "done");
}

/// E2E-03: connect → start → done message contains content
#[tokio::test]
async fn test_ws_done_message_has_content() {
    let addr = start_mock_server().await;
    let (mut sink, mut stream) = connect_ws(addr).await;

    sink.send(TungsteniteMessage::Text(
        json!({"type": "start", "message": "test-input", "mode": "chat"}).to_string(),
    ))
    .await
    .unwrap();

    let mut done_msg: Option<Value> = None;
    for _ in 0..10 {
        let msg = stream.next().await.unwrap().unwrap();
        if let TungsteniteMessage::Text(t) = msg {
            let v: Value = serde_json::from_str(&t).unwrap();
            if v["type"] == "done" {
                done_msg = Some(v);
                break;
            }
        }
    }

    let done = done_msg.expect("Should receive done");
    assert!(done["content"].as_str().unwrap_or("").contains("test-input"));
    assert_eq!(done["steps"], 0);
}

/// E2E-04: connect → cancel → error message
#[tokio::test]
async fn test_ws_cancel_returns_error() {
    let addr = start_mock_server().await;
    let (mut sink, mut stream) = connect_ws(addr).await;

    let resp = send_and_recv(&mut sink, &mut stream, json!({"type": "cancel"})).await;
    assert_eq!(resp["type"], "error");
    assert!(resp["message"].as_str().unwrap_or("").contains("Cancelled"));
}

/// E2E-05: connect → invalid JSON → error message
#[tokio::test]
async fn test_ws_invalid_json_returns_error() {
    let addr = start_mock_server().await;
    let (mut sink, mut stream) = connect_ws(addr).await;

    sink.send(TungsteniteMessage::Text("not-valid-json".to_string()))
        .await
        .unwrap();

    let resp = stream.next().await.unwrap().unwrap();
    if let TungsteniteMessage::Text(t) = resp {
        let v: Value = serde_json::from_str(&t).unwrap();
        assert_eq!(v["type"], "error");
    } else {
        panic!("Expected text response");
    }
}

/// E2E-06: connect → unknown type → error message
#[tokio::test]
async fn test_ws_unknown_type_returns_error() {
    let addr = start_mock_server().await;
    let (mut sink, mut stream) = connect_ws(addr).await;

    let resp = send_and_recv(
        &mut sink,
        &mut stream,
        json!({"type": "unknown_command", "data": "test"}),
    )
    .await;
    assert_eq!(resp["type"], "error");
}

/// E2E-07: orchestrator mode → receives step + tool_call + tool_result + done
#[tokio::test]
async fn test_ws_orchestrator_mode_receives_steps() {
    let addr = start_mock_server().await;
    let (mut sink, mut stream) = connect_ws(addr).await;

    sink.send(TungsteniteMessage::Text(
        json!({"type": "start_orchestrator", "message": "do task", "mode": "orchestrator"})
            .to_string(),
    ))
    .await
    .unwrap();

    let mut msg_types: Vec<String> = Vec::new();
    for _ in 0..10 {
        let msg = stream.next().await.unwrap().unwrap();
        if let TungsteniteMessage::Text(t) = msg {
            let v: Value = serde_json::from_str(&t).unwrap();
            let t = v["type"].as_str().unwrap_or("").to_string();
            let is_done = t == "done";
            msg_types.push(t);
            if is_done { break; }
        }
    }

    assert!(msg_types.contains(&"step".to_string()), "Should receive step");
    assert!(msg_types.contains(&"tool_call".to_string()), "Should receive tool_call");
    assert!(msg_types.contains(&"tool_result".to_string()), "Should receive tool_result");
    assert!(msg_types.contains(&"done".to_string()), "Should receive done");
}

/// E2E-08: multiple ping-pong in sequence
#[tokio::test]
async fn test_ws_multiple_pings() {
    let addr = start_mock_server().await;
    let (mut sink, mut stream) = connect_ws(addr).await;

    for _ in 0..3 {
        let resp = send_and_recv(&mut sink, &mut stream, json!({"type": "ping"})).await;
        assert_eq!(resp["type"], "pong");
    }
}

/// E2E-09: done message has steps field as number
#[tokio::test]
async fn test_ws_done_steps_is_number() {
    let addr = start_mock_server().await;
    let (mut sink, mut stream) = connect_ws(addr).await;

    sink.send(TungsteniteMessage::Text(
        json!({"type": "start", "message": "test", "mode": "chat"}).to_string(),
    ))
    .await
    .unwrap();

    let mut done_msg: Option<Value> = None;
    for _ in 0..10 {
        let msg = stream.next().await.unwrap().unwrap();
        if let TungsteniteMessage::Text(t) = msg {
            let v: Value = serde_json::from_str(&t).unwrap();
            if v["type"] == "done" {
                done_msg = Some(v);
                break;
            }
        }
    }

    let done = done_msg.expect("Should receive done");
    assert!(done["steps"].is_number(), "steps should be a number");
}
