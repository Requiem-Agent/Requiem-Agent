//! # Memory & RAG Module — نظام الذاكرة واسترجاع المعلومات المدعوم بالدلالة
//!
//! ## المكونات
//! - `vector_store.rs` — مخزن المتجهات للبحث الدلالي
//! - `rag.rs` — محرك RAG لاسترجاع السياق ذات الصلة
//! - `session_memory.rs` — ذاكرة الجلسة短期 والطويلة
//! - `embeddings.rs` — توليد التضمينات الدلالية
//! - `context_window.rs` — إدارة نافذة السياق (4M توكن)

pub mod vector_store;
pub mod rag;
pub mod session_memory;
pub mod embeddings;
pub mod context_window;

pub use vector_store::*;
pub use rag::*;
pub use session_memory::*;
pub use embeddings::*;
pub use context_window::*;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// نوع الذاكرة
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum MemoryType {
    /// ذاكرة مؤقتة — خلال الجلسة الحالية فقط
    ShortTerm,
    /// ذاكرة طويلة — تستمر عبر الجلسات
    LongTerm,
    /// ذاكرة عمل — معلومات عن المشاريع والمهام
    Working,
    /// ذاكرة دلالية — معلومات مسترجعة بناءً على التشابه
    Semantic,
}

impl MemoryType {
    pub fn name(&self) -> &str {
        match self {
            Self::ShortTerm => "short_term",
            Self::LongTerm => "long_term",
            Self::Working => "working",
            Self::Semantic => "semantic",
        }
    }
}

/// مستوى الأولوية للذاكرة
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum MemoryPriority {
    Low = 0,
    Medium = 1,
    High = 2,
    Critical = 3,
}

/// عنصر ذاكرة واحد
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    pub id: String,
    pub user_id: String,
    pub session_id: Option<String>,
    pub memory_type: MemoryType,
    pub priority: MemoryPriority,
    pub content: String,
    pub metadata: HashMap<String, String>,
    pub embedding: Option<Vec<f32>>,
    pub created_at: String,
    pub last_accessed: String,
    pub access_count: u32,
    pub ttl_seconds: Option<u64>,
}

impl MemoryEntry {
    pub fn new(
        user_id: &str,
        memory_type: MemoryType,
        content: &str,
        priority: MemoryPriority,
    ) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            user_id: user_id.to_string(),
            session_id: None,
            memory_type,
            priority,
            content: content.to_string(),
            metadata: HashMap::new(),
            embedding: None,
            created_at: now.clone(),
            last_accessed: now,
            access_count: 0,
            ttl_seconds: None,
        }
    }

    /// هل انتهت صلاحية هذا العنصر؟
    pub fn is_expired(&self) -> bool {
        if let Some(ttl) = self.ttl_seconds {
            if let Ok(created) = chrono::DateTime::parse_from_rfc3339(&self.created_at) {
                let elapsed = chrono::Utc::now().signed_duration_since(created);
                elapsed.num_seconds() as u64 > ttl
            } else {
                false
            }
        } else {
            false
        }
    }

    /// تحديث آخر وصول
    pub fn touch(&mut self) {
        self.last_accessed = chrono::Utc::now().to_rfc3339();
        self.access_count += 1;
    }
}

/// سياق الاسترجاع من RAG
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrievalContext {
    pub query: String,
    pub retrieved_memories: Vec<MemoryEntry>,
    pub total_tokens: usize,
    pub max_tokens: usize,
    pub relevance_threshold: f32,
}

impl RetrievalContext {
    pub fn new(query: &str, max_tokens: usize) -> Self {
        Self {
            query: query.to_string(),
            retrieved_memories: Vec::new(),
            total_tokens: 0,
            max_tokens,
            relevance_threshold: 0.7,
        }
    }

    /// إضافة ذاكرة إلى السياق
    pub fn add_memory(&mut self, entry: MemoryEntry) -> bool {
        let estimated_tokens = entry.content.len() / 4; // تقدير تقريبي
        if self.total_tokens + estimated_tokens <= self.max_tokens {
            self.total_tokens += estimated_tokens;
            self.retrieved_memories.push(entry);
            true
        } else {
            false
        }
    }

    /// تحويل إلى نص للإدخال في البرومبت
    pub fn to_context_string(&self) -> String {
        if self.retrieved_memories.is_empty() {
            return String::new();
        }

        let mut context = String::from("## Retrieved Context (RAG)\n\n");
        for (i, memory) in self.retrieved_memories.iter().enumerate() {
            context.push_str(&format!(
                "### {}. [{}] {}\n{}\n\n",
                i + 1,
                memory.memory_type.name(),
                memory.priority.clone() as i32,
                memory.content
            ));
        }
        context
    }
}

/// إحصائيات الذاكرة
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryStats {
    pub total_entries: usize,
    pub by_type: HashMap<String, usize>,
    pub by_priority: HashMap<String, usize>,
    pub total_tokens: usize,
    pub avg_access_count: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_entry_creation() {
        let entry = MemoryEntry::new(
            "user1",
            MemoryType::ShortTerm,
            "Test memory content",
            MemoryPriority::Medium,
        );
        assert_eq!(entry.user_id, "user1");
        assert_eq!(entry.memory_type, MemoryType::ShortTerm);
        assert!(!entry.is_expired());
    }

    #[test]
    fn test_retrieval_context() {
        let mut ctx = RetrievalContext::new("test query", 1000);
        let entry = MemoryEntry::new("user1", MemoryType::LongTerm, "Test", MemoryPriority::High);
        assert!(ctx.add_memory(entry));
        assert_eq!(ctx.retrieved_memories.len(), 1);
    }

    #[test]
    fn test_memory_priority_ordering() {
        assert!(MemoryPriority::Critical > MemoryPriority::High);
        assert!(MemoryPriority::High > MemoryPriority::Medium);
        assert!(MemoryPriority::Medium > MemoryPriority::Low);
    }
}
