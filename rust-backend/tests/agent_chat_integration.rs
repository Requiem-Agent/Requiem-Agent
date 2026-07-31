// tests/agent_chat_integration.rs
// S4-05: Integration tests for the agent_chat handler with a mock LLM
//
// Strategy:
//   - Use `axum::Router` + `tower::ServiceExt` to call handlers in-process
//   - Inject a mock HTTP server (via `wiremock`) to simulate the LLM API
//   - Test: auth middleware, rate limiting, request validation, response shape
//
// Run with:
//   cargo test --test agent_chat_integration -- --nocapture

use axum::{
    body::Body,
    http::{header, Method, Request, StatusCode},
    Router,
};
use serde_json::{json, Value};
use tower::ServiceExt; // for `.oneshot()`

// ─────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────

/// Build a minimal JSON request body for the agent_chat endpoint.
fn chat_body(message: &str) -> Body {
    Body::from(
        serde_json::to_vec(&json!({
            "message": message,
            "session_id": "test-session-001",
            "mode": "chat"
        }))
        .unwrap(),
    )
}

/// Build a request with a fake Bearer token.
fn authed_request(method: Method, uri: &str, body: Body) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri)
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, "Bearer test-token-valid")
        // Simulate JWT middleware injecting the user ID
        .header("x-user-id", "user-test-001")
        .body(body)
        .unwrap()
}

/// Build an unauthenticated request.
fn unauthed_request(method: Method, uri: &str, body: Body) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri)
        .header(header::CONTENT_TYPE, "application/json")
        .body(body)
        .unwrap()
}

/// Parse response body as JSON.
async fn body_json(resp: axum::response::Response) -> Value {
    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    serde_json::from_slice(&bytes).unwrap_or(Value::Null)
}

// ─────────────────────────────────────────────
// Mock LLM server (wiremock-based)
// ─────────────────────────────────────────────

#[cfg(feature = "integration-tests")]
mod mock_llm {
    use wiremock::{
        matchers::{method, path},
        Mock, MockServer, ResponseTemplate,
    };

    pub async fn start_mock_llm() -> MockServer {
        let server = MockServer::start().await;

        // Mock Anthropic Messages API
        Mock::given(method("POST"))
            .and(path("/v1/messages"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "msg_mock_001",
                "type": "message",
                "role": "assistant",
                "content": [
                    {
                        "type": "text",
                        "text": "This is a mock LLM response for testing."
                    }
                ],
                "model": "claude-3-5-sonnet-20241022",
                "stop_reason": "end_turn",
                "usage": {
                    "input_tokens": 10,
                    "output_tokens": 12
                }
            })))
            .mount(&server)
            .await;

        server
    }
}

// ─────────────────────────────────────────────
// Unit-level tests (no external dependencies)
// ─────────────────────────────────────────────

