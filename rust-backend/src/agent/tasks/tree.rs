//! # TaskTree — شجرة المهام الهرمية
//!
//! ## مثال
//! ```rust
//! let mut tree = TaskTree::new("إنشاء تطبيق ويب", "user-1");
//! let backend = tree.add_task("بناء الـ Backend", Some(tree.root_id)).unwrap();
//! let api = tree.add_task("تصميم API", Some(backend)).unwrap();
//! let frontend = tree.add_task("بناء الواجهة", Some(tree.root_id)).unwrap();
//! // api يعتمد على backend
//! tree.add_dependency(&api, &backend).unwrap();
//! ```

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// حالة المهمة
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TaskStatus {
    Pending,
    InProgress,
    Completed,
    Blocked(String),
    Failed(String),
    Cancelled,
}

impl TaskStatus {
    pub fn is_active(&self) -> bool {
        matches!(self, Self::Pending | Self::InProgress)
    }
    pub fn is_final(&self) -> bool {
        matches!(self, Self::Completed | Self::Failed(_) | Self::Cancelled)
    }
    pub fn name(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::InProgress => "in_progress",
            Self::Completed => "completed",
            Self::Blocked(_) => "blocked",
            Self::Failed(_) => "failed",
            Self::Cancelled => "cancelled",
        }
    }
}

/// أولوية المهمة
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Priority {
    Low,
    Medium,
    High,
    Critical,
}

impl Priority {
    pub fn score(&self) -> u8 {
        match self {
            Self::Low => 1,
            Self::Medium => 3,
            Self::High => 5,
            Self::Critical => 8,
        }
    }
}

/// عقدة مهمة واحدة
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskNode {
    pub id: String,
    pub content: String,
    pub status: TaskStatus,
    pub priority: Priority,
    pub effort_estimate: String,     // "low", "medium", "high", "max"
    pub parent_id: Option<String>,
    pub children: Vec<String>,
    pub depends_on: Vec<String>,      // IDs of tasks this depends on
    pub depended_by: Vec<String>,     // IDs of tasks that depend on this
    pub assigned_model: Option<String>,
    pub sub_agent_id: Option<String>,
    pub result: Option<serde_json::Value>,
    pub error: Option<String>,
    pub created_at: String,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
    pub tags: Vec<String>,
}

impl TaskNode {
    fn new(id: String, content: String, parent_id: Option<String>) -> Self {
        Self {
            id,
            content,
            status: TaskStatus::Pending,
            priority: Priority::Medium,
            effort_estimate: "medium".into(),
            parent_id,
            children: vec![],
            depends_on: vec![],
            depended_by: vec![],
            assigned_model: None,
            sub_agent_id: None,
            result: None,
            error: None,
            created_at: chrono::Utc::now().to_rfc3339(),
            started_at: None,
            completed_at: None,
            tags: vec![],
        }
    }
}

/// شجرة المهام
pub struct TaskTree {
    pub root_id: String,
    pub nodes: HashMap<String, TaskNode>,
    next_id: u64,
}

impl TaskTree {
    /// إنشاء شجرة مهام جديدة بمهمة جذر
    pub fn new(root_content: &str, _owner: &str) -> Self {
        let root_id = "task-root".to_string();
        let mut nodes = HashMap::new();
        nodes.insert(root_id.clone(), TaskNode::new(
            root_id.clone(), root_content.to_string(), None,
        ));
        Self { root_id, nodes, next_id: 1 }
    }

    /// إضافة مهمة فرعية
    pub fn add_task(&mut self, content: &str, parent_id: Option<&str>) -> Result<String, String> {
        let pid = parent_id.unwrap_or(&self.root_id);
        if !self.nodes.contains_key(pid) {
            return Err(format!("المهمة الأم {} غير موجودة", pid));
        }

        let id = format!("task-{}", self.next_id);
        self.next_id += 1;

        let node = TaskNode::new(id.clone(), content.to_string(), Some(pid.to_string()));
        self.nodes.insert(id.clone(), node);

        // ربط بالوالد
        if let Some(parent) = self.nodes.get_mut(pid) {
            parent.children.push(id.clone());
        }

        Ok(id)
    }

