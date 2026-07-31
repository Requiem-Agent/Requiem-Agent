//! # Protocol Module — بروتوكولات الوكيل البرمجية الصارمة
//!
//! ## المكونات
//! - `thinking.rs` — Structured Thinking Protocol (STP)
//! - `mode.rs` — Agent Mode Protocol
//! - `sub_agent.rs` — Sub-Agent Spawn Protocol

pub mod thinking;
pub mod mode;
pub mod sub_agent;

use serde::{Deserialize, Serialize};
use crate::agent::protocol::thinking::{ThinkingProtocol, ProtocolMode};

/// حالة الوكيل الكاملة — تُستخدم لاتخاذ القرارات
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentState {
    pub mode: mode::AgentMode,
    pub thinking: Option<serde_json::Value>,
    pub active_sub_agents: Vec<String>,
    pub current_task: Option<String>,
    pub steps_taken: u32,
    pub max_steps: u32,
}

impl AgentState {
    pub fn can_proceed(&self) -> bool {
        self.steps_taken < self.max_steps
    }
}

/// ProtocolVersion — لتتبع التغييرات في البروتوكولات
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtocolVersion {
    pub protocol: String,
    pub version: String,
    pub required: bool,
}
