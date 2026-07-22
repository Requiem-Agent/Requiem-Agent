//! # ReAct Loop Engine — S2-02
//!
//! تطبيق نمط ReAct (Reasoning + Acting) للـ agent.
//!
//! ## الفلسفة:
//! ```
//! User Input
//!     ↓
//! [THINK] — تحليل المشكلة، تحديد الأدوات المطلوبة
//!     ↓
//! [ACT]   — تنفيذ أداة (tool call)
//!     ↓
//! [OBSERVE] — قراءة نتيجة الأداة
//!     ↓
//! [THINK] — هل اكتملت المهمة؟
//!     ↓ (إذا لا)
//! [ACT]   — أداة أخرى
//!     ↓
//! [FINAL] — الإجابة النهائية
//! ```
//!
//! ## الحد الأقصى للدورات: 10 (منع infinite loops)
//! ## Timeout لكل دورة: 45 ثانية

use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
use tracing::{debug, info, warn};
use crate::orchestrator::{TaskClassifier, Effort, ParallelExecutor};

// ─── الثوابت ───────────────────────────────────────────────────────────────

/// الحد الأقصى لدورات ReAct (منع infinite loops)
const MAX_REACT_ITERATIONS: usize = 10;
/// Timeout لكل دورة بالثواني
const REACT_ITERATION_TIMEOUT_SECS: u64 = 45;
/// Timeout إجمالي للـ loop كاملاً
const REACT_TOTAL_TIMEOUT_SECS: u64 = 300; // 5 دقائق

// ─── أنواع الخطوات ─────────────────────────────────────────────────────────

/// نوع خطوة ReAct
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum StepType {
    /// تفكير — تحليل الوضع وتحديد الخطوة التالية
    Think,
    /// تصرف — استدعاء أداة
    Act,
    /// مراقبة — قراءة نتيجة الأداة
    Observe,
    /// إجابة نهائية
    Final,
    /// خطأ في الدورة
    Error,
}

/// خطوة واحدة في دورة ReAct
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReActStep {
    /// نوع الخطوة
    pub step_type: StepType,
    /// رقم الدورة (1-based)
    pub iteration: usize,
    /// محتوى الخطوة (تفكير أو نتيجة أداة)
    pub content: String,
    /// اسم الأداة المستخدمة (إذا كانت Act)
    pub tool_name: Option<String>,
    /// مدخلات الأداة (إذا كانت Act)
    pub tool_input: Option<serde_json::Value>,
    /// مدة تنفيذ الخطوة بالمللي ثانية
    pub duration_ms: u64,
}

/// نتيجة دورة ReAct كاملة
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReActResult {
    /// هل اكتملت المهمة بنجاح؟
    pub success: bool,
    /// الإجابة النهائية
    pub final_answer: Option<String>,
    /// جميع الخطوات المنفذة
    pub steps: Vec<ReActStep>,
    /// عدد الدورات المستخدمة
    pub iterations_used: usize,
    /// المدة الإجمالية بالمللي ثانية
    pub total_duration_ms: u64,
    /// سبب التوقف (نجاح، حد الدورات، timeout، خطأ)
    pub stop_reason: StopReason,
}

/// سبب توقف الـ loop
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StopReason {
    /// اكتملت المهمة بنجاح
    Completed,
    /// وصل للحد الأقصى من الدورات
    MaxIterationsReached,
    /// انتهت المهلة الزمنية
    Timeout,
    /// خطأ في التنفيذ
    Error(String),
}

// ─── Tool Registry ─────────────────────────────────────────────────────────

/// تعريف أداة يمكن للـ agent استخدامها
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    /// اسم الأداة (يُستخدم في tool calls)
    pub name: String,
    /// وصف الأداة للـ LLM
    pub description: String,
    /// مخطط المدخلات (JSON Schema)
    pub input_schema: serde_json::Value,
}

/// نتيجة تنفيذ أداة
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    /// اسم الأداة
    pub tool_name: String,
    /// هل نجح التنفيذ؟
    pub success: bool,
    /// النتيجة (إذا نجح)
    pub output: Option<serde_json::Value>,
    /// رسالة الخطأ (إذا فشل)
    pub error: Option<String>,
    /// مدة التنفيذ
    pub duration_ms: u64,
}

