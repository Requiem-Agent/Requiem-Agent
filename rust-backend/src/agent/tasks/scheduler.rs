//! # TaskScheduler — جدولة المهام وتوزيعها
//!
//! يدير تنفيذ المهام من TaskTree مع:
//! - تحديد أولويات المهام
//! - توزيع المهام على النماذج المتاحة
//! - إعادة المحاولة عند الفشل
//! - كشف المهام العالقة وإعادة توزيعها

use crate::agent::tasks::tree::{TaskTree, TaskStatus, Priority, TaskNode};
use std::collections::HashMap;
use std::time::Duration;

/// إعدادات الجدولة
#[derive(Debug, Clone)]
pub struct SchedulerConfig {
    pub max_concurrent: usize,
    pub retry_limit: u32,
    pub retry_delay_ms: u64,
    pub enable_auto_redistribute: bool,
    pub max_task_duration_minutes: u32,
}

impl Default for SchedulerConfig {
    fn default() -> Self {
        Self {
            max_concurrent: 5,
            retry_limit: 3,
            retry_delay_ms: 2000,
            enable_auto_redistribute: true,
            max_task_duration_minutes: 60,
        }
    }
}

/// حدث في دورة حياة المهمة
#[derive(Debug, Clone)]
pub enum TaskEvent {
    Started(String),
    Completed(String),
    Failed(String, String),
    Blocked(String, String),
    Retrying(String, u32),
    Reassigned(String, String),  // task_id, new_model
}

/// الجدولة
pub struct TaskScheduler {
    config: SchedulerConfig,
    events: Vec<TaskEvent>,
    task_retries: HashMap<String, u32>,
    task_started: HashMap<String, std::time::Instant>,
}

impl TaskScheduler {
    pub fn new(config: SchedulerConfig) -> Self {
        Self {
            config,
            events: Vec::new(),
            task_retries: HashMap::new(),
            task_started: HashMap::new(),
        }
    }

    /// بدء تنفيذ مهمة
    pub fn start_task(&mut self, task_id: &str, tree: &mut TaskTree) -> Result<(), String> {
        let task = tree.nodes.get(task_id)
            .ok_or_else(|| format!("المهمة {} غير موجودة", task_id))?;

        if task.status.is_final() {
            return Err(format!("المهمة {} انتهت بالفعل", task_id));
        }

        tree.update_status(task_id, TaskStatus::InProgress)?;
        self.task_started.insert(task_id.to_string(), std::time::Instant::now());
        self.events.push(TaskEvent::Started(task_id.to_string()));
        Ok(())
    }

    /// إكمال مهمة بنجاح
    pub fn complete_task(&mut self, task_id: &str, result: serde_json::Value, tree: &mut TaskTree) -> Result<(), String> {
        tree.update_status(task_id, TaskStatus::Completed)?;
        if let Some(task) = tree.nodes.get_mut(task_id) {
            task.result = Some(result);
        }
        self.task_started.remove(task_id);
        self.task_retries.remove(task_id);
        self.events.push(TaskEvent::Completed(task_id.to_string()));
        Ok(())
    }

    /// فشل مهمة — مع إعادة محاولة
    pub fn fail_task(&mut self, task_id: &str, error: &str, tree: &mut TaskTree) -> Result<(), String> {
        let retries = self.task_retries.get(task_id).copied().unwrap_or(0);

        if retries < self.config.retry_limit {
            // إعادة محاولة
            self.task_retries.insert(task_id.to_string(), retries + 1);
            tree.update_status(task_id, TaskStatus::Pending)?;
            self.events.push(TaskEvent::Retrying(task_id.to_string(), retries + 1));
            Ok(())
        } else {
            // فشل نهائي
            tree.update_status(task_id, TaskStatus::Failed(error.to_string()))?;
            self.task_started.remove(task_id);
            self.events.push(TaskEvent::Failed(task_id.to_string(), error.to_string()));
            Ok(())
        }
    }

