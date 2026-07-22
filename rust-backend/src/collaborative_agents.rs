// collaborative_agents.rs — S10-01: Collaborative Agent System
// يسمح لعدة agent instances بالتواصل وتفويض المهام لبعضها
//
// Architecture:
//   AgentBus (mpsc channels) → يُوزّع الرسائل بين الـ agents
//   AgentInstance → كل instance له ID وقدرات محددة
//   TaskDelegation → تفويض مهمة من agent لآخر مع tracking

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, info, warn};
use uuid::Uuid;

// ─────────────────────────────────────────────────────────────────────────────
// Types
// ─────────────────────────────────────────────────────────────────────────────

/// رسالة بين الـ agents
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMessage {
    pub id: String,
    pub from_agent: String,
    pub to_agent: String,
    pub message_type: AgentMessageType,
    pub payload: serde_json::Value,
    pub correlation_id: Option<String>,  // لربط الطلب بالرد
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum AgentMessageType {
    TaskRequest,        // طلب تنفيذ مهمة
    TaskResponse,       // رد على مهمة
    StatusQuery,        // استعلام عن حالة agent
    StatusResponse,     // رد على استعلام الحالة
    Broadcast,          // رسالة لجميع الـ agents
    Heartbeat,          // نبضة حياة
}

/// قدرات الـ agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentCapabilities {
    pub agent_id: String,
    pub name: String,
    pub specializations: Vec<String>,   // ["code", "research", "math", "writing"]
    pub max_concurrent_tasks: usize,
    pub current_load: usize,
    pub is_available: bool,
}

/// مهمة مُفوَّضة
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DelegatedTask {
    pub task_id: String,
    pub from_agent: String,
    pub to_agent: String,
    pub task_type: String,
    pub input: String,
    pub status: TaskStatus,
    pub result: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
    Cancelled,
}

// ─────────────────────────────────────────────────────────────────────────────
// AgentBus: ناقل الرسائل بين الـ agents
// ─────────────────────────────────────────────────────────────────────────────

pub struct AgentBus {
    /// قنوات الإرسال لكل agent
    senders: RwLock<HashMap<String, mpsc::Sender<AgentMessage>>>,
    /// سجل المهام المُفوَّضة
    tasks: RwLock<HashMap<String, DelegatedTask>>,
    /// قدرات الـ agents المسجَّلة
    capabilities: RwLock<HashMap<String, AgentCapabilities>>,
}

