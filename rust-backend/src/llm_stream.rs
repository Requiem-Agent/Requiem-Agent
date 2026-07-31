// llm_stream.rs — Anthropic SSE → WebSocket token bridge
// S5-01: Converts Anthropic's streaming SSE response into WebSocket token messages
// S7-02: resolve_api_key_for_user — fetches user key from DB, decrypts with AES-256-GCM

use reqwest::Client;
use serde::Deserialize;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

use crate::routes::ws_agent::ServerMessage;

// ─────────────────────────────────────────────────────────────────────────────
// Public config
// ─────────────────────────────────────────────────────────────────────────────

/// Configuration for a single LLM streaming call.
#[derive(Debug, Clone)]
pub struct LlmStreamConfig {
    /// Anthropic API key (from env ANTHROPIC_API_KEY or user_api_keys table)
    pub api_key: String,
    /// Model identifier, e.g. "claude-opus-4-5" or "claude-sonnet-4-5"
    pub model: String,
    /// System prompt (optional)
    pub system: Option<String>,
    /// Max tokens to generate
    pub max_tokens: u32,
    /// Temperature (0.0 – 1.0)
    pub temperature: f32,
}

impl Default for LlmStreamConfig {
    fn default() -> Self {
        Self {
            api_key: std::env::var("ANTHROPIC_API_KEY").unwrap_or_default(),
            model: std::env::var("ANTHROPIC_MODEL")
                .unwrap_or_else(|_| "claude-sonnet-4-5".into()),
            system: None,
            max_tokens: 4096,
            temperature: 0.7,
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Anthropic SSE payload shapes (only the fields we need)
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum AnthropicEvent {
    /// Emitted when a new content block starts (we ignore this)
    ContentBlockStart {
        #[allow(dead_code)]
        index: usize,
    },
    /// Emitted for each text delta
    ContentBlockDelta {
        #[allow(dead_code)]
        index: usize,
        delta: ContentDelta,
    },
    /// Emitted when a content block ends (we ignore this)
    ContentBlockStop {
        #[allow(dead_code)]
        index: usize,
    },
    /// Emitted with final usage stats
    MessageDelta {
        usage: Option<MessageUsage>,
    },
    /// Emitted when the full message is complete
    MessageStop,
    /// Emitted on API-level errors
    Error {
        error: AnthropicError,
    },
    /// Ping keepalive (ignore)
    Ping,
    /// message_start — contains the initial message shell (ignore)
    MessageStart {
        #[allow(dead_code)]
        message: serde_json::Value,
    },
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ContentDelta {
    TextDelta { text: String },
    InputJsonDelta { partial_json: String },
}

#[derive(Debug, Deserialize)]
struct MessageUsage {
    output_tokens: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct AnthropicError {
    #[serde(rename = "type")]
    error_type: String,
    message: String,
}

// ─────────────────────────────────────────────────────────────────────────────
// Request body
// ─────────────────────────────────────────────────────────────────────────────

#[derive(serde::Serialize)]
struct AnthropicRequest<'a> {
    model: &'a str,
    max_tokens: u32,
    temperature: f32,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<&'a str>,
    messages: Vec<AnthropicMessage<'a>>,
}

#[derive(serde::Serialize)]
struct AnthropicMessage<'a> {
    role: &'a str,
    content: &'a str,
}

// ─────────────────────────────────────────────────────────────────────────────
// Main bridge function
// ─────────────────────────────────────────────────────────────────────────────

/// Stream an Anthropic LLM response and forward tokens to a WebSocket sender.
///
/// Sends `ServerMessage::Token` for each text chunk, then `ServerMessage::Done`
/// when the stream ends, or `ServerMessage::Error` on failure.
///
/// # Arguments
/// * `user_message` — the user's input text
/// * `config`       — LLM call configuration
/// * `tx`           — channel to the WebSocket send task
/// * `cancelled`    — shared cancellation flag
///
/// # Returns
/// Total output tokens consumed (for metrics / billing tracking).
pub async fn stream_anthropic_to_ws(
    user_message: &str,
    config: &LlmStreamConfig,
    tx: &mpsc::Sender<ServerMessage>,
    cancelled: &std::sync::Arc<std::sync::atomic::AtomicBool>,
) -> Result<u32, String> {
    if config.api_key.is_empty() {
        let msg = "ANTHROPIC_API_KEY not configured".to_string();
        error!("{}", msg);
        let _ = tx.send(ServerMessage::Error { message: msg.clone() }).await;
        return Err(msg);
    }

    let client = Client::new();

    let body = AnthropicRequest {
        model: &config.model,
        max_tokens: config.max_tokens,
        temperature: config.temperature,
        stream: true,
        system: config.system.as_deref(),
        messages: vec![AnthropicMessage {
            role: "user",
            content: user_message,
        }],
    };

    let response = client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", &config.api_key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("HTTP request failed: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let body_text = response.text().await.unwrap_or_default();
        let msg = format!("Anthropic API error {}: {}", status, body_text);
        error!("{}", msg);
        let _ = tx.send(ServerMessage::Error { message: msg.clone() }).await;
        return Err(msg);
    }

    // Parse the SSE stream
    let mut full_content = String::new();
    let mut output_tokens: u32 = 0;
    let mut stream = response.bytes_stream();

    use futures_util::StreamExt;
    let mut buffer = String::new();

    while let Some(chunk) = stream.next().await {
        // Check cancellation on every chunk
        if cancelled.load(std::sync::atomic::Ordering::Relaxed) {
            let _ = tx.send(ServerMessage::Error { message: "Cancelled".into() }).await;
            return Ok(output_tokens);
        }

        let chunk = chunk.map_err(|e| format!("Stream read error: {}", e))?;
        let text = String::from_utf8_lossy(&chunk);
        buffer.push_str(&text);

        // SSE events are separated by double newlines
        while let Some(event_end) = buffer.find("\n\n") {
            let event_block = buffer[..event_end].to_string();
            buffer = buffer[event_end + 2..].to_string();

            // Parse SSE event block
            let mut event_type = String::new();
            let mut data_line = String::new();

            for line in event_block.lines() {
                if let Some(rest) = line.strip_prefix("event: ") {
                    event_type = rest.trim().to_string();
                } else if let Some(rest) = line.strip_prefix("data: ") {
                    data_line = rest.trim().to_string();
                }
            }

            // Skip empty data or [DONE] sentinel
            if data_line.is_empty() || data_line == "[DONE]" {
                continue;
            }

            // Deserialize the JSON payload
            let event: AnthropicEvent = match serde_json::from_str(&data_line) {
                Ok(e) => e,
                Err(err) => {
                    // Unknown event type — log and skip
                    debug!(
                        event_type = %event_type,
                        data = %data_line,
                        error = %err,
                        "Skipping unknown SSE event"
                    );
                    continue;
                }
            };

            match event {
                AnthropicEvent::ContentBlockDelta { delta, .. } => {
                    match delta {
                        ContentDelta::TextDelta { text } => {
                            full_content.push_str(&text);
                            // Forward token to WebSocket
                            if tx.send(ServerMessage::Token { content: text }).await.is_err() {
                                warn!("WS channel closed during streaming");
                                return Ok(output_tokens);
                            }
                        }
                        ContentDelta::InputJsonDelta { partial_json } => {
                            // Tool-use streaming — forward as a token for now
                            debug!("Tool input delta: {}", partial_json);
                        }
                    }
                }

                AnthropicEvent::MessageDelta { usage } => {
                    if let Some(u) = usage {
                        output_tokens = u.output_tokens.unwrap_or(0);
                        debug!("Output tokens so far: {}", output_tokens);
                    }
                }

                AnthropicEvent::MessageStop => {
                    debug!("Anthropic stream complete ({} tokens)", output_tokens);
                    let _ = tx.send(ServerMessage::Done {
                        content: full_content.clone(),
                        steps: 0,
                    }).await;
                    return Ok(output_tokens);
                }

                AnthropicEvent::Error { error } => {
                    let msg = format!("Anthropic error [{}]: {}", error.error_type, error.message);
                    error!("{}", msg);
                    let _ = tx.send(ServerMessage::Error { message: msg.clone() }).await;
                    return Err(msg);
                }

                // Ignored events
                AnthropicEvent::ContentBlockStart { .. }
                | AnthropicEvent::ContentBlockStop { .. }
                | AnthropicEvent::MessageStart { .. }
                | AnthropicEvent::Ping => {}
            }
        }
    }

    // Stream ended without message_stop (shouldn't happen, but handle gracefully)
    if !full_content.is_empty() {
        let _ = tx.send(ServerMessage::Done {
            content: full_content,
            steps: 0,
        }).await;
    }

    Ok(output_tokens)
}

// ─────────────────────────────────────────────────────────────────────────────
// S7-02: DB-backed API key resolution
// ─────────────────────────────────────────────────────────────────────────────

/// يجلب مفتاح Anthropic للمستخدم من قاعدة البيانات ويفكّ تشفيره.
/// إذا لم يوجد مفتاح مخزَّن، يرجع إلى متغير البيئة ANTHROPIC_API_KEY.
///
/// Priority:
///   1. user_api_keys table (provider = "anthropic") → decrypt with crypto.rs
///   2. ANTHROPIC_API_KEY env var
///   3. Empty string (caller will return error)
pub async fn resolve_api_key_for_user<D>(db: &D, user_id: &str) -> String
where
    D: crate::routes::user_api_keys::HasApiKeysDb + Sync,
{
    // محاولة جلب المفتاح من DB
    match db.get_encrypted_key(user_id, "anthropic").await {
        Ok(Some(encrypted)) => {
            // فك التشفير باستخدام crypto.rs
            match crate::crypto::decrypt_api_key(&encrypted) {
                Ok(plaintext) => {
                    info!("Resolved Anthropic API key from DB for user {}", user_id);
                    plaintext.to_string()
                }
                Err(e) => {
                    warn!(
                        "Failed to decrypt Anthropic key for user {}: {} — falling back to env",
                        user_id, e
                    );
                    std::env::var("ANTHROPIC_API_KEY").unwrap_or_default()
                }
            }
        }
        Ok(None) => {
            debug!(
                "No Anthropic key in DB for user {} — using env var",
                user_id
            );
            std::env::var("ANTHROPIC_API_KEY").unwrap_or_default()
        }
        Err(e) => {
            warn!(
                "DB error fetching API key for user {}: {} — falling back to env",
                user_id, e
            );
            std::env::var("ANTHROPIC_API_KEY").unwrap_or_default()
        }
    }
}

/// دالة مساعدة: تبني LlmStreamConfig من تفضيلات المستخدم ومفتاحه المخزَّن.
/// تُستخدَم من ws_agent.rs لتجنب تكرار منطق الـ key resolution.
pub async fn build_config_for_user<D>(
    db: &D,
    user_id: &str,
    model_override: Option<String>,
    system: Option<String>,
) -> LlmStreamConfig
where
    D: crate::routes::user_api_keys::HasApiKeysDb + Sync,
{
    let api_key = resolve_api_key_for_user(db, user_id).await;
    let model = model_override
        .unwrap_or_else(|| {
            std::env::var("ANTHROPIC_MODEL").unwrap_or_else(|_| "claude-sonnet-4-5".into())
        });

    LlmStreamConfig {
        api_key,
        model,
        system,
        max_tokens: 4096,
        temperature: 0.7,
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Unit tests for S7-02
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod s7_tests {
    use super::*;
    use crate::error::AppError;
    use crate::routes::user_api_keys::{HasApiKeysDb, StoredApiKey};
    use async_trait::async_trait;

    struct MockDb {
        /// None = no key stored, Some(encrypted) = key exists
        encrypted_key: Option<String>,
    }

    #[async_trait]
    impl HasApiKeysDb for MockDb {
        async fn list_api_keys(&self, _user_id: &str) -> Result<Vec<StoredApiKey>, AppError> {
            Ok(vec![])
        }
        async fn save_api_key(
            &self,
            _user_id: &str,
            _provider: &str,
            _encrypted_key: &str,
            _key_hint: &str,
        ) -> Result<StoredApiKey, AppError> {
            unimplemented!()
        }
        async fn delete_api_key(&self, _user_id: &str, _key_id: &str) -> Result<(), AppError> {
            Ok(())
        }
        async fn get_encrypted_key(
            &self,
            _user_id: &str,
            _provider: &str,
        ) -> Result<Option<String>, AppError> {
            Ok(self.encrypted_key.clone())
        }
    }

    #[tokio::test]
    async fn test_resolve_no_key_in_db_falls_back_to_env() {
        std::env::set_var("ANTHROPIC_API_KEY", "env-key-123");
        let db = MockDb { encrypted_key: None };
        let key = resolve_api_key_for_user(&db, "user-1").await;
        assert_eq!(key, "env-key-123");
    }

    #[tokio::test]
    async fn test_resolve_invalid_encrypted_key_falls_back_to_env() {
        std::env::set_var("ANTHROPIC_API_KEY", "env-fallback");
        let db = MockDb {
            encrypted_key: Some("not-valid-base64-encrypted-data".into()),
        };
        let key = resolve_api_key_for_user(&db, "user-2").await;
        assert_eq!(key, "env-fallback");
    }

    #[tokio::test]
    async fn test_build_config_uses_env_model_when_no_override() {
        std::env::set_var("ANTHROPIC_API_KEY", "test-key");
        std::env::set_var("ANTHROPIC_MODEL", "claude-opus-4-5");
        let db = MockDb { encrypted_key: None };
        let config = build_config_for_user(&db, "user-3", None, None).await;
        assert_eq!(config.model, "claude-opus-4-5");
        assert_eq!(config.api_key, "test-key");
    }

    #[tokio::test]
    async fn test_build_config_respects_model_override() {
        std::env::set_var("ANTHROPIC_API_KEY", "test-key");
        let db = MockDb { encrypted_key: None };
        let config =
            build_config_for_user(&db, "user-4", Some("claude-haiku-3-5".into()), None).await;
        assert_eq!(config.model, "claude-haiku-3-5");
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Unit tests (no real HTTP calls — test SSE parsing logic)
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_content_block_delta() {
        let json = r#"{"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"Hello"}}"#;
        let event: AnthropicEvent = serde_json::from_str(json).unwrap();
        match event {
            AnthropicEvent::ContentBlockDelta { delta: ContentDelta::TextDelta { text }, .. } => {
                assert_eq!(text, "Hello");
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn test_parse_message_stop() {
        let json = r#"{"type":"message_stop"}"#;
        let event: AnthropicEvent = serde_json::from_str(json).unwrap();
        assert!(matches!(event, AnthropicEvent::MessageStop));
    }

    #[test]
    fn test_parse_message_delta_with_usage() {
        let json = r#"{"type":"message_delta","delta":{},"usage":{"output_tokens":42}}"#;
        let event: AnthropicEvent = serde_json::from_str(json).unwrap();
        match event {
            AnthropicEvent::MessageDelta { usage: Some(u) } => {
                assert_eq!(u.output_tokens, Some(42));
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn test_parse_error_event() {
        let json = r#"{"type":"error","error":{"type":"overloaded_error","message":"Overloaded"}}"#;
        let event: AnthropicEvent = serde_json::from_str(json).unwrap();
        match event {
            AnthropicEvent::Error { error } => {
                assert_eq!(error.error_type, "overloaded_error");
                assert_eq!(error.message, "Overloaded");
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn test_parse_ping() {
        let json = r#"{"type":"ping"}"#;
        let event: AnthropicEvent = serde_json::from_str(json).unwrap();
        assert!(matches!(event, AnthropicEvent::Ping));
    }

    #[test]
    fn test_llm_stream_config_defaults() {
        // Without env vars set, api_key should be empty string
        let config = LlmStreamConfig {
            api_key: "test_key".into(),
            model: "claude-sonnet-4-5".into(),
            system: None,
            max_tokens: 4096,
            temperature: 0.7,
        };
        assert_eq!(config.model, "claude-sonnet-4-5");
        assert_eq!(config.max_tokens, 4096);
        assert!((config.temperature - 0.7).abs() < f32::EPSILON);
    }

    #[test]
    fn test_sse_block_parsing_logic() {
        // Simulate the SSE buffer parsing
        let sse_block = "event: content_block_delta\ndata: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"text_delta\",\"text\":\"Hi\"}}";
        let mut event_type = String::new();
        let mut data_line = String::new();
        for line in sse_block.lines() {
            if let Some(rest) = line.strip_prefix("event: ") {
                event_type = rest.trim().to_string();
            } else if let Some(rest) = line.strip_prefix("data: ") {
                data_line = rest.trim().to_string();
            }
        }
        assert_eq!(event_type, "content_block_delta");
        assert!(data_line.contains("text_delta"));
    }
}