    /// إضافة اعتماد: task تعتمد على depends_on
    pub fn add_dependency(&mut self, task_id: &str, depends_on_id: &str) -> Result<(), String> {
        if !self.nodes.contains_key(task_id) {
            return Err(format!("المهمة {} غير موجودة", task_id));
        }
        if !self.nodes.contains_key(depends_on_id) {
            return Err(format!("المهمة {} غير موجودة", depends_on_id));
        }
        if task_id == depends_on_id {
            return Err("المهمة لا يمكن أن تعتمد على نفسها".into());
        }

        if let Some(task) = self.nodes.get_mut(task_id) {
            if !task.depends_on.contains(&depends_on_id.to_string()) {
                task.depends_on.push(depends_on_id.to_string());
            }
        }
        if let Some(dep) = self.nodes.get_mut(depends_on_id) {
            if !dep.depended_by.contains(&task_id.to_string()) {
                dep.depended_by.push(task_id.to_string());
            }
        }

        Ok(())
    }

    /// تحديث حالة مهمة
    pub fn update_status(&mut self, task_id: &str, status: TaskStatus) -> Result<(), String> {
        let task = self.nodes.get_mut(task_id)
            .ok_or_else(|| format!("المهمة {} غير موجودة", task_id))?;

        task.status = status.clone();

 match &status {
            TaskStatus::InProgress => {
                task.started_at = Some(chrono::Utc::now().to_rfc3339());
            }
            TaskStatus::Completed => {
                task.completed_at = Some(chrono::Utc::now().to_rfc3339());
                // تحقق من أن كل المهام التابعة جاهزة
                self.unblock_children(task_id);
            }
            TaskStatus::Failed(e) => {
                task.error = Some(e.clone());
                task.completed_at = Some(chrono::Utc::now().to_rfc3339());
                // أبلغ المهام التابعة
                self.block_dependents(task_id, &format!("المهمة الأم فشلت: {e}"));
            }
            _ => {}
        }

        Ok(())
    }

    /// تعيين نموذج لمهمة
    pub fn assign_model(&mut self, task_id: &str, model_id: &str) -> Result<(), String> {
        let task = self.nodes.get_mut(task_id)
            .ok_or_else(|| format!("المهمة {} غير موجودة", task_id))?;
        task.assigned_model = Some(model_id.to_string());
        Ok(())
    }

    /// تعيين وكيل فرعي لمهمة
    pub fn assign_sub_agent(&mut self, task_id: &str, sub_agent_id: &str) -> Result<(), String> {
        let task = self.nodes.get_mut(task_id)
            .ok_or_else(|| format!("المهمة {} غير موجودة", task_id))?;
        task.sub_agent_id = Some(sub_agent_id.to_string());
        Ok(())
    }

    /// إلغاء قفل المهام التابعة عند اكتمال مهمة
    fn unblock_children(&mut self, task_id: &str) {
        let dependents: Vec<String> = self.nodes.get(task_id)
            .map(|t| t.depended_by.clone())
            .unwrap_or_default();

        for dep_id in dependents {
            if let Some(dep) = self.nodes.get(&dep_id) {
                if matches!(&dep.status, TaskStatus::Blocked(reason) if reason.contains("تنتظر")) {
                    let _ = self.update_status(&dep_id, TaskStatus::Pending);
                }
            }
        }
    }

    /// حظر المهام التابعة عند فشل مهمة
    fn block_dependents(&mut self, task_id: &str, reason: &str) {
        let dependents: Vec<String> = self.nodes.get(task_id)
            .map(|t| t.depended_by.clone())
            .unwrap_or_default();

        for dep_id in dependents {
            let _ = self.update_status(&dep_id, TaskStatus::Blocked(reason.to_string()));
        }
    }

    /// المهام الجاهزة للتنفيذ (Pending وليس لها اعتماديات غير مكتملة)
    pub fn ready_tasks(&self) -> Vec<&TaskNode> {
        self.nodes.values().filter(|task| {
            if task.status != TaskStatus::Pending { return false; }
            task.depends_on.iter().all(|dep_id| {
                self.nodes.get(dep_id).map(|d| d.status == TaskStatus::Completed).unwrap_or(false)
            })
        }).collect()
    }

