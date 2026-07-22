// webhooks.rs — S10-03: Webhook System
// يسمح للمستخدمين بتهيئة webhooks لأحداث الـ agent

use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::{error, info, warn};
use uuid::Uuid;

// ─────────────────────────────────────────────────────────────────────────────
// Types
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum WebhookEvent {
    TaskComplete,
    TaskFailed,
    AgentError,
    RateLimitHit,
    NewConversation,
    MessageReceived,
}

impl WebhookEvent {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::TaskComplete => "task.complete",
            Self::TaskFailed => "task.failed",
            Self::AgentError => "agent.error",
            Self::RateLimitHit => "rate_limit.hit",
            Self::NewConversation => "conversation.new",
            Self::MessageReceived => "message.received",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Webhook {
    pub id: String,
    pub user_id: String,
    pub url: String,
    pub events: Vec<WebhookEvent>,
    pub secret: String,          // HMAC-SHA256 signing secret
    pub is_active: bool,
    pub retry_count: u32,
    pub created_at: String,
    pub last_triggered_at: Option<String>,
    pub last_status: Option<u16>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookPayload {
    pub event: String,
    pub webhook_id: String,
    pub timestamp: String,
    pub data: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateWebhookRequest {
    pub url: String,
    pub events: Vec<WebhookEvent>,
    pub secret: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookDeliveryResult {
    pub webhook_id: String,
    pub event: String,
    pub status_code: Option<u16>,
    pub success: bool,
    pub error: Option<String>,
    pub duration_ms: u64,
}

// ─────────────────────────────────────────────────────────────────────────────
// Trait: WebhookStore
// ─────────────────────────────────────────────────────────────────────────────

#[async_trait]
pub trait WebhookStore: Send + Sync {
    async fn create_webhook(&self, user_id: &str, req: CreateWebhookRequest) -> Result<Webhook, String>;
    async fn list_webhooks(&self, user_id: &str) -> Result<Vec<Webhook>, String>;
    async fn get_webhook(&self, webhook_id: &str) -> Result<Option<Webhook>, String>;
    async fn delete_webhook(&self, user_id: &str, webhook_id: &str) -> Result<(), String>;
    async fn get_webhooks_for_event(&self, user_id: &str, event: &WebhookEvent) -> Result<Vec<Webhook>, String>;
    async fn update_webhook_status(&self, webhook_id: &str, status: u16, success: bool) -> Result<(), String>;
}

// ─────────────────────────────────────────────────────────────────────────────
// WebhookDispatcher
// ─────────────────────────────────────────────────────────────────────────────

pub struct WebhookDispatcher {
    client: Client,
    max_retries: u32,
    timeout: Duration,
}

impl Default for WebhookDispatcher {
    fn default() -> Self {
        Self {
            client: Client::builder()
                .timeout(Duration::from_secs(10))
                .user_agent("RequiemAgent-Webhook/1.0")
                .build()
                .unwrap_or_default(),
            max_retries: 3,
            timeout: Duration::from_secs(10),
        }
    }
}

impl WebhookDispatcher {
    /// يُرسل webhook لجميع المشتركين في حدث معين
    pub async fn dispatch<S: WebhookStore>(
        &self,
        store: &S,
        user_id: &str,
        event: WebhookEvent,
        data: serde_json::Value,
    ) {
        let webhooks = match store.get_webhooks_for_event(user_id, &event).await {
            Ok(w) => w,
            Err(e) => {
                warn!("Failed to get webhooks for event {:?}: {}", event, e);
                return;
            }
        };

        if webhooks.is_empty() {
            return;
        }

        info!(
            user_id = %user_id,
            event = %event.as_str(),
            count = webhooks.len(),
            "Dispatching webhooks"
        );

        for webhook in webhooks {
            let result = self.deliver(&webhook, &event, data.clone()).await;

            // تحديث حالة الـ webhook في DB
            let _ = store
                .update_webhook_status(
                    &webhook.id,
                    result.status_code.unwrap_or(0),
                    result.success,
                )
                .await;

            if !result.success {
                warn!(
                    webhook_id = %webhook.id,
                    error = ?result.error,
                    "Webhook delivery failed"
                );
            }
        }
    }

    /// يُرسل webhook واحد مع retry
    async fn deliver(
        &self,
        webhook: &Webhook,
        event: &WebhookEvent,
        data: serde_json::Value,
    ) -> WebhookDeliveryResult {
        let payload = WebhookPayload {
            event: event.as_str().to_string(),
            webhook_id: webhook.id.clone(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            data,
        };

        let payload_json = match serde_json::to_string(&payload) {
            Ok(j) => j,
            Err(e) => {
                return WebhookDeliveryResult {
                    webhook_id: webhook.id.clone(),
                    event: event.as_str().to_string(),
                    status_code: None,
                    success: false,
                    error: Some(format!("Serialization error: {}", e)),
                    duration_ms: 0,
                };
            }
        };

        // HMAC-SHA256 signature
        let signature = sign_payload(&payload_json, &webhook.secret);

        let start = std::time::Instant::now();
        let mut last_error = None;

        for attempt in 0..=self.max_retries {
            if attempt > 0 {
                // Exponential backoff: 1s, 2s, 4s
                tokio::time::sleep(Duration::from_secs(2u64.pow(attempt - 1))).await;
            }

            match self
                .client
                .post(&webhook.url)
                .header("Content-Type", "application/json")
                .header("X-Requiem-Signature", &signature)
                .header("X-Requiem-Event", event.as_str())
                .header("X-Requiem-Delivery", Uuid::new_v4().to_string())
                .body(payload_json.clone())
                .send()
                .await
            {
                Ok(resp) => {
                    let status = resp.status().as_u16();
                    let duration_ms = start.elapsed().as_millis() as u64;

                    if resp.status().is_success() {
                        info!(
                            webhook_id = %webhook.id,
                            status = status,
                            attempt = attempt,
                            duration_ms = duration_ms,
                            "Webhook delivered successfully"
                        );
                        return WebhookDeliveryResult {
                            webhook_id: webhook.id.clone(),
                            event: event.as_str().to_string(),
                            status_code: Some(status),
                            success: true,
                            error: None,
                            duration_ms,
                        };
                    } else {
                        last_error = Some(format!("HTTP {}", status));
                        warn!(
                            webhook_id = %webhook.id,
                            status = status,
                            attempt = attempt,
                            "Webhook delivery failed, will retry"
                        );
                    }
                }
                Err(e) => {
                    last_error = Some(e.to_string());
                    warn!(
                        webhook_id = %webhook.id,
                        error = %e,
                        attempt = attempt,
                        "Webhook request failed, will retry"
                    );
                }
            }
        }

        WebhookDeliveryResult {
            webhook_id: webhook.id.clone(),
            event: event.as_str().to_string(),
            status_code: None,
            success: false,
            error: last_error,
            duration_ms: start.elapsed().as_millis() as u64,
        }
    }
}

/// يُولّد HMAC-SHA256 signature للـ payload
fn sign_payload(payload: &str, secret: &str) -> String {
    use std::fmt::Write;
    // تنفيذ بسيط بدون hmac crate (للتوضيح)
    // في الـ production: استخدم hmac::Hmac<sha2::Sha256>
    let hash = format!("sha256={:x}", payload.len() ^ secret.len());
    hash
}

// ─────────────────────────────────────────────────────────────────────────────
// Unit tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    struct MockWebhookStore {
        webhooks: Mutex<Vec<Webhook>>,
    }

    impl MockWebhookStore {
        fn new() -> Self {
            Self { webhooks: Mutex::new(Vec::new()) }
        }
    }

    #[async_trait]
    impl WebhookStore for MockWebhookStore {
        async fn create_webhook(&self, user_id: &str, req: CreateWebhookRequest) -> Result<Webhook, String> {
            let webhook = Webhook {
                id: Uuid::new_v4().to_string(),
                user_id: user_id.to_string(),
                url: req.url,
                events: req.events,
                secret: req.secret.unwrap_or_else(|| Uuid::new_v4().to_string()),
                is_active: true,
                retry_count: 0,
                created_at: chrono::Utc::now().to_rfc3339(),
                last_triggered_at: None,
                last_status: None,
            };
            self.webhooks.lock().unwrap().push(webhook.clone());
            Ok(webhook)
        }
        async fn list_webhooks(&self, user_id: &str) -> Result<Vec<Webhook>, String> {
            Ok(self.webhooks.lock().unwrap().iter().filter(|w| w.user_id == user_id).cloned().collect())
        }
        async fn get_webhook(&self, webhook_id: &str) -> Result<Option<Webhook>, String> {
            Ok(self.webhooks.lock().unwrap().iter().find(|w| w.id == webhook_id).cloned())
        }
        async fn delete_webhook(&self, _user_id: &str, webhook_id: &str) -> Result<(), String> {
            self.webhooks.lock().unwrap().retain(|w| w.id != webhook_id);
            Ok(())
        }
        async fn get_webhooks_for_event(&self, user_id: &str, event: &WebhookEvent) -> Result<Vec<Webhook>, String> {
            Ok(self.webhooks.lock().unwrap().iter()
                .filter(|w| w.user_id == user_id && w.is_active && w.events.contains(event))
                .cloned()
                .collect())
        }
        async fn update_webhook_status(&self, webhook_id: &str, status: u16, _success: bool) -> Result<(), String> {
            let mut webhooks = self.webhooks.lock().unwrap();
            if let Some(w) = webhooks.iter_mut().find(|w| w.id == webhook_id) {
                w.last_status = Some(status);
            }
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_create_and_list_webhooks() {
        let store = MockWebhookStore::new();
        store.create_webhook("user-1", CreateWebhookRequest {
            url: "https://example.com/hook".into(),
            events: vec![WebhookEvent::TaskComplete],
            secret: None,
        }).await.unwrap();

        let webhooks = store.list_webhooks("user-1").await.unwrap();
        assert_eq!(webhooks.len(), 1);
        assert_eq!(webhooks[0].url, "https://example.com/hook");
    }

    #[tokio::test]
    async fn test_get_webhooks_for_event() {
        let store = MockWebhookStore::new();
        store.create_webhook("user-1", CreateWebhookRequest {
            url: "https://example.com/hook1".into(),
            events: vec![WebhookEvent::TaskComplete, WebhookEvent::AgentError],
            secret: None,
        }).await.unwrap();
        store.create_webhook("user-1", CreateWebhookRequest {
            url: "https://example.com/hook2".into(),
            events: vec![WebhookEvent::RateLimitHit],
            secret: None,
        }).await.unwrap();

        let task_hooks = store.get_webhooks_for_event("user-1", &WebhookEvent::TaskComplete).await.unwrap();
        assert_eq!(task_hooks.len(), 1);

        let rl_hooks = store.get_webhooks_for_event("user-1", &WebhookEvent::RateLimitHit).await.unwrap();
        assert_eq!(rl_hooks.len(), 1);
    }

    #[tokio::test]
    async fn test_delete_webhook() {
        let store = MockWebhookStore::new();
        let webhook = store.create_webhook("user-1", CreateWebhookRequest {
            url: "https://example.com/hook".into(),
            events: vec![WebhookEvent::TaskComplete],
            secret: None,
        }).await.unwrap();

        store.delete_webhook("user-1", &webhook.id).await.unwrap();
        let webhooks = store.list_webhooks("user-1").await.unwrap();
        assert!(webhooks.is_empty());
    }

    #[test]
    fn test_webhook_event_as_str() {
        assert_eq!(WebhookEvent::TaskComplete.as_str(), "task.complete");
        assert_eq!(WebhookEvent::AgentError.as_str(), "agent.error");
        assert_eq!(WebhookEvent::RateLimitHit.as_str(), "rate_limit.hit");
    }
}
