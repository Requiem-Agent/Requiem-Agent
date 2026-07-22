//! # Agent Engine — محرك الوكيل الرئيسي
//!
//! يربط جميع أنظمة الوكيل في محرك واحد:
//! - بروتوكولات التفكير (STP)
//! - التحكم في الأوضاع (Mode)
//! - الوكلاء الفرعيون (Sub-Agent)
//! - المترجم والمصحح (Compiler)
//! - المهارات (Skills)
//! - التفاعل مع المستخدم (Questions)
//! - إدارة المهام (Tasks)
//! - الأدوات (Tools) - search, parser, diff, vcs, file_finder

pub mod protocol;
pub mod compiler;
pub mod tasks;
pub mod skills;
pub mod user_questions;
pub mod anti_printer;
pub mod synergy;
pub mod memory;
pub mod identity_shield;
pub mod tools;

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use crate::agent::protocol::mode::{AgentMode, ModeController, ModeConstraints};
use crate::agent::protocol::thinking::{ThinkingProtocol, ProtocolMode, ThinkingValidation};
use crate::agent::protocol::sub_agent::{SubAgentOrchestrator, SubAgentSpec, SubAgentProgress};
use crate::agent::compiler::output::{AgentOutputCompiler, CompiledOutput, CompilerConfig};
use crate::agent::compiler::auto_correct::JsonAutoCorrect;
use crate::agent::anti_printer::{CompilerPipeline, AntiPrinterReport, ContextRouter, PatternDetector, SemanticEngine};
use crate::agent::synergy::{ModelSynergyCoordinator, SynergyPattern, SynergyRound};
use crate::agent::memory::{RagEngine, SessionMemory, UserMemory};
use crate::agent::identity_shield::{IdentityShieldV3, IdentityCheckResult};
use crate::enforce::audit::AuditLog;
use libsql::Connection;
use crate::agent::tools::AgentTools;

/// محرك الوكيل الرئيسي
pub struct AgentEngine {
    pub orchestrator_active: bool,
    pub mode: ModeController,
    pub thinking: ThinkingProtocol,
    pub sub_agents: SubAgentOrchestrator,
    pub compiler: AgentOutputCompiler,
    pub auto_correct: JsonAutoCorrect,
    pub pipeline: CompilerPipeline,
    pub context_router: ContextRouter,
    pub synergy: ModelSynergyCoordinator,
    pub last_task_category: Option<crate::orchestrator::TaskCategory>,
    pub last_model_used: Option<String>,
    pub rag: Arc<RwLock<RagEngine>>,
    pub identity_shield: IdentityShieldV3,
    pub tools: AgentTools,
    pub audit_log: Arc<RwLock<AuditLog>>,
    pub user_id: String,
    pub steps_taken: u64,
    pub max_steps: u64,
}

impl AgentEngine {
    /// إنشاء محرك وكيل جديد
    pub fn new(user_id: &str, mode: AgentMode, audit_log: Arc<RwLock<AuditLog>>, conn: Arc<Connection>) -> Self {
            orchestrator_active: true,
        let thinking_mode = mode.constraints().thinking_mode;
        Self {
            mode: ModeController::new(mode),
            thinking: ThinkingProtocol::new(thinking_mode),
            sub_agents: SubAgentOrchestrator::new(mode.constraints().max_sub_agents),
            compiler: AgentOutputCompiler::new(CompilerConfig {
                strictness: if mode == AgentMode::Turbo {
                    crate::tools::Strictness::Normal
                } else {
                    crate::tools::Strictness::Strict
                },
                ..Default::default()
            }),
            auto_correct: JsonAutoCorrect::new(),
            pipeline: CompilerPipeline::new(),
            context_router: ContextRouter::new(),
            synergy: ModelSynergyCoordinator::new(),
            rag: Arc::new(RwLock::new(RagEngine::new(conn, user_id))),
            identity_shield: IdentityShieldV3::new("internal-model"), // الهوية موحدة - صارم
            tools: AgentTools::default(),
            audit_log,
            user_id: user_id.to_string(),
            steps_taken: 0,
            max_steps: 100,
            last_task_category: None,
            last_model_used: None,
        }
    }

