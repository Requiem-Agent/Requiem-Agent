//! # Vector Store — مخزن المتجهات للبحث الدلالي
//!
//! يخزن التضمينات الدلالية ويبحث عنها باستخدام cosine similarity
//! مصمم للعمل في الذاكرة مع دعم التخزين المؤقت

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info};

use super::{MemoryEntry, MemoryType, MemoryPriority};

/// متجه واحد مع بياناته
#[derive(Debug, Clone)]
pub struct VectorEntry {
    pub id: String,
    pub vector: Vec<f32>,
    pub metadata: VectorMetadata,
}

/// بيانات وصفية للمتجه
#[derive(Debug, Clone)]
pub struct VectorMetadata {
    pub user_id: String,
    pub session_id: Option<String>,
    pub memory_type: MemoryType,
    pub priority: MemoryPriority,
    pub content_preview: String,
    pub created_at: String,
}

/// نتيجة البحث
#[derive(Debug, Clone)]
pub struct SearchResult {
    pub entry_id: String,
    pub score: f32,
    pub metadata: VectorMetadata,
}

/// مخزن المتجهات الرئيسي
pub struct VectorStore {
    /// المتجهات المخزنة: user_id -> VectorEntry
    vectors: RwLock<HashMap<String, Vec<VectorEntry>>>,
    /// أبعاد المتجهات
    dimension: usize,
    /// الحد الأقصى للمتجهات لكل مستخدم
    max_vectors_per_user: usize,
}

impl VectorStore {
    /// إنشاء مخزن متجهات جديد
    pub fn new(dimension: usize, max_vectors_per_user: usize) -> Arc<Self> {
        info!("VectorStore initialized: dimension={}, max_per_user={}", dimension, max_vectors_per_user);
        Arc::new(Self {
            vectors: RwLock::new(HashMap::new()),
            dimension,
            max_vectors_per_user,
        })
    }

    /// إضافة متجه جديد
    pub async fn insert(
        &self,
        user_id: &str,
        entry: VectorEntry,
    ) -> Result<(), VectorStoreError> {
        if entry.vector.len() != self.dimension {
            return Err(VectorStoreError::DimensionMismatch {
                expected: self.dimension,
                actual: entry.vector.len(),
            });
        }

        let mut vectors = self.vectors.write().await;
        let user_vectors = vectors.entry(user_id.to_string()).or_insert_with(Vec::new);

        // التحقق من الحد الأقصى
        if user_vectors.len() >= self.max_vectors_per_user {
            // إزالة الأقدم (FIFO)
            user_vectors.remove(0);
        }

        user_vectors.push(entry);
        debug!("Inserted vector for user={}", user_id);
        Ok(())
    }

    /// البحث عن أقرب المتجهات
    pub async fn search(
        &self,
        user_id: &str,
        query: &[f32],
        top_k: usize,
        min_score: f32,
    ) -> Result<Vec<SearchResult>, VectorStoreError> {
        if query.len() != self.dimension {
            return Err(VectorStoreError::DimensionMismatch {
                expected: self.dimension,
                actual: query.len(),
            });
        }

        let vectors = self.vectors.read().await;
        let user_vectors = match vectors.get(user_id) {
            Some(v) => v,
            None => return Ok(Vec::new()),
        };

        // حساب cosine similarity لكل متجه
        let mut results: Vec<SearchResult> = user_vectors
            .iter()
            .filter_map(|entry| {
                let score = cosine_similarity(query, &entry.vector);
                if score >= min_score {
                    Some(SearchResult {
                        entry_id: entry.id.clone(),
                        score,
                        metadata: entry.metadata.clone(),
                    })
                } else {
                    None
                }
            })
            .collect();

        // ترتيب حسب النتيجة (الأعلى أولاً)
        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        results.truncate(top_k);

        debug!("Search returned {} results for user={}", results.len(), user_id);
        Ok(results)
    }

    /// حذف متجه بواسطة المعرف
    pub async fn delete(&self, user_id: &str, entry_id: &str) -> Result<bool, VectorStoreError> {
        let mut vectors = self.vectors.write().await;
        if let Some(user_vectors) = vectors.get_mut(user_id) {
            let initial_len = user_vectors.len();
            user_vectors.retain(|v| v.id != entry_id);
            Ok(user_vectors.len() < initial_len)
        } else {
            Ok(false)
        }
    }

    /// حذف جميع متجهات مستخدم
    pub async fn clear_user(&self, user_id: &str) -> Result<(), VectorStoreError> {
        let mut vectors = self.vectors.write().await;
        vectors.remove(user_id);
        debug!("Cleared all vectors for user={}", user_id);
        Ok(())
    }

    /// عدد المتجهات لكل مستخدم
    pub async fn count(&self, user_id: &str) -> usize {
        let vectors = self.vectors.read().await;
        vectors.get(user_id).map(|v| v.len()).unwrap_or(0)
    }

    /// عدد جميع المتجهات
    pub async fn total_count(&self) -> usize {
        let vectors = self.vectors.read().await;
        vectors.values().map(|v| v.len()).sum()
    }
}

/// حساب cosine similarity بين متجهين
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }

    let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

    if norm_a == 0.0 || norm_b == 0.0 {
        0.0
    } else {
        dot_product / (norm_a * norm_b)
    }
}

/// أخطاء مخزن المتجهات
#[derive(Debug, thiserror::Error)]
pub enum VectorStoreError {
    #[error("Dimension mismatch: expected {expected}, got {actual}")]
    DimensionMismatch { expected: usize, actual: usize },

    #[error("Vector not found: {0}")]
    NotFound(String),

    #[error("Storage error: {0}")]
    StorageError(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_vector_store_insert_and_search() {
        let store = VectorStore::new(4, 100);

        let entry = VectorEntry {
            id: "test1".to_string(),
            vector: vec![1.0, 0.0, 0.0, 0.0],
            metadata: VectorMetadata {
                user_id: "user1".to_string(),
                session_id: None,
                memory_type: MemoryType::ShortTerm,
                priority: MemoryPriority::Medium,
                content_preview: "Test".to_string(),
                created_at: chrono::Utc::now().to_rfc3339(),
            },
        };

        store.insert("user1", entry).await.unwrap();

        let results = store.search("user1", &[1.0, 0.0, 0.0, 0.0], 10, 0.5).await.unwrap();
        assert_eq!(results.len(), 1);
        assert!(results[0].score > 0.9);
    }

    #[test]
    fn test_cosine_similarity() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        assert!((cosine_similarity(&a, &b) - 1.0).abs() < 0.001);

        let c = vec![0.0, 1.0, 0.0];
        assert!((cosine_similarity(&a, &c) - 0.0).abs() < 0.001);
    }
}
