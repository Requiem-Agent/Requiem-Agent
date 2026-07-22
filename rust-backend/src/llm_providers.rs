// llm_providers.rs — S8-01: Multi-model LLM provider support
// يدعم: Anthropic (Claude), OpenAI (GPT), Google Gemini, Mistral
// كل provider يحوّل SSE stream إلى ServerMessage tokens عبر mpsc channel

use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::{atomic::AtomicBool, Arc};
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

use crate::routes::ws_agent::ServerMessage;

// ─────────────────────────────────────────────────────────────────────────────
// Provider enum + config
// ─────────────────────────────────────────────────────────────────────────────

/// LLM provider identifier
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LlmProvider {
    Anthropic,
    OpenAI,
    Gemini,
    Mistral,
}

impl LlmProvider {
    /// اسم الـ provider كما يُخزَّن في user_api_keys.provider
    pub fn db_name(&self) -> &'static str {
        match self {
            Self::Anthropic => "anthropic",
            Self::OpenAI => "openai",
            Self::Gemini => "gemini",
            Self::Mistral => "mistral",
        }
    }

    /// الـ model الافتراضي لكل provider
    pub fn default_model(&self) -> &'static str {
        match self {
            Self::Anthropic => "claude-sonnet-4-5",
            Self::OpenAI => "gpt-4o",
            Self::Gemini => "gemini-1.5-pro",
            Self::Mistral => "mistral-large-latest",
        }
    }

    /// Base URL لـ API
    pub fn api_base(&self) -> &'static str {
        match self {
            Self::Anthropic => "https://api.anthropic.com/v1",
            Self::OpenAI => "https://api.openai.com/v1",
            Self::Gemini => "https://generativelanguage.googleapis.com/v1beta",
            Self::Mistral => "https://api.mistral.ai/v1",
        }
    }

    /// Parse from string (case-insensitive)
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "anthropic" | "claude" => Some(Self::Anthropic),
            "openai" | "gpt" | "chatgpt" => Some(Self::OpenAI),
            "gemini" | "google" => Some(Self::Gemini),
            "mistral" => Some(Self::Mistral),
            _ => None,
        }
    }
}

/// Unified config for any LLM provider
#[derive(Debug, Clone)]
pub struct MultiProviderConfig {
    pub provider: LlmProvider,
    pub api_key: String,
    pub model: String,
    pub system: Option<String>,
    pub max_tokens: u32,
    pub temperature: f32,
}

impl MultiProviderConfig {
    pub fn new(provider: LlmProvider, api_key: String) -> Self {
        let model = provider.default_model().to_string();
        Self {
            provider,
            api_key,
            model,
            system: None,
            max_tokens: 4096,
            temperature: 0.7,
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// OpenAI-compatible SSE shapes (used by OpenAI + Mistral)
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct OpenAiChunk {
    choices: Vec<OpenAiChoice>,
}

#[derive(Debug, Deserialize)]
struct OpenAiChoice {
    delta: OpenAiDelta,
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OpenAiDelta {
    content: Option<String>,
}

#[derive(Debug, Serialize)]
struct OpenAiRequest<'a> {
    model: &'a str,
    messages: Vec<OpenAiMessage<'a>>,
    max_tokens: u32,
    temperature: f32,
    stream: bool,
}

#[derive(Debug, Serialize)]
struct OpenAiMessage<'a> {
    role: &'a str,
    content: &'a str,
}

// ─────────────────────────────────────────────────────────────────────────────
// Gemini SSE shapes
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct GeminiChunk {
    candidates: Option<Vec<GeminiCandidate>>,
}

#[derive(Debug, Deserialize)]
struct GeminiCandidate {
    content: Option<GeminiContent>,
    #[serde(rename = "finishReason")]
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GeminiContent {
    parts: Vec<GeminiPart>,
}

#[derive(Debug, Deserialize)]
struct GeminiPart {
    text: Option<String>,
}

#[derive(Debug, Serialize)]
struct GeminiRequest<'a> {
    contents: Vec<GeminiRequestContent<'a>>,
    #[serde(rename = "generationConfig")]
    generation_config: GeminiGenerationConfig,
    #[serde(rename = "systemInstruction", skip_serializing_if = "Option::is_none")]
    system_instruction: Option<GeminiSystemInstruction<'a>>,
}

#[derive(Debug, Serialize)]
struct GeminiRequestContent<'a> {
    role: &'a str,
    parts: Vec<GeminiRequestPart<'a>>,
}

#[derive(Debug, Serialize)]
struct GeminiRequestPart<'a> {
    text: &'a str,
}

