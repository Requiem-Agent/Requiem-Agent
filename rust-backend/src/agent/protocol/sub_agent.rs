//! # Sub-Agent Protocol — إنشاء وتنسيق الوكلاء الفرعيين
//!
//! يسمح للوكيل الرئيسي بتفويض المهام لوكلاء فرعيين معزولين:
//! - كل وكيل فرعي له مساحة عمل معزولة
//! - صلاحيات وأدوات محددة
//! - إخراج متوقع (Output Schema)
//! - إمكانية المراقبة والإلغاء

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use crate::agent::protocol::mode::AgentMode;
use crate::tools::JsonSchema;

/// مستوى العزل للوكيل الفرعي
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IsolationLevel {
    /// يرى كل شيء — نفس سياق الوكيل الأب
    Full,
    /// يرى فقط المهمة المسندة إليه
    TaskOnly,
    /// معزول تماماً — يبدأ من الصفر
    Isolated,
}

/// مواصفات الوكيل الفرعي
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubAgentSpec {
    pub id: String,
    pub task: String,
    pub model_id: String,
    pub mode: AgentMode,
    pub tools: Vec<String>,
    pub max_steps: usize,
    pub output_schema: JsonSchema,
    pub parent_id: Option<String>,
    pub isolation: IsolationLevel,
    pub context: Option<serde_json::Value>,
    pub priority: u8, // 0-10
    pub timeout_minutes: u32,
}

/// حالة الوكيل الفرعي
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SubAgentStatus {
    Pending,
    Spawning,
    Running { started_at: String },
    Completed { result: serde_json::Value },
    Failed { error: String },
    Cancelled { reason: String },
    TimedOut,
}

/// تقدم الوكيل الفرعي
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubAgentProgress {
    pub id: String,
    pub status: SubAgentStatus,
    pub steps_taken: usize,
    pub max_steps: usize,
    pub duration_ms: u64,
    pub tokens_used: u32,
    pub last_action: Option<String>,
    pub output_so_far: Option<serde_json::Value>,
}