    /// تقرير التقدم
    pub fn progress_report(&self) -> TaskProgressReport {
        let total = self.nodes.len();
        let completed = self.nodes.values().filter(|t| t.status == TaskStatus::Completed).count();
        let in_progress = self.nodes.values().filter(|t| t.status == TaskStatus::InProgress).count();
        let blocked = self.nodes.values().filter(|t| matches!(t.status, TaskStatus::Blocked(_))).count();
        let failed = self.nodes.values().filter(|t| matches!(t.status, TaskStatus::Failed(_))).count();

        TaskProgressReport {
            total,
            completed,
            in_progress,
            blocked,
            failed,
            percent: if total > 0 { completed as f64 / total as f64 * 100.0 } else { 0.0 },
            root_id: self.root_id.clone(),
        }
    }

    /// تحويل الشجرة إلى JSON
    pub fn to_json(&self) -> serde_json::Value {
        let nodes: Vec<serde_json::Value> = self.nodes.values().map(|node| {
            serde_json::json!({
                "id": node.id,
                "content": node.content,
                "status": node.status.name(),
                "priority": format!("{:?}", node.priority),
                "parent_id": node.parent_id,
                "children": node.children,
                "depends_on": node.depends_on,
                "assigned_model": node.assigned_model,
                "progress": self.progress_report(),
            })
        }).collect();

        serde_json::json!({
            "root_id": self.root_id,
            "total": self.nodes.len(),
            "tree": nodes,
            "progress": self.progress_report(),
        })
    }
}

/// تقرير التقدم
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskProgressReport {
    pub total: usize,
    pub completed: usize,
    pub in_progress: usize,
    pub blocked: usize,
    pub failed: usize,
    pub percent: f64,
    pub root_id: String,
}

// ─── اختبارات ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_tree() {
        let tree = TaskTree::new("المهمة الرئيسية", "user1");
        assert_eq!(tree.nodes.len(), 1);
        assert_eq!(tree.root_id, "task-root");
    }

    #[test]
    fn test_add_task() {
        let mut tree = TaskTree::new("الرئيسية", "user1");
        let id = tree.add_task("مهمة فرعية", Some(&tree.root_id)).unwrap();
        assert_eq!(tree.nodes.len(), 2);
        assert_eq!(tree.nodes[&id].parent_id, Some(tree.root_id.clone()));
    }

    #[test]
    fn test_dependency() {
        let mut tree = TaskTree::new("الرئيسية", "user1");
        let a = tree.add_task("المهمة أ", Some(&tree.root_id)).unwrap();
        let b = tree.add_task("المهمة ب", Some(&tree.root_id)).unwrap();
        tree.add_dependency(&b, &a).unwrap();
        assert!(tree.nodes[&b].depends_on.contains(&a));

        // ب غير جاهزة لأن أ لم تكتمل
        let ready: Vec<&TaskNode> = tree.ready_tasks();
        assert!(ready.iter().all(|t| t.id != b));

        // أكمل أ
        tree.update_status(&a, TaskStatus::Completed).unwrap();
        let ready = tree.ready_tasks();
        assert!(ready.iter().any(|t| t.id == b));
    }

    #[test]
    fn test_progress() {
        let mut tree = TaskTree::new("الرئيسية", "user1");
        let a = tree.add_task("أ", Some(&tree.root_id)).unwrap();
        tree.update_status(&a, TaskStatus::Completed).unwrap();
        let progress = tree.progress_report();
        assert_eq!(progress.total, 2);
        assert_eq!(progress.completed, 1);
        assert_eq!(progress.percent, 50.0);
    }

    #[test]
    fn test_block_on_failure() {
        let mut tree = TaskTree::new("الرئيسية", "user1");
        let a = tree.add_task("أ", Some(&tree.root_id)).unwrap();
        let b = tree.add_task("ب", Some(&tree.root_id)).unwrap();
        tree.add_dependency(&b, &a).unwrap();
        tree.update_status(&a, TaskStatus::Failed("خطأ".into())).unwrap();

        // ب يجب أن تكون blocked
        assert!(matches!(tree.nodes[&b].status, TaskStatus::Blocked(_)));
    }
}
