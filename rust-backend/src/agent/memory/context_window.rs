//! # Context Window — إدارة نافذة السياق
//!
//! يدير نافذة السياق الكبيرة (4M توكن) ويتحكم في ضخ السياق
//! في كل طلب بناءً على أعلى النتائج من RAG

use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use tracing::{debug, info};

/// نافذة السياق — تحدد حدود السياق المدخل
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextWindow {
    /// الحد الأقصى للتوكنات في النافذة
    pub max_tokens: usize,
    /// الحد الأقصى للتوكنات المُضخّة في كل طلب
    pub injection_limit: usize,
    /// الحد الأدنى للتوكنات المتاحة
    pub min_free_tokens: usize,
    /// نسبة التخزين المؤقت
    pub cache_ratio: f32,
}

impl Default for ContextWindow {
    fn default() -> Self {
        Self {
            max_tokens: 4_000_000,    // 4M توكن
            injection_limit: 200_000, // 200K لكل طلب
            min_free_tokens: 100_000, // 100K احتياطي
            cache_ratio: 0.8,         // 80% للتخزين المؤقت
        }
    }
}

impl ContextWindow {
    /// إنشاء نافذة سياق جديدة
    pub fn new(max_tokens: usize, injection_limit: usize) -> Self {
        Self {
            max_tokens,
            injection_limit,
            min_free_tokens: max_tokens / 40, // 2.5%
            cache_ratio: 0.8,
        }
    }

    /// حساب مساحة السياق المتاحة
    pub fn available_tokens(&self, used_tokens: usize) -> usize {
        self.max_tokens
            .saturating_sub(used_tokens + self.min_free_tokens)
    }

    /// هل يمكن إضافة سياق إضافي؟
    pub fn can_inject(&self, used_tokens: usize) -> bool {
        self.available_tokens(used_tokens) > 0
    }

    /// حساب عدد التوكنات المسموح بها للإدخال
    pub fn injection_budget(&self, used_tokens: usize) -> usize {
        let available = self.available_tokens(used_tokens);
        available.min(self.injection_limit)
    }
}

/// إدارة تدفق السياق — يتحكم في إدخال السياق من مصادر متعددة
pub struct ContextManager {
    /// نافذة السياق
    window: ContextWindow,
    /// السياق الحالي المستخدم
    current_usage: usize,
    /// ذاكرة التخزين المؤقت
    cache: VecDeque<CachedContext>,
    /// الحد الأقصى للعناصر في ذاكرة التخزين المؤقت
    max_cache_size: usize,
}

/// سياق مؤقت
#[derive(Debug, Clone)]
pub struct CachedContext {
    pub key: String,
    pub content: String,
    pub tokens: usize,
    pub priority: u32,
    pub last_used: std::time::Instant,
}

impl ContextManager {
    /// إنشاء مدير سياق جديد
    pub fn new(window: ContextWindow) -> Self {
        Self {
            window,
            current_usage: 0,
            cache: VecDeque::new(),
            max_cache_size: 1000,
        }
    }

    /// تعيين الاستخدام الحالي
    pub fn set_usage(&mut self, tokens: usize) {
        self.current_usage = tokens;
    }

    /// إضافة سياق إلى الذاكرة المؤقتة
    pub fn cache_context(&mut self, key: &str, content: &str, tokens: usize, priority: u32) {
        // التحقق من عدم التكرار
        if self.cache.iter().any(|c| c.key == key) {
            return;
        }

        // التحقق من المساحة
        if self.cache.len() >= self.max_cache_size {
            self.cache.pop_front();
        }

        self.cache.push_back(CachedContext {
            key: key.to_string(),
            content: content.to_string(),
            tokens,
            priority,
            last_used: std::time::Instant::now(),
        });
    }

    /// جلب سياق من التخزين المؤقت
    pub fn get_cached(&mut self, key: &str) -> Option<String> {
        if let Some(ctx) = self.cache.iter_mut().find(|c| c.key == key) {
            ctx.last_used = std::time::Instant::now();
            Some(ctx.content.clone())
        } else {
            None
        }
    }

    /// حساب ميزانية السياق للمهمة الحالية
    pub fn calculate_budget(&self, task_tokens: usize) -> ContextBudget {
        let available = self.window.available_tokens(self.current_usage);
        let injection_budget = self.window.injection_budget(self.current_usage);

        ContextBudget {
            total_window: self.window.max_tokens,
            used_tokens: self.current_usage,
            available_tokens: available,
            injection_budget,
            task_tokens,
            free_after_task: available.saturating_sub(task_tokens),
        }
    }

    /// تحسين السياق — إزالة العناصر الأقل أهمية
    pub fn optimize_context(&mut self, entries: &mut Vec<ContextEntry>) {
        // ترتيب حسب الأهمية
        entries.sort_by(|a, b| {
            b.importance
                .partial_cmp(&a.importance)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // حساب إجمالي التوكنات
        let total: usize = entries.iter().map(|e| e.tokens).sum();

        // إذا تجاوز الحد، أزل العناصر الأقل أهمية
        if total > self.window.injection_limit {
            let mut remaining = self.window.injection_limit;
            entries.retain(|e| {
                if e.tokens <= remaining {
                    remaining -= e.tokens;
                    true
                } else {
                    false
                }
            });
        }
    }

    /// إحصائيات مدير السياق
    pub fn stats(&self) -> ContextManagerStats {
        ContextManagerStats {
            window_size: self.window.max_tokens,
            current_usage: self.current_usage,
            cache_size: self.cache.len(),
            available: self.window.available_tokens(self.current_usage),
        }
    }
}

/// ميزانية السياق المحسوبة
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextBudget {
    pub total_window: usize,
    pub used_tokens: usize,
    pub available_tokens: usize,
    pub injection_budget: usize,
    pub task_tokens: usize,
    pub free_after_task: usize,
}

/// عنصر سياق
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextEntry {
    pub content: String,
    pub tokens: usize,
    pub importance: f32,
    pub source: String,
}

/// إحصائيات مدير السياق
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextManagerStats {
    pub window_size: usize,
    pub current_usage: usize,
    pub cache_size: usize,
    pub available: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_window_budget() {
        let window = ContextWindow::default();
        let budget = window.injection_budget(1_000_000);
        assert!(budget > 0);
        assert!(budget <= 200_000);
    }

    #[test]
    fn test_context_manager_optimize() {
        let mut manager = ContextManager::new(ContextWindow::default());
        let mut entries = vec![
            ContextEntry {
                content: "Low importance".to_string(),
                tokens: 1000,
                importance: 0.3,
                source: "test".to_string(),
            },
            ContextEntry {
                content: "High importance".to_string(),
                tokens: 1000,
                importance: 0.9,
                source: "test".to_string(),
            },
        ];

        manager.optimize_context(&mut entries);
        assert_eq!(entries.len(), 2);
        assert!(entries[0].importance > entries[1].importance);
    }
}