#[derive(Debug, Serialize)]
struct GeminiGenerationConfig {
    #[serde(rename = "maxOutputTokens")]
    max_output_tokens: u32,
    temperature: f32,
}

#[derive(Debug, Serialize)]
struct GeminiSystemInstruction<'a> {
    parts: Vec<GeminiRequestPart<'a>>,
}

// ─────────────────────────────────────────────────────────────────────────────
// Main dispatch function
// ─────────────────────────────────────────────────────────────────────────────

/// يُرسل رسالة إلى أي LLM provider ويبثّ الـ tokens عبر WebSocket channel.
/// يختار الـ implementation المناسب بناءً على `config.provider`.
pub async fn stream_to_ws(
    user_message: &str,
    config: &MultiProviderConfig,
    tx: &mpsc::Sender<ServerMessage>,
    cancelled: &Arc<AtomicBool>,
) -> Result<u32, String> {
    if config.api_key.is_empty() {
        let msg = format!("{:?} API key not configured", config.provider);
        error!("{}", msg);
        let _ = tx.send(ServerMessage::Error { message: msg.clone() }).await;
        return Err(msg);
    }

    info!(
        provider = ?config.provider,
        model = %config.model,
        "Starting LLM stream"
    );

    match config.provider {
        LlmProvider::Anthropic => {
            stream_anthropic(user_message, config, tx, cancelled).await
        }
        LlmProvider::OpenAI | LlmProvider::Mistral => {
            stream_openai_compatible(user_message, config, tx, cancelled).await
        }
        LlmProvider::Gemini => {
            stream_gemini(user_message, config, tx, cancelled).await
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Anthropic streaming
// ─────────────────────────────────────────────────────────────────────────────

async fn stream_anthropic(
    user_message: &str,
    config: &MultiProviderConfig,
    tx: &mpsc::Sender<ServerMessage>,
    cancelled: &Arc<AtomicBool>,
) -> Result<u32, String> {
    use crate::llm_stream::{LlmStreamConfig, stream_anthropic_to_ws};

    let llm_config = LlmStreamConfig {
        api_key: config.api_key.clone(),
        model: config.model.clone(),
        system: config.system.clone(),
        max_tokens: config.max_tokens,
        temperature: config.temperature,
    };

    // نستخدم الـ implementation الموجود في llm_stream.rs
    let cancelled_bool = Arc::new(tokio::sync::AtomicBool::new(
        cancelled.load(std::sync::atomic::Ordering::Relaxed),
    ));
    stream_anthropic_to_ws(user_message, &llm_config, tx, &cancelled_bool).await
}

// ─────────────────────────────────────────────────────────────────────────────
// OpenAI-compatible streaming (OpenAI + Mistral)
// ─────────────────────────────────────────────────────────────────────────────

async fn stream_openai_compatible(
    user_message: &str,
    config: &MultiProviderConfig,
    tx: &mpsc::Sender<ServerMessage>,
    cancelled: &Arc<AtomicBool>,
) -> Result<u32, String> {
    let client = Client::new();

    let mut messages = Vec::new();
    if let Some(ref sys) = config.system {
        messages.push(OpenAiMessage { role: "system", content: sys.as_str() });
    }
    messages.push(OpenAiMessage { role: "user", content: user_message });

    let body = OpenAiRequest {
        model: &config.model,
        messages,
        max_tokens: config.max_tokens,
        temperature: config.temperature,
        stream: true,
    };

    let url = format!("{}/chat/completions", config.provider.api_base());

    let response = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", config.api_key))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("HTTP request failed: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let body_text = response.text().await.unwrap_or_default();
        let msg = format!("{:?} API error {}: {}", config.provider, status, body_text);
        error!("{}", msg);
        let _ = tx.send(ServerMessage::Error { message: msg.clone() }).await;
        return Err(msg);
    }

    let mut total_tokens = 0u32;
    let mut full_content = String::new();
    let mut stream = response.bytes_stream();

    use futures_util::StreamExt;
    let mut buffer = String::new();

    while let Some(chunk) = stream.next().await {
        if cancelled.load(std::sync::atomic::Ordering::Relaxed) {
            debug!("OpenAI stream cancelled");
            break;
        }

        let bytes = chunk.map_err(|e| format!("Stream error: {}", e))?;
        buffer.push_str(&String::from_utf8_lossy(&bytes));

        // Process complete SSE lines
        while let Some(newline_pos) = buffer.find('\n') {
            let line = buffer[..newline_pos].trim().to_string();
            buffer = buffer[newline_pos + 1..].to_string();

            if line.is_empty() || line == "data: [DONE]" {
                continue;
            }

            if let Some(json_str) = line.strip_prefix("data: ") {
                match serde_json::from_str::<OpenAiChunk>(json_str) {
                    Ok(chunk) => {
                        for choice in &chunk.choices {
                            if let Some(ref text) = choice.delta.content {
                                if !text.is_empty() {
                                    full_content.push_str(text);
                                    total_tokens += 1;
                                    let _ = tx
                                        .send(ServerMessage::Token { content: text.clone() })
                                        .await;
                                }
                            }
                            if choice.finish_reason.as_deref() == Some("stop") {
                                debug!("OpenAI stream finished");
                            }
                        }
                    }
                    Err(e) => {
                        warn!("Failed to parse OpenAI chunk: {} — raw: {}", e, json_str);
                    }
                }
            }
        }
    }

    let _ = tx
        .send(ServerMessage::Done {
            content: full_content,
            steps: 0,
        })
        .await;

    Ok(total_tokens)
}

// ─────────────────────────────────────────────────────────────────────────────
// Google Gemini streaming
// ─────────────────────────────────────────────────────────────────────────────

async fn stream_gemini(
    user_message: &str,
    config: &MultiProviderConfig,
    tx: &mpsc::Sender<ServerMessage>,
    cancelled: &Arc<AtomicBool>,
) -> Result<u32, String> {
    let client = Client::new();

    let system_instruction = config.system.as_ref().map(|s| GeminiSystemInstruction {
        parts: vec![GeminiRequestPart { text: s.as_str() }],
    });

    let body = GeminiRequest {
        contents: vec![GeminiRequestContent {
            role: "user",
            parts: vec![GeminiRequestPart { text: user_message }],
        }],
        generation_config: GeminiGenerationConfig {
            max_output_tokens: config.max_tokens,
            temperature: config.temperature,
        },
        system_instruction,
    };

    // Gemini uses ?key=API_KEY in URL + alt=sse for streaming
    let url = format!(
        "{}/models/{}:streamGenerateContent?key={}&alt=sse",
        config.provider.api_base(),
        config.model,
        config.api_key
    );

    let response = client
        .post(&url)
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Gemini HTTP request failed: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let body_text = response.text().await.unwrap_or_default();
        let msg = format!("Gemini API error {}: {}", status, body_text);
        error!("{}", msg);
        let _ = tx.send(ServerMessage::Error { message: msg.clone() }).await;
        return Err(msg);
    }

    let mut total_tokens = 0u32;
    let mut full_content = String::new();
    let mut stream = response.bytes_stream();
    let mut buffer = String::new();

    use futures_util::StreamExt;

    while let Some(chunk) = stream.next().await {
        if cancelled.load(std::sync::atomic::Ordering::Relaxed) {
            break;
        }

        let bytes = chunk.map_err(|e| format!("Gemini stream error: {}", e))?;
        buffer.push_str(&String::from_utf8_lossy(&bytes));

        while let Some(newline_pos) = buffer.find('\n') {
            let line = buffer[..newline_pos].trim().to_string();
            buffer = buffer[newline_pos + 1..].to_string();

            if line.is_empty() {
                continue;
            }

            if let Some(json_str) = line.strip_prefix("data: ") {
                match serde_json::from_str::<GeminiChunk>(json_str) {
                    Ok(chunk) => {
                        if let Some(candidates) = chunk.candidates {
                            for candidate in candidates {
                                if let Some(content) = candidate.content {
                                    for part in content.parts {
                                        if let Some(text) = part.text {
                                            if !text.is_empty() {
                                                full_content.push_str(&text);
                                                total_tokens += 1;
                                                let _ = tx
                                                    .send(ServerMessage::Token { content: text })
                                                    .await;
                                            }
                                        }
                                    }
                                }
                                if candidate.finish_reason.as_deref() == Some("STOP") {
                                    debug!("Gemini stream finished");
                                }
                            }
                        }
                    }
                    Err(e) => {
                        warn!("Failed to parse Gemini chunk: {} — raw: {}", e, json_str);
                    }
                }
            }
        }
    }

    let _ = tx
        .send(ServerMessage::Done {
            content: full_content,
            steps: 0,
        })
        .await;

    Ok(total_tokens)
}

// ─────────────────────────────────────────────────────────────────────────────
// Key resolution for any provider
// ─────────────────────────────────────────────────────────────────────────────

/// يجلب مفتاح API لأي provider من DB أو env var.
pub async fn resolve_provider_key<D>(
    db: &D,
    user_id: &str,
    provider: &LlmProvider,
) -> String
where
    D: crate::routes::user_api_keys::HasApiKeysDb + Sync,
{
    let provider_name = provider.db_name();

    match db.get_encrypted_key(user_id, provider_name).await {
        Ok(Some(encrypted)) => {
            match crate::crypto::decrypt_api_key(&encrypted) {
                Ok(plaintext) => {
                    info!("Resolved {} API key from DB for user {}", provider_name, user_id);
                    plaintext.to_string()
                }
                Err(e) => {
                    warn!("Failed to decrypt {} key for user {}: {}", provider_name, user_id, e);
                    env_key_for_provider(provider)
                }
            }
        }
        Ok(None) => {
            debug!("No {} key in DB for user {} — using env", provider_name, user_id);
            env_key_for_provider(provider)
        }
        Err(e) => {
            warn!("DB error fetching {} key for user {}: {}", provider_name, user_id, e);
            env_key_for_provider(provider)
        }
    }
}

fn env_key_for_provider(provider: &LlmProvider) -> String {
    let env_var = match provider {
        LlmProvider::Anthropic => "ANTHROPIC_API_KEY",
        LlmProvider::OpenAI => "OPENAI_API_KEY",
        LlmProvider::Gemini => "GEMINI_API_KEY",
        LlmProvider::Mistral => "MISTRAL_API_KEY",
    };
    std::env::var(env_var).unwrap_or_default()
}

// ─────────────────────────────────────────────────────────────────────────────
// Unit tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_db_names() {
        assert_eq!(LlmProvider::Anthropic.db_name(), "anthropic");
        assert_eq!(LlmProvider::OpenAI.db_name(), "openai");
        assert_eq!(LlmProvider::Gemini.db_name(), "gemini");
        assert_eq!(LlmProvider::Mistral.db_name(), "mistral");
    }

    #[test]
    fn test_provider_from_str() {
        assert_eq!(LlmProvider::from_str("anthropic"), Some(LlmProvider::Anthropic));
        assert_eq!(LlmProvider::from_str("Claude"), Some(LlmProvider::Anthropic));
        assert_eq!(LlmProvider::from_str("gpt"), Some(LlmProvider::OpenAI));
        assert_eq!(LlmProvider::from_str("GEMINI"), Some(LlmProvider::Gemini));
        assert_eq!(LlmProvider::from_str("mistral"), Some(LlmProvider::Mistral));
        assert_eq!(LlmProvider::from_str("unknown"), None);
    }

    #[test]
    fn test_provider_default_models() {
        assert!(LlmProvider::Anthropic.default_model().contains("claude"));
        assert!(LlmProvider::OpenAI.default_model().contains("gpt"));
        assert!(LlmProvider::Gemini.default_model().contains("gemini"));
        assert!(LlmProvider::Mistral.default_model().contains("mistral"));
    }

    #[test]
    fn test_multi_provider_config_new() {
        let config = MultiProviderConfig::new(LlmProvider::OpenAI, "sk-test".into());
        assert_eq!(config.provider, LlmProvider::OpenAI);
        assert_eq!(config.api_key, "sk-test");
        assert!(config.model.contains("gpt"));
        assert_eq!(config.max_tokens, 4096);
    }

    #[test]
    fn test_api_base_urls() {
        assert!(LlmProvider::Anthropic.api_base().contains("anthropic.com"));
        assert!(LlmProvider::OpenAI.api_base().contains("openai.com"));
        assert!(LlmProvider::Gemini.api_base().contains("googleapis.com"));
        assert!(LlmProvider::Mistral.api_base().contains("mistral.ai"));
    }
}