// ─── ReAct Engine ──────────────────────────────────────────────────────────

/// محرك ReAct Loop
pub struct ReActEngine {
    /// الأدوات المتاحة
    tools: Vec<ToolDefinition>,
    /// الحد الأقصى للدورات
    max_iterations: usize,
    /// Timeout لكل دورة
    iteration_timeout: Duration,
    /// Timeout إجمالي
    total_timeout: Duration,
}

impl ReActEngine {
    /// إنشاء محرك ReAct جديد
    pub fn new(tools: Vec<ToolDefinition>) -> Self {
        Self {
            tools,
            max_iterations: MAX_REACT_ITERATIONS,
            iteration_timeout: Duration::from_secs(REACT_ITERATION_TIMEOUT_SECS),
            total_timeout: Duration::from_secs(REACT_TOTAL_TIMEOUT_SECS),
        }
    }

    /// إنشاء محرك بإعدادات مخصصة
    pub fn with_config(
        tools: Vec<ToolDefinition>,
        max_iterations: usize,
        iteration_timeout_secs: u64,
    ) -> Self {
        Self {
            tools,
            max_iterations,
            iteration_timeout: Duration::from_secs(iteration_timeout_secs),
            total_timeout: Duration::from_secs(REACT_TOTAL_TIMEOUT_SECS),
        }
    }

