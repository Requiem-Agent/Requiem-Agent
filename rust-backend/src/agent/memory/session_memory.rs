//! # Session Memory — ذاكرة الجلسة
//!
//! يدير الذاكرة القصيرة والطويلة لكل جلسة
//! يحفظ السياق والتعلم من المحادثات السابقة

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info};

use super::{MemoryEntry, MemoryType, MemoryPriority, RagEngine};

/// ذاكرة الجلسة — تحتفظ بسياق المحادثة الحالية
pub struct SessionMemory {
    /// معرف الجلسة
    pub session_id: String,
    /// معرف المستخدم
    pub user_id: String,
    /// الذاكرة القصيرة — آخر N رسالة
    short_term: VecDeque<MemoryEntry>,
    /// ذاكرة العمل — معلومات عن المهمة الحالية
    working_memory: HashMap<String, String>,
    /// الحد الأقصى للذاكرة القصيرة
    max_short_term: usize,
    /// إجمالي التوكنات المستخدمة
    total_tokens: usize,
    /// الحد الأقصى للتوكنات
    max_tokens: usize,
}

impl SessionMemory {
    /// إنشاء ذاكرة جلسة جديدة
    pub fn new(session_id: &str, user_id: &str, max_short_term: usize, max_tokens: usize) -> Self {
        info!(
            "SessionMemory initialized: session={}, max_short_term={}, max_tokens={}",
            session_id, max_short_term, max_tokens
        );

        Self {
            session_id: session_id.to_string(),
            user_id: user_id.to_string(),
            short_term: VecDeque::with_capacity(max_short_term),
            working_memory: HashMap::new(),
            max_short_term,
            total_tokens: 0,
            max_tokens,
        }
    }

    /// إضافة رسالة إلى الذاكرة القصيرة
    pub fn add_message(&mut self, role: &str, content: &str, model: Option<&str>) -> bool {
        let estimated_tokens = content.len() / 4;

        // التحقق من المساحة
        if self.total_tokens + estimated_tokens > self.max_tokens {
            debug!("Session memory full, removing oldest entries");
            self.evict_old_entries(estimated_tokens);
        }

        let mut entry = MemoryEntry::new(
            &self.user_id,
            MemoryType::ShortTerm,
            content,
            MemoryPriority::Medium,
        );
        entry.session_id = Some(self.session_id.clone());
        entry.metadata.insert("role".to_string(), role.to_string());
        if let Some(m) = model {
            entry.metadata.insert("model".to_string(), m.to_string());
        }

        self.short_term.push_back(entry);
        self.total_tokens += estimated_tokens;

        // التحقق من الحد الأقصى
        while self.short_term.len() > self.max_short_term {
            if let Some(old) = self.short_term.pop_front() {
                self.total_tokens = self.total_tokens.saturating_sub(old.content.len() / 4);
            }
        }

        true
    }

    /// إزالة العناصر القديمة
    fn evict_old_entries(&mut self, needed_tokens: usize) {
        let mut freed_tokens = 0;
        while freed_tokens < needed_tokens && !self.short_term.is_empty() {
            if let Some(old) = self.short_term.pop_front() {
                freed_tokens += old.content.len() / 4;
            }
        }
        self.total_tokens = self.total_tokens.saturating_sub(freed_tokens);
    }

    /// تحديث ذاكرة العمل
    pub fn set_working(&mut self, key: &str, value: &str) {
        self.working_memory.insert(key.to_string(), value.to_string());
    }

    /// جلب قيمة من ذاكرة العمل
    pub fn get_working(&self, key: &str) -> Option<&str> {
        self.working_memory.get(key).map(|s| s.as_str())
    }

    /// جلب سياق المحادثة للإدخال في البرومبت
    pub fn get_context_for_prompt(&self, max_messages: usize) -> String {
        let messages: Vec<&MemoryEntry> = self.short_term.iter().rev().take(max_messages).collect();
        let mut context = String::new();

        for msg in messages.iter().rev() {
            let role = msg.metadata.get("role").map(|s| s.as_str()).unwrap_or("user");
            context.push_str(&format!("{}: {}\n", role, msg.content));
        }

        context
    }

    /// حفظ الذاكرة في RAG
    pub async fn persist_to_rag(
        &self,
        rag: &Arc<RwLock<RagEngine>>,
        memory_type: MemoryType,
    ) -> Result<(), super::vector_store::VectorStoreError> {
        let rag_engine = rag.read().await;

        for entry in &self.short_term {
            if entry.memory_type == memory_type {
                rag_engine.store(
                    &entry.content,
                    entry.memory_type.name(),
                    match &entry.priority {
                        super::MemoryPriority::Low => "low",
                        super::MemoryPriority::Medium => "medium",
                        super::MemoryPriority::High => "high",
                        super::MemoryPriority::Critical => "critical",
                    },
                    entry.session_id.as_deref(),
                ).await
                    .map(|_| ())
                    .map_err(|e| super::vector_store::VectorStoreError::StorageError(e.to_string()))?;
            }
        }

        Ok(())
    }