    /// تغيير وضع الوكيل — يحدّث كل الأنظمة المرتبطة
    pub fn switch_mode(&mut self, new_mode: AgentMode, reason: &str) -> ModeConstraints {
        let constraints = self.mode.switch(new_mode, reason, "agent");
        self.thinking = ThinkingProtocol::new(constraints.thinking_mode);
        let max = constraints.max_sub_agents;
        self.sub_agents = SubAgentOrchestrator::new(max);
        constraints
    }

    /// هل يمكن للوكيل المتابعة؟
    pub fn can_proceed(&self) -> bool {
        self.steps_taken < self.max_steps
            && self.mode.constraints().max_consecutive_steps as u64 > self.steps_taken
    }

    /// تسجيل خطوة — يسجل في audit log
    pub async fn record_step(&mut self, action: &str, params: &serde_json::Value, success: bool) {
        self.steps_taken += 1;
        self.audit_log.write().await.record(
            &self.user_id, action, params, success,
        );
    }

    /// تصنيف مهمة وتوجيهها عبر Orchestrator — Sprint 1B
    pub fn classify_and_route(&mut self, query: &str, effort: Option<crate::orchestrator::Effort>) -> (crate::orchestrator::TaskCategory, Vec<String>) {
        let category = crate::orchestrator::TaskClassifier::classify(query);
        let effort = effort.unwrap_or(crate::orchestrator::Effort::Medium);
        let models = crate::orchestrator::TaskClassifier::suggest_models(category, effort);
        let model_names: Vec<String> = models.iter().map(|s| s.to_string()).collect();
        self.last_task_category = Some(category);
        self.last_model_used = model_names.first().cloned();
        (category, model_names)
    }

    /// تقرير كامل عن حالة الوكيل
    pub fn status_report(&self) -> serde_json::Value {
        serde_json::json!({
            "engine": "requiem-agent-v2",
            "user_id": self.user_id,
            "orchestrator": {
                "active": self.orchestrator_active,
                "last_category": self.last_task_category.map(|c| c.to_string()),
                "last_model": self.last_model_used.clone(),
            },
            "mode": self.mode.current().name(),
            "steps_taken": self.steps_taken,
            "max_steps": self.max_steps,
            "can_proceed": self.can_proceed(),
            "thinking_mode": format!("{:?}", self.mode.constraints().thinking_mode),
            "sub_agents": {
                "active": self.sub_agents.active_count(),
                "total": self.sub_agents.list_children().len(),
            },
            "compiler_ready": true,
            "anti_printer": {
                "pipeline_ready": true,
                "semantic_engine": true,
                "pattern_detector": true,
                "models_available": self.context_router.models.len(),
                "current_strategy": self.context_router.strategy.name(),
            },
            "synergy": {
                "active_pattern": self.synergy.active_pattern.name(),
                "rounds_completed": self.synergy.history.len(),
                "models_tracked": self.synergy.router.history.len(),
            },
            "rag": {
                "ready": true,
                "embedding_dimension": 128,
            },
            "tools": {
                "search": "ready",
                "parser": "ready",
                "diff": "ready",
                "vcs": "ready",
                "file_finder": "ready",
            },
        })
    }

    /// تشغيل Pipeline التصحيح الكامل
    pub async fn run_pipeline(&mut self, thinking: &str, tool_calls: &str, history: &[String], step_id: u64) -> AntiPrinterReport {
        let report = self.pipeline.run(thinking, tool_calls, history, step_id).await;
        if report.anti_printer.requires_retry {
            self.audit_log.write().await.record(&self.user_id, "anti_printer_retry", &serde_json::json!({
                "patterns": report.anti_printer.patterns.len(),
                "quality": report.anti_printer.quality_score,
            }), false);
        }
        report.anti_printer
    }

    /// توزيع مهمة على نموذج
    pub fn route_task(&mut self, description: &str, task_id: &str, tokens: usize) -> crate::agent::anti_printer::TaskDistribution {
        self.context_router.route(description, task_id, tokens)
    }

    /// تحليل دلالي لنص
    pub fn analyze_semantic(&mut self, text: &str, step_id: u64) -> crate::agent::anti_printer::SemanticResult {
        self.pipeline.semantic.analyze(text, step_id)
    }

    /// استرجاع سياق من RAG
    pub async fn retrieve_rag_context(&self, query: &str, max_tokens: usize) -> String {
        let rag = self.rag.read().await;
        match rag.build_context(query, None, max_tokens).await {
            Ok(result) if result.memories_used > 0 => result.system_context,
            Ok(_) => String::new(),
            Err(e) => {
                tracing::warn!("RAG retrieval failed: {}", e);
                String::new()
            }
        }
    }