    /// تنفيذ دورة ReAct كاملة
    ///
    /// # Arguments
    /// * `user_query` - طلب المستخدم
    /// * `context` - سياق إضافي (محادثة سابقة، ملفات، إلخ)
    /// * `tool_executor` - دالة تنفيذ الأدوات
    pub async fn run<F, Fut>(
        &self,
        user_query: &str,
        context: Option<&str>,
        tool_executor: F,
    ) -> ReActResult
    where
        F: Fn(String, serde_json::Value) -> Fut + Send + Sync,
        Fut: std::future::Future<Output = ToolResult> + Send,
    {
        let total_start = Instant::now();
        let mut steps = Vec::new();
        let mut iteration = 0;

        // بناء الـ prompt الأولي
        let initial_prompt = self.build_initial_prompt(user_query, context);
        let mut conversation_history = vec![
            serde_json::json!({
                "role": "user",
                "content": initial_prompt,
            })
        ];

        info!("ReAct loop started: query={:?}, max_iter={}", &user_query[..user_query.len().min(50)], self.max_iterations);

        loop {
            // التحقق من الـ timeout الإجمالي
            if total_start.elapsed() > self.total_timeout {
                warn!("ReAct loop total timeout after {}s", total_start.elapsed().as_secs());
                return ReActResult {
                    success: false,
                    final_answer: None,
                    steps,
                    iterations_used: iteration,
                    total_duration_ms: total_start.elapsed().as_millis() as u64,
                    stop_reason: StopReason::Timeout,
                };
            }

            // التحقق من الحد الأقصى للدورات
            if iteration >= self.max_iterations {
                warn!("ReAct loop max iterations ({}) reached", self.max_iterations);
                return ReActResult {
                    success: false,
                    final_answer: self.extract_best_answer(&steps),
                    steps,
                    iterations_used: iteration,
                    total_duration_ms: total_start.elapsed().as_millis() as u64,
                    stop_reason: StopReason::MaxIterationsReached,
                };
            }

            iteration += 1;
            let iter_start = Instant::now();

            debug!("ReAct iteration {}/{}", iteration, self.max_iterations);

            // ─── THINK: استدعاء LLM للتفكير ───────────────────────────────
            let think_result = tokio::time::timeout(
                self.iteration_timeout,
                self.call_llm_think(&conversation_history),
            ).await;

            let think_content = match think_result {
                Err(_) => {
                    warn!("ReAct think timeout at iteration {iteration}");
                    steps.push(ReActStep {
                        step_type: StepType::Error,
                        iteration,
                        content: "Think timeout".to_string(),
                        tool_name: None,
                        tool_input: None,
                        duration_ms: iter_start.elapsed().as_millis() as u64,
                    });
                    continue;
                }
                Ok(Err(e)) => {
                    warn!("ReAct think error at iteration {iteration}: {e}");
                    steps.push(ReActStep {
                        step_type: StepType::Error,
                        iteration,
                        content: format!("Think error: {e}"),
                        tool_name: None,
                        tool_input: None,
                        duration_ms: iter_start.elapsed().as_millis() as u64,
                    });
                    continue;
                }
                Ok(Ok(content)) => content,
            };

            steps.push(ReActStep {
                step_type: StepType::Think,
                iteration,
                content: think_content.clone(),
                tool_name: None,
                tool_input: None,
                duration_ms: iter_start.elapsed().as_millis() as u64,
            });

            // إضافة تفكير الـ LLM للمحادثة
            conversation_history.push(serde_json::json!({
                "role": "assistant",
                "content": think_content.clone(),
            }));

            // ─── تحليل الرد: هل هو إجابة نهائية أم tool call؟ ────────────
            if let Some(final_answer) = self.extract_final_answer(&think_content) {
                info!("ReAct completed at iteration {iteration} with final answer");
                steps.push(ReActStep {
                    step_type: StepType::Final,
                    iteration,
                    content: final_answer.clone(),
                    tool_name: None,
                    tool_input: None,
                    duration_ms: iter_start.elapsed().as_millis() as u64,
                });
                return ReActResult {
                    success: true,
                    final_answer: Some(final_answer),
                    steps,
                    iterations_used: iteration,
                    total_duration_ms: total_start.elapsed().as_millis() as u64,
                    stop_reason: StopReason::Completed,
                };
            }

            // ─── ACT: تنفيذ أداة ──────────────────────────────────────────
            if let Some((tool_name, tool_input)) = self.extract_tool_call(&think_content) {
                steps.push(ReActStep {
                    step_type: StepType::Act,
                    iteration,
                    content: format!("Calling tool: {tool_name}"),
                    tool_name: Some(tool_name.clone()),
                    tool_input: Some(tool_input.clone()),
                    duration_ms: 0,
                });

                let act_start = Instant::now();
                let tool_result = tokio::time::timeout(
                    self.iteration_timeout,
                    tool_executor(tool_name.clone(), tool_input),
                ).await;

                let observe_content = match tool_result {
                    Err(_) => {
                        format!("Tool '{tool_name}' timed out after {}s", self.iteration_timeout.as_secs())
                    }
                    Ok(result) => {
                        if result.success {
                            serde_json::to_string_pretty(&result.output.unwrap_or_default())
                                .unwrap_or_else(|_| "Tool executed successfully".to_string())
                        } else {
                            format!("Tool error: {}", result.error.unwrap_or_default())
                        }
                    }
                };

                // ─── OBSERVE: تسجيل نتيجة الأداة ─────────────────────────
                steps.push(ReActStep {
                    step_type: StepType::Observe,
                    iteration,
                    content: observe_content.clone(),
                    tool_name: Some(tool_name),
                    tool_input: None,
                    duration_ms: act_start.elapsed().as_millis() as u64,
                });

                // إضافة نتيجة الأداة للمحادثة
                conversation_history.push(serde_json::json!({
                    "role": "user",
                    "content": format!("Observation: {observe_content}\n\nContinue with the next step."),
                }));
            }
            // إذا لم يكن هناك tool call ولا final answer — نطلب من LLM التوضيح
            else {
                conversation_history.push(serde_json::json!({
                    "role": "user",
                    "content": "Please either use a tool or provide a final answer with 'Final Answer:' prefix.",
                }));
            }
        }
    }

    /// بناء الـ prompt الأولي للـ ReAct loop
    fn build_initial_prompt(&self, query: &str, context: Option<&str>) -> String {
        let tools_desc = self.tools.iter()
            .map(|t| format!("- **{}**: {}", t.name, t.description))
            .collect::<Vec<_>>()
            .join("\n");

        let context_section = context
            .map(|c| format!("\n\n## Context:\n{c}"))
            .unwrap_or_default();

        format!(
            r#"You are an AI agent that solves tasks step by step using the ReAct pattern.

## Available Tools:
{tools_desc}

## Instructions:
1. Think about the problem and what tool to use
2. Use a tool with: `Tool: <tool_name>\nInput: <json_input>`
3. After observing the result, think again
4. When done, provide: `Final Answer: <your answer>`

## Task:
{query}{context_section}

Begin your reasoning:"#
        )
    }