impl AgentBus {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            senders: RwLock::new(HashMap::new()),
            tasks: RwLock::new(HashMap::new()),
            capabilities: RwLock::new(HashMap::new()),
        })
    }

    /// تسجيل agent جديد في الـ bus
    pub async fn register_agent(
        &self,
        capabilities: AgentCapabilities,
    ) -> mpsc::Receiver<AgentMessage> {
        let (tx, rx) = mpsc::channel(100);
        let agent_id = capabilities.agent_id.clone();

        self.senders.write().await.insert(agent_id.clone(), tx);
        self.capabilities.write().await.insert(agent_id.clone(), capabilities);

        info!("Agent {} registered on bus", agent_id);
        rx
    }

    /// إرسال رسالة لـ agent محدد
    pub async fn send(&self, message: AgentMessage) -> Result<(), String> {
        let senders = self.senders.read().await;
        match senders.get(&message.to_agent) {
            Some(tx) => {
                tx.send(message.clone())
                    .await
                    .map_err(|e| format!("Failed to send to agent {}: {}", message.to_agent, e))?;
                debug!("Message sent from {} to {}", message.from_agent, message.to_agent);
                Ok(())
            }
            None => Err(format!("Agent '{}' not found on bus", message.to_agent)),
        }
    }

    /// بث رسالة لجميع الـ agents
    pub async fn broadcast(&self, from_agent: &str, payload: serde_json::Value) {
        let senders = self.senders.read().await;
        for (agent_id, tx) in senders.iter() {
            if agent_id == from_agent {
                continue;
            }
            let msg = AgentMessage {
                id: Uuid::new_v4().to_string(),
                from_agent: from_agent.to_string(),
                to_agent: agent_id.clone(),
                message_type: AgentMessageType::Broadcast,
                payload: payload.clone(),
                correlation_id: None,
                created_at: chrono::Utc::now(),
            };
            let _ = tx.send(msg).await;
        }
    }

    /// تفويض مهمة لأفضل agent متاح
    pub async fn delegate_task(
        &self,
        from_agent: &str,
        task_type: &str,
        input: &str,
    ) -> Result<String, String> {
        // إيجاد أفضل agent للمهمة
        let best_agent = self.find_best_agent(task_type).await?;

        let task_id = Uuid::new_v4().to_string();
        let task = DelegatedTask {
            task_id: task_id.clone(),
            from_agent: from_agent.to_string(),
            to_agent: best_agent.clone(),
            task_type: task_type.to_string(),
            input: input.to_string(),
            status: TaskStatus::Pending,
            result: None,
            created_at: chrono::Utc::now(),
            completed_at: None,
        };

        self.tasks.write().await.insert(task_id.clone(), task);

        // إرسال طلب التفويض
        let message = AgentMessage {
            id: Uuid::new_v4().to_string(),
            from_agent: from_agent.to_string(),
            to_agent: best_agent.clone(),
            message_type: AgentMessageType::TaskRequest,
            payload: serde_json::json!({
                "task_id": task_id,
                "task_type": task_type,
                "input": input
            }),
            correlation_id: Some(task_id.clone()),
            created_at: chrono::Utc::now(),
        };

        self.send(message).await?;

        info!(
            "Task {} delegated from {} to {}",
            task_id, from_agent, best_agent
        );

        Ok(task_id)
    }

    /// إيجاد أفضل agent لنوع مهمة معين
    async fn find_best_agent(&self, task_type: &str) -> Result<String, String> {
        let capabilities = self.capabilities.read().await;

        // فلترة الـ agents المتاحة والمتخصصة في هذا النوع
        let mut candidates: Vec<&AgentCapabilities> = capabilities
            .values()
            .filter(|c| {
                c.is_available
                    && c.current_load < c.max_concurrent_tasks
                    && (c.specializations.contains(&task_type.to_string())
                        || c.specializations.contains(&"general".to_string()))
            })
            .collect();

        if candidates.is_empty() {
            return Err(format!("No available agent for task type: {}", task_type));
        }

        // اختيار الـ agent الأقل تحميلاً
        candidates.sort_by_key(|c| c.current_load);
        Ok(candidates[0].agent_id.clone())
    }

    /// تحديث حالة مهمة
    pub async fn update_task_status(
        &self,
        task_id: &str,
        status: TaskStatus,
        result: Option<String>,
    ) {
        let mut tasks = self.tasks.write().await;
        if let Some(task) = tasks.get_mut(task_id) {
            task.status = status.clone();
            task.result = result;
            if status == TaskStatus::Completed || status == TaskStatus::Failed {
                task.completed_at = Some(chrono::Utc::now());
            }
        }
    }

    /// جلب حالة مهمة
    pub async fn get_task(&self, task_id: &str) -> Option<DelegatedTask> {
        self.tasks.read().await.get(task_id).cloned()
    }

    /// قائمة الـ agents المتاحة
    pub async fn list_agents(&self) -> Vec<AgentCapabilities> {
        self.capabilities.read().await.values().cloned().collect()
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Unit tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_capabilities(id: &str, specializations: Vec<&str>) -> AgentCapabilities {
        AgentCapabilities {
            agent_id: id.to_string(),
            name: format!("Agent {}", id),
            specializations: specializations.into_iter().map(|s| s.to_string()).collect(),
            max_concurrent_tasks: 5,
            current_load: 0,
            is_available: true,
        }
    }

    #[tokio::test]
    async fn test_register_and_list_agents() {
        let bus = AgentBus::new();
        bus.register_agent(make_capabilities("agent-1", vec!["code"])).await;
        bus.register_agent(make_capabilities("agent-2", vec!["research"])).await;

        let agents = bus.list_agents().await;
        assert_eq!(agents.len(), 2);
    }

    #[tokio::test]
    async fn test_send_message_to_registered_agent() {
        let bus = AgentBus::new();
        let mut rx = bus.register_agent(make_capabilities("agent-1", vec!["general"])).await;

        let msg = AgentMessage {
            id: "msg-1".into(),
            from_agent: "agent-0".into(),
            to_agent: "agent-1".into(),
            message_type: AgentMessageType::Heartbeat,
            payload: serde_json::json!({}),
            correlation_id: None,
            created_at: chrono::Utc::now(),
        };

        bus.send(msg).await.unwrap();
        let received = rx.recv().await.unwrap();
        assert_eq!(received.from_agent, "agent-0");
    }

    #[tokio::test]
    async fn test_send_to_unknown_agent_fails() {
        let bus = AgentBus::new();
        let msg = AgentMessage {
            id: "msg-1".into(),
            from_agent: "agent-0".into(),
            to_agent: "nonexistent".into(),
            message_type: AgentMessageType::Heartbeat,
            payload: serde_json::json!({}),
            correlation_id: None,
            created_at: chrono::Utc::now(),
        };
        let result = bus.send(msg).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_delegate_task_to_best_agent() {
        let bus = AgentBus::new();
        let _rx1 = bus.register_agent(make_capabilities("agent-code", vec!["code"])).await;
        let _rx2 = bus.register_agent(make_capabilities("agent-research", vec!["research"])).await;

        let task_id = bus.delegate_task("orchestrator", "code", "write a function").await.unwrap();
        assert!(!task_id.is_empty());

        let task = bus.get_task(&task_id).await.unwrap();
        assert_eq!(task.to_agent, "agent-code");
        assert_eq!(task.status, TaskStatus::Pending);
    }

    #[tokio::test]
    async fn test_update_task_status() {
        let bus = AgentBus::new();
        let _rx = bus.register_agent(make_capabilities("agent-1", vec!["general"])).await;
        let task_id = bus.delegate_task("orchestrator", "general", "do something").await.unwrap();

        bus.update_task_status(&task_id, TaskStatus::Completed, Some("done!".into())).await;

        let task = bus.get_task(&task_id).await.unwrap();
        assert_eq!(task.status, TaskStatus::Completed);
        assert_eq!(task.result.as_deref(), Some("done!"));
        assert!(task.completed_at.is_some());
    }
}