    /// حفظ ذاكرة في RAG
    pub async fn store_rag_memory(&self, content: &str, memory_type: memory::MemoryType, priority: memory::MemoryPriority) -> anyhow::Result<String> {
        let rag = self.rag.read().await;
        let priority_str = match priority {
            memory::MemoryPriority::Low => "low",
            memory::MemoryPriority::Medium => "medium",
            memory::MemoryPriority::High => "high",
            memory::MemoryPriority::Critical => "critical",
        };
        rag.store(content, memory_type.name(), priority_str, None).await
    }

    /// إحصائيات RAG
    pub async fn rag_stats(&self) -> memory::RagStats {
        let rag = self.rag.read().await;
        rag.stats().await.unwrap_or_else(|_| memory::RagStats {
            total: 0,
            by_type: std::collections::HashMap::new(),
            by_priority: std::collections::HashMap::new(),
        })
    }

    /// فحص محاولة اختراق الهوية (v3)
    pub fn check_identity_probe(&mut self, user_input: &str) -> IdentityCheckResult {
        self.identity_shield.check(user_input)
    }

    /// فحص سريع لمحاولة الاختراق
    pub fn is_identity_probe(&self, user_input: &str) -> bool {
        self.identity_shield.check_quick(user_input)
    }

    /// توليد system prompt للهوية
    pub fn identity_context(&self) -> String {
        self.identity_shield.generate_system_prompt()
    }

    /// إحصائيات درع الهوية
    pub fn identity_stats(&self) -> identity_shield::ShieldStats {
        self.identity_shield.stats()
    }

    /// فحص تجاوز حد المعرفة
    pub fn check_knowledge_cutoff(&self, query: &str) -> identity_shield::CutoffCheckResult {
        let detector = identity_shield::KnowledgeCutoffDetector::new();
        detector.needs_current_info(query)
    }

    // ─── Tool Methods ─────────────────────────────────────────────────────

    /// Search code in the project using ripgrep
    pub async fn search_code(
        &self,
        pattern: &str,
        root_path: &str,
        extensions: Option<Vec<String>>,
    ) -> Result<Vec<tools::search::SearchResult>, tools::search::SearchError> {
        self.tools.search.search(pattern, root_path, extensions).await
    }

    /// Parse a file using tree-sitter
    pub fn parse_file(&self, file_path: &str) -> Result<tools::parser::AstNode, tools::parser::ParserError> {
        self.tools.parser.parse_file(file_path)
    }

    /// Compare two texts and generate diff
    pub fn compare_texts(&self, old_text: &str, new_text: &str) -> tools::diff::DiffResult {
        self.tools.diff.compare_texts(old_text, new_text)
    }

    /// Compare two files and generate diff
    pub fn compare_files(&self, old_file: &str, new_file: &str) -> Result<tools::diff::DiffResult, tools::diff::DiffError> {
        self.tools.diff.compare_files(old_file, new_file)
    }

    /// Get VCS repository information
    pub fn get_vcs_info(&self) -> Result<tools::vcs::RepositoryInfo, tools::vcs::VcsError> {
        self.tools.vcs.get_info()
    }

    /// Create a checkpoint (commit) in VCS
    pub fn create_checkpoint(&self, message: &str) -> Result<String, tools::vcs::VcsError> {
        self.tools.vcs.checkpoint(message)
    }

    /// Rollback to previous commit in VCS
    pub fn rollback_vcs(&self) -> Result<(), tools::vcs::VcsError> {
        self.tools.vcs.rollback()
    }

    /// Find files in the project
    pub fn find_files(&self, root_path: &str) -> Result<Vec<tools::file_finder::FoundFile>, tools::file_finder::FileFinderError> {
        self.tools.file_finder.find_files(root_path)
    }

    /// Find files by extension
    pub fn find_files_by_extension(&self, root_path: &str, extension: &str) -> Result<Vec<tools::file_finder::FoundFile>, tools::file_finder::FileFinderError> {
        self.tools.file_finder.find_by_extension(root_path, extension)
    }

    /// Get file size statistics
    pub fn get_file_stats(&self, root_path: &str) -> Result<tools::file_finder::SizeStats, tools::file_finder::FileFinderError> {
        self.tools.file_finder.get_size_stats(root_path)
    }
}