    /// استدعاء LLM حقيقي عبر Orchestrator — Sprint 1 Alpha
    async fn call_llm_think(
        &self,
        conversation: &[serde_json::Value],
    ) -> Result<String, String> {
        // استخراج query من آخر رسالة
        let query = conversation
            .last()
            .and_then(|m| m["content"].as_str())
            .unwrap_or("");
        
        // تصنيف المهمة واختيار النماذج المناسبة
        let category = TaskClassifier::classify(query);
        let models = TaskClassifier::suggest_models(category, Effort::Medium);
        
        let model_refs: Vec<&str> = models.iter().map(|s| s.as_str()).collect();
        
        info!(
            "ReAct calling LLM: category={}, models={:?}, query_len={}",
            category, model_refs, query.len()
        );
        
        let user_id = std::env::var("DEFAULT_USER_ID").unwrap_or_else(|_| "default".to_string());
        
        // تنفيذ متوازي على عدة نماذج واختيار الأفضل
        let result = ParallelExecutor::execute_parallel(
            &model_refs,
            conversation,
            &user_id,
        ).await;
        
        match result.best_content {
            Some(content) if !content.is_empty() => {
                info!(
                    "ReAct LLM response: model={}, len={}, duration={}ms",
                    result.selected_from.as_deref().unwrap_or("unknown"),
                    content.len(),
                    result.total_duration_ms
                );
                Ok(content)
            }
            _ => {
                // Fallback: استخدم نموذج واحد سريع
                warn!("Parallel execution returned empty, trying direct fallback");
                let fallback_result = ParallelExecutor::execute_parallel(
                    &["deepseek-v4-flash-free"],
                    conversation,
                    &user_id,
                ).await;
                
                fallback_result.best_content
                    .filter(|c| !c.is_empty())
                    .ok_or_else(|| "All LLM models failed to respond".to_string())
            }
        }
    }

    /// استخراج الإجابة النهائية من رد LLM
    fn extract_final_answer(&self, content: &str) -> Option<String> {
        // البحث عن "Final Answer:" في الرد
        let markers = ["Final Answer:", "FINAL ANSWER:", "final answer:"];
        for marker in &markers {
            if let Some(pos) = content.find(marker) {
                let answer = content[pos + marker.len()..].trim().to_string();
                if !answer.is_empty() {
                    return Some(answer);
                }
            }
        }
        None
    }

    /// استخراج tool call من رد LLM
    fn extract_tool_call(&self, content: &str) -> Option<(String, serde_json::Value)> {
        // البحث عن "Tool: <name>" و "Input: <json>"
        let tool_line = content.lines()
            .find(|l| l.trim_start().starts_with("Tool:"))?;

        let tool_name = tool_line
            .trim_start_matches("Tool:")
            .trim()
            .to_string();

        // التحقق من أن الأداة موجودة في السجل
        if !self.tools.iter().any(|t| t.name == tool_name) {
            warn!("LLM requested unknown tool: {tool_name}");
            return None;
        }

        // استخراج الـ input
        let input_line = content.lines()
            .find(|l| l.trim_start().starts_with("Input:"))?;

        let input_str = input_line
            .trim_start_matches("Input:")
            .trim();

        let input: serde_json::Value = serde_json::from_str(input_str)
            .unwrap_or_else(|_| serde_json::json!({"raw": input_str}));

        Some((tool_name, input))
    }

    /// استخراج أفضل إجابة من الخطوات المنفذة (عند الوصول للحد الأقصى)
    fn extract_best_answer(&self, steps: &[ReActStep]) -> Option<String> {
        // أخذ آخر خطوة Think كإجابة تقريبية
        steps.iter()
            .rev()
            .find(|s| s.step_type == StepType::Think)
            .map(|s| s.content.clone())
    }
}

// ─── الأدوات الافتراضية للـ Requiem Agent ─────────────────────────────────

