// rag_memory.rs — S8-03: RAG Memory Integration
// يربط جداول RAG memory بـ ReActEngine لتذكّر السياق عبر المحادثات
//
// Architecture:
//   ReActEngine → RagMemoryStore → PostgreSQL (memories + embeddings tables)
//   عند كل محادثة: جلب الذكريات ذات الصلة → حقنها في system prompt
//   بعد المحادثة: حفظ الذكريات الجديدة

use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

// ─────────────────────────────────────────────────────────────────────────────
// Types
// ─────────────────────────────────────────────────────────────────────────────

/// ذاكرة واحدة مخزَّنة في قاعدة البيانات
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Memory {
    pub id: String,
    pub user_id: String,
    pub content: String,
    pub memory_type: MemoryType,
    pub importance: f32,        // 0.0 - 1.0
    pub access_count: i32,
    pub created_at: String,
    pub last_accessed_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum MemoryType {
    Fact,           // حقيقة عن المستخدم أو العالم
    Preference,     // تفضيل المستخدم
    Conversation,   // ملخص محادثة سابقة
    Task,           // مهمة أو هدف
    Skill,          // مهارة أو قدرة تعلّمها الـ agent
}

/// نتيجة البحث في الذاكرة مع درجة التشابه
#[derive(Debug, Clone)]
pub struct MemorySearchResult {
    pub memory: Memory,
    pub similarity: f32,    // 0.0 - 1.0 (cosine similarity)
}

/// طلب حفظ ذاكرة جديدة
#[derive(Debug, Clone)]
pub struct SaveMemoryRequest {
    pub user_id: String,
    pub content: String,
    pub memory_type: MemoryType,
    pub importance: f32,
}

// ─────────────────────────────────────────────────────────────────────────────
// Trait: RagMemoryStore
// ─────────────────────────────────────────────────────────────────────────────

/// Trait يجب أن يُطبَّق من AppState لدعم RAG memory
#[async_trait::async_trait]
pub trait RagMemoryStore: Send + Sync {
    /// البحث في الذاكرة بالتشابه الدلالي
    async fn search_memories(
        &self,
        user_id: &str,
        query: &str,
        limit: usize,
    ) -> Result<Vec<MemorySearchResult>, String>;

    /// حفظ ذاكرة جديدة
    async fn save_memory(&self, req: SaveMemoryRequest) -> Result<Memory, String>;

    /// جلب أحدث الذكريات (بدون بحث دلالي)
    async fn get_recent_memories(
        &self,
        user_id: &str,
        limit: usize,
    ) -> Result<Vec<Memory>, String>;

    /// حذف ذاكرة
    async fn delete_memory(&self, user_id: &str, memory_id: &str) -> Result<(), String>;

    /// تحديث access_count و last_accessed_at
    async fn mark_accessed(&self, memory_ids: &[String]) -> Result<(), String>;
}

// ─────────────────────────────────────────────────────────────────────────────
// RagContextBuilder: يبني system prompt context من الذاكرة
// ─────────────────────────────────────────────────────────────────────────────

/// يبني context للـ ReActEngine من الذكريات ذات الصلة
pub struct RagContextBuilder {
    pub max_memories: usize,
    pub min_similarity: f32,
    pub max_context_chars: usize,
}

impl Default for RagContextBuilder {
    fn default() -> Self {
        Self {
            max_memories: 5,
            min_similarity: 0.6,
            max_context_chars: 2000,
        }
    }
}

impl RagContextBuilder {
    /// يبني context string من الذكريات ذات الصلة بالرسالة الحالية
    pub async fn build_context<S: RagMemoryStore>(
        &self,
        store: &S,
        user_id: &str,
        current_message: &str,
    ) -> String {
        let results = match store
            .search_memories(user_id, current_message, self.max_memories)
            .await
        {
            Ok(r) => r,
            Err(e) => {
                warn!("Failed to search RAG memories: {}", e);
                return String::new();
            }
        };

        // فلترة بالـ similarity threshold
        let relevant: Vec<_> = results
            .into_iter()
            .filter(|r| r.similarity >= self.min_similarity)
            .collect();

        if relevant.is_empty() {
            debug!("No relevant memories found for user {}", user_id);
            return String::new();
        }

        info!(
            user_id = %user_id,
            count = relevant.len(),
            "Injecting RAG memories into context"
        );

        // بناء الـ context string
        let mut context = String::from("## ذكريات ذات صلة من محادثات سابقة:\n\n");
        let mut total_chars = context.len();

        for (i, result) in relevant.iter().enumerate() {
            let entry = format!(
                "{}. [{}] {}\n",
                i + 1,
                format_memory_type(&result.memory.memory_type),
                result.memory.content
            );

            if total_chars + entry.len() > self.max_context_chars {
                break;
            }

            context.push_str(&entry);
            total_chars += entry.len();
        }

        context.push_str("\n---\n\n");
        context
    }

    /// يستخرج ذكريات جديدة من محادثة منتهية ويحفظها
    pub async fn extract_and_save_memories<S: RagMemoryStore>(
        &self,
        store: &S,
        user_id: &str,
        conversation: &[ConversationTurn],
    ) {
        // استخراج الحقائق والتفضيلات من المحادثة
        let memories = extract_memories_from_conversation(conversation);

        for mem in memories {
            match store
                .save_memory(SaveMemoryRequest {
                    user_id: user_id.to_string(),
                    content: mem.content,
                    memory_type: mem.memory_type,
                    importance: mem.importance,
                })
                .await
            {
                Ok(saved) => {
                    debug!("Saved memory {} for user {}", saved.id, user_id);
                }
                Err(e) => {
                    warn!("Failed to save memory for user {}: {}", user_id, e);
                }
            }
        }
    }
}

/// دور في المحادثة
#[derive(Debug, Clone)]
pub struct ConversationTurn {
    pub role: String,   // "user" | "assistant"
    pub content: String,
}

/// ذاكرة مستخرجة (قبل الحفظ)
struct ExtractedMemory {
    content: String,
    memory_type: MemoryType,
    importance: f32,
}

/// يستخرج الذكريات من محادثة (heuristic-based, بدون LLM)
fn extract_memories_from_conversation(turns: &[ConversationTurn]) -> Vec<ExtractedMemory> {
    let mut memories = Vec::new();

    for turn in turns {
        if turn.role != "user" {
            continue;
        }

        let content = &turn.content;

        // تفضيلات: "أفضّل", "أحب", "لا أحب"
        if content.contains("أفضّل") || content.contains("أحب") || content.contains("أريد دائماً") {
            memories.push(ExtractedMemory {
                content: format!("تفضيل المستخدم: {}", truncate(content, 200)),
                memory_type: MemoryType::Preference,
                importance: 0.8,
            });
        }

        // حقائق: "اسمي", "أنا من", "أعمل في"
        if content.contains("اسمي") || content.contains("أنا من") || content.contains("أعمل في") {
            memories.push(ExtractedMemory {
                content: format!("معلومة عن المستخدم: {}", truncate(content, 200)),
                memory_type: MemoryType::Fact,
                importance: 0.9,
            });
        }

        // مهام: "أريد أن", "أحتاج إلى", "ساعدني في"
        if content.len() > 50
            && (content.contains("أريد أن") || content.contains("أحتاج") || content.contains("ساعدني"))
        {
            memories.push(ExtractedMemory {
                content: format!("طلب المستخدم: {}", truncate(content, 150)),
                memory_type: MemoryType::Task,
                importance: 0.6,
            });
        }
    }

    memories
}

fn truncate(s: &str, max_chars: usize) -> &str {
    if s.len() <= max_chars {
        s
    } else {
        &s[..max_chars]
    }
}

fn format_memory_type(t: &MemoryType) -> &'static str {
    match t {
        MemoryType::Fact => "حقيقة",
        MemoryType::Preference => "تفضيل",
        MemoryType::Conversation => "محادثة سابقة",
        MemoryType::Task => "مهمة",
        MemoryType::Skill => "مهارة",
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Mock implementation for testing
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
pub struct MockRagStore {
    pub memories: std::sync::Mutex<Vec<Memory>>,
}

#[cfg(test)]
impl MockRagStore {
    pub fn new() -> Self {
        Self {
            memories: std::sync::Mutex::new(Vec::new()),
        }
    }
}

#[cfg(test)]
#[async_trait::async_trait]
impl RagMemoryStore for MockRagStore {
    async fn search_memories(
        &self,
        user_id: &str,
        _query: &str,
        limit: usize,
    ) -> Result<Vec<MemorySearchResult>, String> {
        let memories = self.memories.lock().unwrap();
        Ok(memories
            .iter()
            .filter(|m| m.user_id == user_id)
            .take(limit)
            .map(|m| MemorySearchResult {
                memory: m.clone(),
                similarity: 0.85,
            })
            .collect())
    }

    async fn save_memory(&self, req: SaveMemoryRequest) -> Result<Memory, String> {
        let memory = Memory {
            id: uuid::Uuid::new_v4().to_string(),
            user_id: req.user_id,
            content: req.content,
            memory_type: req.memory_type,
            importance: req.importance,
            access_count: 0,
            created_at: chrono::Utc::now().to_rfc3339(),
            last_accessed_at: None,
        };
        self.memories.lock().unwrap().push(memory.clone());
        Ok(memory)
    }

    async fn get_recent_memories(
        &self,
        user_id: &str,
        limit: usize,
    ) -> Result<Vec<Memory>, String> {
        let memories = self.memories.lock().unwrap();
        Ok(memories
            .iter()
            .filter(|m| m.user_id == user_id)
            .take(limit)
            .cloned()
            .collect())
    }

    async fn delete_memory(&self, _user_id: &str, memory_id: &str) -> Result<(), String> {
        let mut memories = self.memories.lock().unwrap();
        memories.retain(|m| m.id != memory_id);
        Ok(())
    }

    async fn mark_accessed(&self, _memory_ids: &[String]) -> Result<(), String> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_rag_context_builder_empty_store() {
        let store = MockRagStore::new();
        let builder = RagContextBuilder::default();
        let ctx = builder.build_context(&store, "user-1", "مرحبا").await;
        assert!(ctx.is_empty());
    }

    #[tokio::test]
    async fn test_rag_context_builder_with_memories() {
        let store = MockRagStore::new();
        store
            .save_memory(SaveMemoryRequest {
                user_id: "user-1".into(),
                content: "المستخدم يفضّل الردود المختصرة".into(),
                memory_type: MemoryType::Preference,
                importance: 0.9,
            })
            .await
            .unwrap();

        let builder = RagContextBuilder::default();
        let ctx = builder.build_context(&store, "user-1", "كيف حالك").await;
        assert!(ctx.contains("ذكريات"));
        assert!(ctx.contains("المستخدم يفضّل"));
    }

    #[tokio::test]
    async fn test_extract_preference_memory() {
        let turns = vec![ConversationTurn {
            role: "user".into(),
            content: "أفضّل الردود باللغة العربية دائماً".into(),
        }];
        let memories = extract_memories_from_conversation(&turns);
        assert!(!memories.is_empty());
        assert_eq!(memories[0].memory_type, MemoryType::Preference);
    }

    #[tokio::test]
    async fn test_save_and_retrieve_memory() {
        let store = MockRagStore::new();
        let saved = store
            .save_memory(SaveMemoryRequest {
                user_id: "user-2".into(),
                content: "اسم المستخدم هو أحمد".into(),
                memory_type: MemoryType::Fact,
                importance: 0.95,
            })
            .await
            .unwrap();

        assert!(!saved.id.is_empty());

        let recent = store.get_recent_memories("user-2", 10).await.unwrap();
        assert_eq!(recent.len(), 1);
        assert_eq!(recent[0].content, "اسم المستخدم هو أحمد");
    }
}