    /// إحصائيات الذاكرة
    pub fn stats(&self) -> SessionMemoryStats {
        SessionMemoryStats {
            session_id: self.session_id.clone(),
            short_term_count: self.short_term.len(),
            working_memory_count: self.working_memory.len(),
            total_tokens: self.total_tokens,
            max_tokens: self.max_tokens,
            utilization: self.total_tokens as f32 / self.max_tokens as f32,
        }
    }
}

/// ذاكرة المستخدم طويلة المدى — تستمر عبر الجلسات
pub struct UserMemory {
    pub user_id: String,
    /// الذاكرة الدلالية — مفاهيم وتعلم
    semantic_memory: HashMap<String, String>,
    /// تفضيلات المستخدم
    preferences: HashMap<String, String>,
    /// تاريخ المشاريع
    project_history: Vec<ProjectRecord>,
    /// الحد الأقصى للسجلات
    max_history: usize,
}

/// سجل مشروع
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ProjectRecord {
    pub project_id: String,
    pub name: String,
    pub language: String,
    pub created_at: String,
    pub last_accessed: String,
    pub file_count: usize,
}

impl UserMemory {
    /// إنشاء ذاكرة مستخدم جديدة
    pub fn new(user_id: &str) -> Self {
        Self {
            user_id: user_id.to_string(),
            semantic_memory: HashMap::new(),
            preferences: HashMap::new(),
            project_history: Vec::new(),
            max_history: 100,
        }
    }

    /// تعلم مفهوم جديد
    pub fn learn(&mut self, concept: &str, description: &str) {
        self.semantic_memory.insert(concept.to_string(), description.to_string());
    }

    /// جلب مفهوم
    pub fn recall(&self, concept: &str) -> Option<&str> {
        self.semantic_memory.get(concept).map(|s| s.as_str())
    }

    /// تعيين تفضيل
    pub fn set_preference(&mut self, key: &str, value: &str) {
        self.preferences.insert(key.to_string(), value.to_string());
    }

    /// جلب تفضيل
    pub fn get_preference(&self, key: &str) -> Option<&str> {
        self.preferences.get(key).map(|s| s.as_str())
    }

    /// إضافة مشروع إلى السجل
    pub fn add_project(&mut self, record: ProjectRecord) {
        if self.project_history.len() >= self.max_history {
            self.project_history.remove(0);
        }
        self.project_history.push(record);
    }

    /// جلب آخر المشاريع
    pub fn recent_projects(&self, limit: usize) -> Vec<&ProjectRecord> {
        self.project_history.iter().rev().take(limit).collect()
    }

    /// إحصائيات ذاكرة المستخدم
    pub fn stats(&self) -> UserMemoryStats {
        UserMemoryStats {
            user_id: self.user_id.clone(),
            semantic_concepts: self.semantic_memory.len(),
            preferences_count: self.preferences.len(),
            project_count: self.project_history.len(),
        }
    }
}

/// إحصائيات ذاكرة الجلسة
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SessionMemoryStats {
    pub session_id: String,
    pub short_term_count: usize,
    pub working_memory_count: usize,
    pub total_tokens: usize,
    pub max_tokens: usize,
    pub utilization: f32,
}

/// إحصائيات ذاكرة المستخدم
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct UserMemoryStats {
    pub user_id: String,
    pub semantic_concepts: usize,
    pub preferences_count: usize,
    pub project_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_memory_add_message() {
        let mut mem = SessionMemory::new("session1", "user1", 100, 10000);
        mem.add_message("user", "Hello", None);
        mem.add_message("assistant", "Hi there!", None);

        assert_eq!(mem.short_term.len(), 2);
    }

    #[test]
    fn test_user_memory_learn() {
        let mut mem = UserMemory::new("user1");
        mem.learn("rust", "A systems programming language");
        mem.learn("tokio", "An async runtime for Rust");

        assert_eq!(mem.recall("rust"), Some("A systems programming language"));
        assert_eq!(mem.recall("tokio"), Some("An async runtime for Rust"));
    }

    #[test]
    fn test_user_memory_preferences() {
        let mut mem = UserMemory::new("user1");
        mem.set_preference("theme", "dark");
        mem.set_preference("language", "ar");

        assert_eq!(mem.get_preference("theme"), Some("dark"));
    }
}