/// إنشاء قائمة الأدوات الافتراضية للـ Requiem Agent
pub fn default_requiem_tools() -> Vec<ToolDefinition> {
    vec![
        ToolDefinition {
            name: "read_file".to_string(),
            description: "قراءة محتوى ملف من workspace المستخدم".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {"type": "string", "description": "مسار الملف"},
                    "session_id": {"type": "string", "description": "معرف الجلسة"}
                },
                "required": ["path", "session_id"]
            }),
        },
        ToolDefinition {
            name: "write_file".to_string(),
            description: "كتابة محتوى إلى ملف في workspace المستخدم".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {"type": "string"},
                    "content": {"type": "string"},
                    "session_id": {"type": "string"}
                },
                "required": ["path", "content", "session_id"]
            }),
        },
        ToolDefinition {
            name: "execute_code".to_string(),
            description: "تنفيذ كود في sandbox آمن".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "language": {"type": "string", "enum": ["python", "javascript", "rust", "bash"]},
                    "code": {"type": "string"},
                    "timeout_secs": {"type": "integer", "default": 30}
                },
                "required": ["language", "code"]
            }),
        },
        ToolDefinition {
            name: "search_memory".to_string(),
            description: "البحث في ذاكرة المستخدم (RAG)".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "query": {"type": "string"},
                    "limit": {"type": "integer", "default": 5}
                },
                "required": ["query"]
            }),
        },
        ToolDefinition {
            name: "list_files".to_string(),
            description: "عرض قائمة الملفات في workspace".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "session_id": {"type": "string"}
                },
                "required": ["session_id"]
            }),
        },
    ]
}

// ─── اختبارات ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_engine() -> ReActEngine {
        ReActEngine::new(default_requiem_tools())
    }

    #[test]
    fn test_extract_final_answer() {
        let engine = make_engine();
        let content = "I analyzed the problem.\nFinal Answer: The answer is 42.";
        let answer = engine.extract_final_answer(content);
        assert_eq!(answer, Some("The answer is 42.".to_string()));
    }

    #[test]
    fn test_extract_final_answer_none() {
        let engine = make_engine();
        let content = "I need to think more about this.";
        assert!(engine.extract_final_answer(content).is_none());
    }

    #[test]
    fn test_extract_tool_call_valid() {
        let engine = make_engine();
        let content = "I need to read a file.\nTool: read_file\nInput: {\"path\": \"main.rs\", \"session_id\": \"abc\"}";
        let result = engine.extract_tool_call(content);
        assert!(result.is_some());
        let (name, input) = result.unwrap();
        assert_eq!(name, "read_file");
        assert_eq!(input["path"], "main.rs");
    }

    #[test]
    fn test_extract_tool_call_unknown_tool() {
        let engine = make_engine();
        let content = "Tool: unknown_tool\nInput: {}";
        let result = engine.extract_tool_call(content);
        assert!(result.is_none(), "يجب رفض الأدوات غير المعروفة");
    }

    #[test]
    fn test_default_tools_not_empty() {
        let tools = default_requiem_tools();
        assert!(!tools.is_empty());
        assert!(tools.iter().any(|t| t.name == "execute_code"));
        assert!(tools.iter().any(|t| t.name == "read_file"));
    }

    #[test]
    fn test_engine_config() {
        let engine = ReActEngine::with_config(
            default_requiem_tools(),
            5,
            30,
        );
        assert_eq!(engine.max_iterations, 5);
        assert_eq!(engine.iteration_timeout, Duration::from_secs(30));
    }

    #[tokio::test]
    async fn test_react_loop_final_answer() {
        let engine = make_engine();

        // الـ stub يُرجع "Final Answer:" مباشرة
        let result = engine.run(
            "What is 2 + 2?",
            None,
            |_tool, _input| async {
                ToolResult {
                    tool_name: "test".to_string(),
                    success: true,
                    output: Some(serde_json::json!({"result": 4})),
                    error: None,
                    duration_ms: 10,
                }
            },
        ).await;

        assert!(result.success, "يجب أن تنجح الدورة");
        assert!(result.final_answer.is_some(), "يجب أن تكون هناك إجابة نهائية");
        assert_eq!(result.stop_reason as u8, StopReason::Completed as u8);
    }
}

// تطبيق PartialEq لـ StopReason للاختبارات
impl PartialEq for StopReason {
    fn eq(&self, other: &Self) -> bool {
        std::mem::discriminant(self) == std::mem::discriminant(other)
    }
}

// تحويل StopReason إلى u8 للاختبارات
impl StopReason {
    fn as_u8(&self) -> u8 {
        match self {
            Self::Completed => 0,
            Self::MaxIterationsReached => 1,
            Self::Timeout => 2,
            Self::Error(_) => 3,
        }
    }
}