/// خطأ في الـ spawn
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpawnError {
    pub code: SpawnErrorCode,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SpawnErrorCode {
    MaxChildrenReached,
    InvalidModel,
    InvalidTools,
    InvalidSchema,
    Timeout,
    Internal(String),
}

// ─── Sub-Agent Orchestrator ────────────────────────────────────────────────

/// مقبض الوكيل الفرعي — يمثل وكيلاً قيد التشغيل
#[derive(Debug, Clone)]
pub struct SubAgentHandle {
    pub spec: SubAgentSpec,
    pub status: SubAgentStatus,
    pub started_at: Option<Instant>,
    pub steps_taken: usize,
    pub tokens_used: u32,
}

/// منسق الوكلاء الفرعيين
pub struct SubAgentOrchestrator {
    children: HashMap<String, SubAgentHandle>,
    max_children: usize,
    max_total_tokens: u64,
    total_tokens_used: u64,
    spawn_history: Vec<SpawnEvent>,
}

/// حدث إنشاء وكيل فرعي
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpawnEvent {
    pub child_id: String,
    pub task: String,
    pub model_id: String,
    pub timestamp: String,
    pub completed: bool,
    pub duration_ms: Option<u64>,
}

impl SubAgentOrchestrator {
    pub fn new(max_children: usize) -> Self {
        Self {
            children: HashMap::new(),
            max_children,
            max_total_tokens: 1_000_000,
            total_tokens_used: 0,
            spawn_history: Vec::new(),
        }
    }

    /// إنشاء وكيل فرعي
    pub fn spawn(&mut self, spec: SubAgentSpec) -> Result<String, SpawnError> {
        if self.children.len() >= self.max_children {
            return Err(SpawnError {
                code: SpawnErrorCode::MaxChildrenReached,
                message: format!("بلغت الحد الأقصى للوكلاء الفرعيين: {}", self.max_children),
            });
        }

        let id = spec.id.clone();
        self.children.insert(id.clone(), SubAgentHandle {
            spec,
            status: SubAgentStatus::Spawning,
            started_at: Some(Instant::now()),
            steps_taken: 0,
            tokens_used: 0,
        });

        self.spawn_history.push(SpawnEvent {
            child_id: id.clone(),
            task: String::new(), // يُملأ لاحقاً
            model_id: String::new(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            completed: false,
            duration_ms: None,
        });

        Ok(id)
    }

    /// الحصول على تقدم الوكيل الفرعي
    pub fn get_progress(&self, child_id: &str) -> Option<SubAgentProgress> {
        let handle = self.children.get(child_id)?;
        let duration_ms = handle.started_at
            .map(|t| t.elapsed().as_millis() as u64)
            .unwrap_or(0);

        Some(SubAgentProgress {
            id: child_id.to_string(),
            status: handle.status.clone(),
            steps_taken: handle.steps_taken,
            max_steps: handle.spec.max_steps,
            duration_ms,
            tokens_used: handle.tokens_used,
            last_action: None,
            output_so_far: match &handle.status {
                SubAgentStatus::Completed { result } => Some(result.clone()),
                SubAgentStatus::Running { .. } => Some(serde_json::json!({"status": "running"})),
                _ => None,
            },
        })
    }

    /// إلغاء وكيل فرعي
    pub fn cancel(&mut self, child_id: &str, reason: &str) -> Result<(), String> {
        let handle = self.children.get_mut(child_id)
            .ok_or_else(|| format!("الوكيل الفرعي {} غير موجود", child_id))?;

        match &handle.status {
            SubAgentStatus::Completed { .. }
            | SubAgentStatus::Cancelled { .. }
            | SubAgentStatus::Failed { .. }
            | SubAgentStatus::TimedOut => {
                return Err(format!("الوكيل {} انتهى بالفعل", child_id));
            }
            _ => {
                handle.status = SubAgentStatus::Cancelled {
                    reason: reason.to_string(),
                };
                Ok(())
            }
        }
    }

    /// تحديث حالة وكيل فرعي
    pub fn update_status(&mut self, child_id: &str, status: SubAgentStatus) -> Result<(), String> {
        let handle = self.children.get_mut(child_id)
            .ok_or_else(|| format!("الوكيل الفرعي {} غير موجود", child_id))?;

        // تسجيل الإكمال
        if matches!(&status, SubAgentStatus::Completed { .. } | SubAgentStatus::Failed { .. }) {
            if let Some(event) = self.spawn_history.iter_mut().rev().find(|e| e.child_id == child_id) {
                event.completed = true;
                event.duration_ms = handle.started_at.map(|t| t.elapsed().as_millis() as u64);
            }
        }

        handle.status = status;
        Ok(())
    }

    /// دمج نتائج الوكلاء الفرعيين
    pub fn merge_results(&self, child_ids: &[String]) -> Result<serde_json::Value, String> {
        let mut results = serde_json::Map::new();

        for id in child_ids {
            let handle = self.children.get(id)
                .ok_or_else(|| format!("الوكيل {} غير موجود", id))?;

            let value = match &handle.status {
                SubAgentStatus::Completed { result } => result.clone(),
                SubAgentStatus::Running { .. } => {
                    serde_json::json!({"status": "running", "id": id})
                }
                SubAgentStatus::Failed { error } => {
                    serde_json::json!({"status": "failed", "error": error})
                }
                _ => serde_json::json!({"status": "unknown"}),
            };

            results.insert(id.clone(), value);
        }

        Ok(serde_json::Value::Object(results))
    }

    /// عدد الوكلاء الفرعيين النشطين
    pub fn active_count(&self) -> usize {
        self.children.values().filter(|h| {
            matches!(h.status, SubAgentStatus::Spawning | SubAgentStatus::Running { .. })
        }).count()
    }

    /// قائمة بجميع الوكلاء الفرعيين
    pub fn list_children(&self) -> Vec<serde_json::Value> {
        self.children.iter().map(|(id, handle)| {
            serde_json::json!({
                "id": id,
                "task": handle.spec.task,
                "model": handle.spec.model_id,
                "mode": format!("{:?}", handle.spec.mode),
                "status": format!("{:?}", handle.status),
                "steps": handle.steps_taken,
                "max_steps": handle.spec.max_steps,
            })
        }).collect()
    }

    /// إحصائيات
    pub fn stats(&self) -> serde_json::Value {
        let total = self.children.len();
        let completed = self.children.values().filter(|h| {
            matches!(h.status, SubAgentStatus::Completed { .. })
        }).count();
        let failed = self.children.values().filter(|h| {
            matches!(h.status, SubAgentStatus::Failed { .. } | SubAgentStatus::TimedOut)
        }).count();
        let active = self.active_count();

        serde_json::json!({
            "total": total,
            "active": active,
            "completed": completed,
            "failed": failed,
            "max_children": self.max_children,
            "total_tokens_used": self.total_tokens_used,
        })
    }
}

// ─── اختبارات ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::JsonSchema;

    fn dummy_schema() -> JsonSchema {
        JsonSchema {
            schema_type: "object".into(),
            properties: None,
            required: None,
            description: None,
        }
    }

    #[test]
    fn test_spawn_and_track() {
        let mut orch = SubAgentOrchestrator::new(5);
        let spec = SubAgentSpec {
            id: "child-1".into(),
            task: "تحليل الكود".into(),
            model_id: "deepseek-v4-flash-free".into(),
            mode: AgentMode::Autonomous,
            tools: vec!["code_editor".into(), "shell".into()],
            max_steps: 10,
            output_schema: dummy_schema(),
            parent_id: None,
            isolation: IsolationLevel::TaskOnly,
            context: None,
            priority: 5,
            timeout_minutes: 30,
        };

        let id = orch.spawn(spec).unwrap();
        assert_eq!(id, "child-1");

        let progress = orch.get_progress("child-1").unwrap();
        assert_eq!(progress.status, SubAgentStatus::Spawning);
    }

    #[test]
    fn test_max_children() {
        let mut orch = SubAgentOrchestrator::new(2);
        for i in 0..2 {
            let spec = SubAgentSpec {
                id: format!("child-{}", i),
                task: "task".into(),
                model_id: "deepseek".into(),
                mode: AgentMode::Autonomous,
                tools: vec![],
                max_steps: 5,
                output_schema: dummy_schema(),
                parent_id: None,
                isolation: IsolationLevel::Isolated,
                context: None,
                priority: 1,
                timeout_minutes: 10,
            };
            assert!(orch.spawn(spec).is_ok());
        }

        // الثالث مرفوض
        let spec = SubAgentSpec {
            id: "child-3".into(),
            task: "task".into(),
            model_id: "deepseek".into(),
            mode: AgentMode::Autonomous,
            tools: vec![],
            max_steps: 5,
            output_schema: dummy_schema(),
            parent_id: None,
            isolation: IsolationLevel::Isolated,
            context: None,
            priority: 1,
            timeout_minutes: 10,
        };
        assert!(orch.spawn(spec).is_err());
    }

    #[test]
    fn test_cancel() {
        let mut orch = SubAgentOrchestrator::new(5);
        orch.spawn(SubAgentSpec {
            id: "child-1".into(),
            task: "task".into(),
            model_id: "deepseek".into(),
            mode: AgentMode::Autonomous,
            tools: vec![],
            max_steps: 5,
            output_schema: dummy_schema(),
            parent_id: None,
            isolation: IsolationLevel::Isolated,
            context: None,
            priority: 1,
            timeout_minutes: 10,
        }).unwrap();

        assert!(orch.cancel("child-1", "ألغيت المهمة").is_ok());
        let progress = orch.get_progress("child-1").unwrap();
        assert!(matches!(progress.status, SubAgentStatus::Cancelled { .. }));
    }

    #[test]
    fn test_merge_results() {
        let mut orch = SubAgentOrchestrator::new(5);
        orch.spawn(SubAgentSpec {
            id: "a".into(), task: "A".into(), model_id: "deepseek".into(),
            mode: AgentMode::Autonomous, tools: vec![], max_steps: 5,
            output_schema: dummy_schema(), parent_id: None,
            isolation: IsolationLevel::Isolated, context: None,
            priority: 1, timeout_minutes: 10,
        }).unwrap();
        orch.spawn(SubAgentSpec {
            id: "b".into(), task: "B".into(), model_id: "deepseek".into(),
            mode: AgentMode::Autonomous, tools: vec![], max_steps: 5,
            output_schema: dummy_schema(), parent_id: None,
            isolation: IsolationLevel::Isolated, context: None,
            priority: 1, timeout_minutes: 10,
        }).unwrap();

        orch.update_status("a", SubAgentStatus::Completed {
            result: serde_json::json!({"analysis": "done"}),
        }).unwrap();

        let merged = orch.merge_results(&["a".into(), "b".into()]).unwrap();
        assert_eq!(merged["a"]["analysis"], "done");
    }
}