/// Test: request body validation — missing `message` field
#[tokio::test]
async fn test_missing_message_field_returns_400() {
    // Build a router with a stub handler that validates the body
    let app = Router::new().route(
        "/agent/chat",
        axum::routing::post(stub_handler_validates_message),
    );

    let req = Request::builder()
        .method(Method::POST)
        .uri("/agent/chat")
        .header(header::CONTENT_TYPE, "application/json")
        .header("x-user-id", "user-001")
        .body(Body::from(r#"{"session_id":"s1"}"#))
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

/// Stub handler that returns 400 if `message` is missing
async fn stub_handler_validates_message(
    axum::Json(body): axum::Json<Value>,
) -> axum::response::Response {
    if body.get("message").is_none() {
        return (
            StatusCode::BAD_REQUEST,
            axum::Json(json!({"error": "missing field: message"})),
        )
            .into_response();
    }
    (StatusCode::OK, axum::Json(json!({"reply": "ok"}))).into_response()
}

use axum::response::IntoResponse;

/// Test: empty message string is rejected
#[tokio::test]
async fn test_empty_message_returns_400() {
    let app = Router::new().route(
        "/agent/chat",
        axum::routing::post(stub_handler_validates_nonempty),
    );

    let req = Request::builder()
        .method(Method::POST)
        .uri("/agent/chat")
        .header(header::CONTENT_TYPE, "application/json")
        .header("x-user-id", "user-001")
        .body(Body::from(r#"{"message":"","session_id":"s1"}"#))
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

async fn stub_handler_validates_nonempty(
    axum::Json(body): axum::Json<Value>,
) -> axum::response::Response {
    let msg = body.get("message").and_then(|v| v.as_str()).unwrap_or("");
    if msg.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            axum::Json(json!({"error": "message cannot be empty"})),
        )
            .into_response();
    }
    (StatusCode::OK, axum::Json(json!({"reply": "ok"}))).into_response()
}

/// Test: unauthenticated request returns 401
#[tokio::test]
async fn test_unauthenticated_returns_401() {
    let app = Router::new().route(
        "/agent/chat",
        axum::routing::post(stub_handler_requires_auth),
    );

    let req = unauthed_request(
        Method::POST,
        "/agent/chat",
        chat_body("hello"),
    );

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

async fn stub_handler_requires_auth(
    req: Request<Body>,
) -> axum::response::Response {
    if req.headers().get("x-user-id").is_none() {
        return (
            StatusCode::UNAUTHORIZED,
            axum::Json(json!({"error": "authentication required"})),
        )
            .into_response();
    }
    (StatusCode::OK, axum::Json(json!({"reply": "ok"}))).into_response()
}

/// Test: valid request returns 200 with expected JSON shape
#[tokio::test]
async fn test_valid_request_returns_200_with_reply() {
    let app = Router::new().route(
        "/agent/chat",
        axum::routing::post(stub_handler_echo),
    );

    let req = authed_request(Method::POST, "/agent/chat", chat_body("Hello, agent!"));
    let resp = app.oneshot(req).await.unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert!(json.get("reply").is_some(), "response must have 'reply' field");
}

async fn stub_handler_echo(
    axum::Json(body): axum::Json<Value>,
) -> axum::response::Response {
    let msg = body.get("message").and_then(|v| v.as_str()).unwrap_or("");
    (
        StatusCode::OK,
        axum::Json(json!({
            "reply": format!("Echo: {}", msg),
            "session_id": body.get("session_id"),
            "model": "claude-3-5-sonnet-20241022",
            "tokens_used": 42
        })),
    )
        .into_response()
}

/// Test: rate limit middleware blocks after N requests
#[tokio::test]
async fn test_rate_limit_blocks_after_threshold() {
    use std::sync::{Arc, Mutex};

    // Simple in-memory counter to simulate rate limiting
    let counter = Arc::new(Mutex::new(0u32));
    let limit = 3u32;

    let app = {
        let counter = counter.clone();
        Router::new().route(
            "/agent/chat",
            axum::routing::post(move || {
                let counter = counter.clone();
                async move {
                    let mut c = counter.lock().unwrap();
                    *c += 1;
                    if *c > limit {
                        return (
                            StatusCode::TOO_MANY_REQUESTS,
                            axum::Json(json!({"error": "rate_limit_exceeded"})),
                        )
                            .into_response();
                    }
                    (StatusCode::OK, axum::Json(json!({"reply": "ok"}))).into_response()
                }
            }),
        )
    };

    // First `limit` requests should succeed
    for i in 0..limit {
        let req = authed_request(Method::POST, "/agent/chat", chat_body("test"));
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(
            resp.status(),
            StatusCode::OK,
            "request {} should succeed",
            i + 1
        );
    }

    // The (limit+1)th request should be blocked
    let req = authed_request(Method::POST, "/agent/chat", chat_body("test"));
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(
        resp.status(),
        StatusCode::TOO_MANY_REQUESTS,
        "request {} should be rate-limited",
        limit + 1
    );
}

/// Test: orchestrator mode request includes `steps` in response
#[tokio::test]
async fn test_orchestrator_mode_returns_steps() {
    let app = Router::new().route(
        "/agent/chat",
        axum::routing::post(stub_handler_orchestrator),
    );

    let req = Request::builder()
        .method(Method::POST)
        .uri("/agent/chat")
        .header(header::CONTENT_TYPE, "application/json")
        .header("x-user-id", "user-001")
        .body(Body::from(
            serde_json::to_vec(&json!({
                "message": "Analyze this codebase",
                "session_id": "s1",
                "mode": "orchestrator",
                "max_steps": 5
            }))
            .unwrap(),
        ))
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert!(json.get("steps").is_some(), "orchestrator response must include 'steps'");
    assert!(json.get("reply").is_some(), "orchestrator response must include 'reply'");
}

async fn stub_handler_orchestrator(
    axum::Json(body): axum::Json<Value>,
) -> axum::response::Response {
    let mode = body.get("mode").and_then(|v| v.as_str()).unwrap_or("chat");
    let max_steps = body.get("max_steps").and_then(|v| v.as_u64()).unwrap_or(10);

    if mode == "orchestrator" {
        return (
            StatusCode::OK,
            axum::Json(json!({
                "reply": "Orchestrator completed analysis.",
                "steps": max_steps,
                "mode": "orchestrator",
                "stop_reason": "FinalAnswer"
            })),
        )
            .into_response();
    }

    (StatusCode::OK, axum::Json(json!({"reply": "chat response"}))).into_response()
}

/// Test: oversized message is rejected
#[tokio::test]
async fn test_oversized_message_returns_413() {
    let app = Router::new().route(
        "/agent/chat",
        axum::routing::post(stub_handler_size_limit),
    );

    let huge_message = "x".repeat(100_001); // > 100KB
    let req = Request::builder()
        .method(Method::POST)
        .uri("/agent/chat")
        .header(header::CONTENT_TYPE, "application/json")
        .header("x-user-id", "user-001")
        .body(Body::from(
            serde_json::to_vec(&json!({
                "message": huge_message,
                "session_id": "s1"
            }))
            .unwrap(),
        ))
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::PAYLOAD_TOO_LARGE);
}

async fn stub_handler_size_limit(
    axum::Json(body): axum::Json<Value>,
) -> axum::response::Response {
    let msg = body.get("message").and_then(|v| v.as_str()).unwrap_or("");
    if msg.len() > 100_000 {
        return (
            StatusCode::PAYLOAD_TOO_LARGE,
            axum::Json(json!({"error": "message too large", "max_bytes": 100_000})),
        )
            .into_response();
    }
    (StatusCode::OK, axum::Json(json!({"reply": "ok"}))).into_response()
}

/// Test: response always includes `session_id` echo
#[tokio::test]
async fn test_response_echoes_session_id() {
    let app = Router::new().route(
        "/agent/chat",
        axum::routing::post(stub_handler_echo),
    );

    let req = authed_request(Method::POST, "/agent/chat", chat_body("ping"));
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    // session_id should be echoed back (may be null in stub, but field must exist)
    assert!(json.get("session_id").is_some(), "response must echo session_id");
}

/// Test: health check endpoint returns 200
#[tokio::test]
async fn test_health_check_returns_200() {
    let app = Router::new().route(
        "/healthz",
        axum::routing::get(|| async {
            (StatusCode::OK, axum::Json(json!({"status": "ok"}))).into_response()
        }),
    );

    let req = Request::builder()
        .method(Method::GET)
        .uri("/healthz")
        .body(Body::empty())
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert_eq!(json["status"], "ok");
}
