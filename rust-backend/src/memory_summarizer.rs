// memory_summarizer.rs — S9-02: Agent Memory Summarization
// يُلخّص المحادثات الطويلة للحفاظ على context window قابل للإدارة
//
// Strategy:
//   إذا تجاوز عدد الرسائل MAX_MESSAGES → نُلخّص أقدم SUMMARIZE_BATCH رسائل
//   الملخص يُحفَظ في conversation_summaries table
//   الرسائل الملخَّصة تُحذَف من messages table (أو تُعلَّم بـ is_summarized)

use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

// ─────────────────────────────────────────────────────────────────────────────
// Config
// ─────────────────────────────────────────────────────────────────────────────

/// إعدادات نظام التلخيص
#[derive(Debug, Clone)]
pub struct SummarizerConfig {
    /// عدد الرسائل الذي يُشغّل التلخيص
    pub max_messages_before_summarize: usize,
    /// عدد الرسائل التي تُلخَّص في كل مرة
    pub summarize_batch_size: usize,
    /// الـ model المستخدَم للتلخيص
    pub summarizer_model: String,
    /// أقصى عدد tokens للملخص
    pub max_summary_tokens: u32,
}

impl Default for SummarizerConfig {
    fn default() -> Self {
        Self {
            max_messages_before_summarize: 20,
            summarize_batch_size: 10,
            summarizer_model: std::env::var("SUMMARIZER_MODEL")
                .unwrap_or_else(|_| "claude-haiku-3-5".into()),
            max_summary_tokens: 500,
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Types
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageToSummarize {
    pub id: String,
    pub role: String,
    pub content: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SummaryResult {
    pub summary: String,
    pub messages_covered: usize,
    pub model_used: String,
    pub tokens_used: u32,
}

// ─────────────────────────────────────────────────────────────────────────────
// Trait: SummarizerStore
// ─────────────────────────────────────────────────────────────────────────────

#[async_trait::async_trait]
pub trait SummarizerStore: Send + Sync {
    /// جلب أقدم N رسالة في محادثة
    async fn get_oldest_messages(
        &self,
        conversation_id: &str,
        limit: usize,
    ) -> Result<Vec<MessageToSummarize>, String>;

    /// حفظ ملخص محادثة
    async fn save_summary(
        &self,
        conversation_id: &str,
        result: &SummaryResult,
    ) -> Result<(), String>;

    /// حذف الرسائل الملخَّصة
    async fn delete_messages(&self, message_ids: &[String]) -> Result<(), String>;

    /// عدد رسائل محادثة
    async fn count_messages(&self, conversation_id: &str) -> Result<usize, String>;

    /// جلب آخر ملخص للمحادثة
    async fn get_latest_summary(
        &self,
        conversation_id: &str,
    ) -> Result<Option<String>, String>;
}

// ─────────────────────────────────────────────────────────────────────────────
// MemorySummarizer
// ─────────────────────────────────────────────────────────────────────────────

pub struct MemorySummarizer {
    config: SummarizerConfig,
    anthropic_key: String,
}

impl MemorySummarizer {
    pub fn new(config: SummarizerConfig) -> Self {
        let anthropic_key = std::env::var("ANTHROPIC_API_KEY").unwrap_or_default();
        Self { config, anthropic_key }
    }

    /// يتحقق إذا كانت المحادثة تحتاج تلخيصاً ويُنفّذه إذا لزم
    pub async fn maybe_summarize<S: SummarizerStore>(
        &self,
        store: &S,
        conversation_id: &str,
    ) -> Option<SummaryResult> {
        let count = match store.count_messages(conversation_id).await {
            Ok(c) => c,
            Err(e) => {
                warn!("Failed to count messages for {}: {}", conversation_id, e);
                return None;
            }
        };

        if count < self.config.max_messages_before_summarize {
            debug!(
                "Conversation {} has {} messages, no summarization needed",
                conversation_id, count
            );
            return None;
        }

        info!(
            "Conversation {} has {} messages, triggering summarization",
            conversation_id, count
        );

        self.summarize(store, conversation_id).await
    }

    /// يُلخّص أقدم batch من الرسائل
    pub async fn summarize<S: SummarizerStore>(
        &self,
        store: &S,
        conversation_id: &str,
    ) -> Option<SummaryResult> {
        let messages = match store
            .get_oldest_messages(conversation_id, self.config.summarize_batch_size)
            .await
        {
            Ok(m) if !m.is_empty() => m,
            Ok(_) => {
                debug!("No messages to summarize for {}", conversation_id);
                return None;
            }
            Err(e) => {
                warn!("Failed to get messages for summarization: {}", e);
                return None;
            }
        };

        // بناء نص المحادثة للتلخيص
        let conversation_text = messages
            .iter()
            .map(|m| format!("[{}]: {}", m.role, m.content))
            .collect::<Vec<_>>()
            .join("\n\n");

        // استدعاء Anthropic للتلخيص
        let summary = self.call_llm_for_summary(&conversation_text).await?;

        let result = SummaryResult {
            summary: summary.clone(),
            messages_covered: messages.len(),
            model_used: self.config.summarizer_model.clone(),
            tokens_used: estimate_tokens(&summary),
        };

        // حفظ الملخص
        if let Err(e) = store.save_summary(conversation_id, &result).await {
            warn!("Failed to save summary: {}", e);
            return None;
        }

        // حذف الرسائل الملخَّصة
        let ids: Vec<String> = messages.iter().map(|m| m.id.clone()).collect();
        if let Err(e) = store.delete_messages(&ids).await {
            warn!("Failed to delete summarized messages: {}", e);
        }

        info!(
            "Summarized {} messages for conversation {}",
            result.messages_covered, conversation_id
        );

        Some(result)
    }

    /// يبني context string يشمل الملخص + الرسائل الأخيرة
    pub async fn build_context_with_summary<S: SummarizerStore>(
        &self,
        store: &S,
        conversation_id: &str,
        recent_messages: &[MessageToSummarize],
    ) -> String {
        let mut context = String::new();

        // إضافة الملخص إذا وُجد
        if let Ok(Some(summary)) = store.get_latest_summary(conversation_id).await {
            context.push_str("## ملخص المحادثة السابقة:\n");
            context.push_str(&summary);
            context.push_str("\n\n---\n\n## المحادثة الحالية:\n");
        }

        // إضافة الرسائل الأخيرة
        for msg in recent_messages {
            context.push_str(&format!("[{}]: {}\n\n", msg.role, msg.content));
        }

        context
    }

    async fn call_llm_for_summary(&self, conversation_text: &str) -> Option<String> {
        if self.anthropic_key.is_empty() {
            // Fallback: ملخص بسيط بدون LLM
            return Some(self.simple_summary(conversation_text));
        }

        let client = reqwest::Client::new();
        let prompt = format!(
            "لخّص المحادثة التالية في فقرة واحدة موجزة باللغة العربية، مع الحفاظ على النقاط الرئيسية والقرارات المهمة:\n\n{}",
            conversation_text
        );

        let body = serde_json::json!({
            "model": self.config.summarizer_model,
            "max_tokens": self.config.max_summary_tokens,
            "messages": [{"role": "user", "content": prompt}]
        });

        match client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.anthropic_key)
            .header("anthropic-version", "2023-06-01")
            .json(&body)
            .send()
            .await
        {
            Ok(resp) if resp.status().is_success() => {
                resp.json::<serde_json::Value>()
                    .await
                    .ok()
                    .and_then(|v| {
                        v["content"][0]["text"]
                            .as_str()
                            .map(|s| s.to_string())
                    })
            }
            Ok(resp) => {
                warn!("Summarizer API error: {}", resp.status());
                Some(self.simple_summary(conversation_text))
            }
            Err(e) => {
                warn!("Summarizer request failed: {}", e);
                Some(self.simple_summary(conversation_text))
            }
        }
    }

    /// ملخص بسيط بدون LLM (fallback)
    fn simple_summary(&self, text: &str) -> String {
        let lines: Vec<&str> = text.lines().take(5).collect();
        format!(
            "ملخص تلقائي: تحتوي المحادثة على {} رسالة. أول الرسائل: {}",
            text.lines().count(),
            lines.join(" | ")
        )
    }
}

fn estimate_tokens(text: &str) -> u32 {
    // تقدير تقريبي: كل 4 أحرف ≈ token
    (text.len() / 4) as u32
}

// ─────────────────────────────────────────────────────────────────────────────
// Unit tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    struct MockStore {
        messages: Mutex<Vec<MessageToSummarize>>,
        summaries: Mutex<Vec<String>>,
    }

    impl MockStore {
        fn new(messages: Vec<MessageToSummarize>) -> Self {
            Self {
                messages: Mutex::new(messages),
                summaries: Mutex::new(Vec::new()),
            }
        }
    }

    #[async_trait::async_trait]
    impl SummarizerStore for MockStore {
        async fn get_oldest_messages(&self, _conv_id: &str, limit: usize) -> Result<Vec<MessageToSummarize>, String> {
            Ok(self.messages.lock().unwrap().iter().take(limit).cloned().collect())
        }
        async fn save_summary(&self, _conv_id: &str, result: &SummaryResult) -> Result<(), String> {
            self.summaries.lock().unwrap().push(result.summary.clone());
            Ok(())
        }
        async fn delete_messages(&self, ids: &[String]) -> Result<(), String> {
            self.messages.lock().unwrap().retain(|m| !ids.contains(&m.id));
            Ok(())
        }
        async fn count_messages(&self, _conv_id: &str) -> Result<usize, String> {
            Ok(self.messages.lock().unwrap().len())
        }
        async fn get_latest_summary(&self, _conv_id: &str) -> Result<Option<String>, String> {
            Ok(self.summaries.lock().unwrap().last().cloned())
        }
    }

    fn make_messages(n: usize) -> Vec<MessageToSummarize> {
        (0..n).map(|i| MessageToSummarize {
            id: format!("msg-{}", i),
            role: if i % 2 == 0 { "user".into() } else { "assistant".into() },
            content: format!("رسالة رقم {}", i),
            created_at: "2026-01-01T00:00:00Z".into(),
        }).collect()
    }

    #[tokio::test]
    async fn test_no_summarization_below_threshold() {
        let config = SummarizerConfig { max_messages_before_summarize: 20, ..Default::default() };
        let summarizer = MemorySummarizer::new(config);
        let store = MockStore::new(make_messages(5));
        let result = summarizer.maybe_summarize(&store, "conv-1").await;
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_summarization_triggered_above_threshold() {
        let config = SummarizerConfig {
            max_messages_before_summarize: 5,
            summarize_batch_size: 3,
            ..Default::default()
        };
        let summarizer = MemorySummarizer::new(config);
        let store = MockStore::new(make_messages(10));
        let result = summarizer.maybe_summarize(&store, "conv-1").await;
        // يجب أن يُلخّص (حتى بدون Anthropic key — يستخدم simple_summary)
        assert!(result.is_some());
        let r = result.unwrap();
        assert_eq!(r.messages_covered, 3);
    }

    #[tokio::test]
    async fn test_messages_deleted_after_summarization() {
        let config = SummarizerConfig {
            max_messages_before_summarize: 3,
            summarize_batch_size: 3,
            ..Default::default()
        };
        let summarizer = MemorySummarizer::new(config);
        let store = MockStore::new(make_messages(5));
        summarizer.summarize(&store, "conv-1").await;
        let remaining = store.count_messages("conv-1").await.unwrap();
        assert_eq!(remaining, 2); // 5 - 3 = 2
    }

    #[test]
    fn test_estimate_tokens() {
        let text = "a".repeat(400);
        assert_eq!(estimate_tokens(&text), 100);
    }
}