    /// إعادة توزيع المهام العالقة على نماذج أخرى
    pub fn redistribute_blocked(&mut self, tree: &mut TaskTree) -> Vec<String> {
        let mut reassigned = Vec::new();

        if !self.config.enable_auto_redistribute {
            return reassigned;
        }

        let blocked: Vec<String> = tree.nodes.values()
            .filter(|t| matches!(t.status, TaskStatus::Failed(_)))
            .map(|t| t.id.clone())
            .collect();

        for task_id in blocked {
            if let Some(task) = tree.nodes.get_mut(&task_id) {
                if let Some(current_model) = &task.assigned_model {
                    // جرب نموذج آخر
                    let other_model = match current_model.as_str() {
                        "deepseek-v4-flash-free" => "hy3-free",
                        "hy3-free" => "nemotron-3-ultra-free",
                        _ => "deepseek-v4-flash-free",
                    };
                    task.assigned_model = Some(other_model.to_string());
                    task.status = TaskStatus::Pending;
                    task.error = None;
                    task.completed_at = None;
                    reassigned.push(task_id.clone());
                    self.events.push(TaskEvent::Reassigned(task_id, other_model.to_string()));
                }
            }
        }

        reassigned
    }

    /// المهام الجاهزة حسب الأولوية
    pub fn prioritized_ready_tasks<'a>(&self, tree: &'a TaskTree) -> Vec<&'a TaskNode> {
        let mut ready = tree.ready_tasks();
        ready.sort_by(|a, b| {
            b.priority.score().cmp(&a.priority.score())
                .then_with(|| a.created_at.cmp(&b.created_at))
        });
        ready
    }

    /// الأحداث الأخيرة
    pub fn recent_events(&self, n: usize) -> Vec<&TaskEvent> {
        self.events.iter().rev().take(n).collect()
    }

    /// إحصائيات
    pub fn stats(&self) -> serde_json::Value {
        let started = self.events.iter().filter(|e| matches!(e, TaskEvent::Started(_))).count();
        let completed = self.events.iter().filter(|e| matches!(e, TaskEvent::Completed(_))).count();
        let failed = self.events.iter().filter(|e| matches!(e, TaskEvent::Failed(_, _))).count();
        let retried = self.events.iter().filter(|e| matches!(e, TaskEvent::Retrying(_, _))).count();

        serde_json::json!({
            "started": started,
            "completed": completed,
            "failed": failed,
            "retried": retried,
            "active": self.task_started.len(),
            "config": {
                "max_concurrent": self.config.max_concurrent,
                "retry_limit": self.config.retry_limit,
            }
        })
    }
}

// ─── اختبارات ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scheduler_flow() {
        let mut tree = TaskTree::new("الرئيسية", "user1");
        let task_id = tree.add_task("مهمة", Some(&tree.root_id)).unwrap();
        let mut scheduler = TaskScheduler::new(SchedulerConfig::default());

        scheduler.start_task(&task_id, &mut tree).unwrap();
        assert_eq!(tree.nodes[&task_id].status, TaskStatus::InProgress);

        scheduler.complete_task(&task_id, serde_json::json!({"done": true}), &mut tree).unwrap();
        assert_eq!(tree.nodes[&task_id].status, TaskStatus::Completed);
    }

    #[test]
    fn test_retry_on_failure() {
        let mut tree = TaskTree::new("الرئيسية", "user1");
        let task_id = tree.add_task("مهمة", Some(&tree.root_id)).unwrap();
        let mut scheduler = TaskScheduler::new(SchedulerConfig {
            retry_limit: 2, ..Default::default()
        });

        scheduler.start_task(&task_id, &mut tree).unwrap();
        scheduler.fail_task(&task_id, "خطأ مؤقت", &mut tree).unwrap();
        // بعد أول فشل، يجب أن تعود المهمة إلى Pending
        assert_eq!(tree.nodes[&task_id].status, TaskStatus::Pending);

        scheduler.start_task(&task_id, &mut tree).unwrap();
        scheduler.fail_task(&task_id, "خطأ مرة أخرى", &mut tree).unwrap();
        assert_eq!(tree.nodes[&task_id].status, TaskStatus::Pending);

        // المحاولة الثالثة تفشل → نهائي
        scheduler.start_task(&task_id, &mut tree).unwrap();
        scheduler.fail_task(&task_id, "فشل نهائي", &mut tree).unwrap();
        assert!(matches!(tree.nodes[&task_id].status, TaskStatus::Failed(_)));
    }

    #[test]
    fn test_prioritization() {
        let mut tree = TaskTree::new("الرئيسية", "user1");
        let low = tree.add_task("منخفض", Some(&tree.root_id)).unwrap();
        let high = tree.add_task("مرتفع", Some(&tree.root_id)).unwrap();

        if let Some(task) = tree.nodes.get_mut(&high) {
            task.priority = Priority::High;
        }

        let scheduler = TaskScheduler::new(SchedulerConfig::default());
        let ready = scheduler.prioritized_ready_tasks(&tree);
        // المهمة ذات الأولوية العليا أولاً
        assert_eq!(ready[0].id, high);
    }
}
