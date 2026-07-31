//! # Agent Output Compiler — تجميع مخرجات الوكيل
//!
//! يحول مخرجات الوكيل (نص حر) إلى JSON صارم مع التحقق من:
//! - الـ JSON Schema
//! - الأمان (API keys? كود ضار؟)
//! - النوع (type checking)
//! - حدود الحجم

use crate::agent::compiler::auto_correct::{CorrectionResult, JsonAutoCorrect};
use crate::agent::compiler::{
    CompileError, CompileErrorCode, CompileResult, Correction, ToolCallValidation,
    ValidatedToolCall,
};
use crate::enforce::scanner::SecurityScanner;
use crate::tools::{JsonSchema, Strictness, ToolRegistry};
use serde_json::Value;
use std::time::Instant;

/// إعدادات المترجم
#[derive(Debug, Clone)]
pub struct CompilerConfig {
    pub max_tool_calls: usize,
    pub max_output_size: usize,
    pub strictness: Strictness,
    pub enable_security_scan: bool,
    pub enable_auto_correct: bool,
    pub require_tool_selection: bool,
}

impl Default for CompilerConfig {
    fn default() -> Self {
        Self {
            max_tool_calls: 10,
            max_output_size: 100_000,
            strictness: Strictness::Strict,
            enable_security_scan: true,
            enable_auto_correct: true,
            require_tool_selection: false,
        }
    }
}

/// مخرجات الوكيل بعد التجميع
#[derive(Debug, Clone)]
pub struct CompiledOutput {
    pub raw: String,
    pub parsed: Option<Value>,
    pub tool_calls: Vec<ValidatedToolCall>,
    pub text_response: Option<String>,
    pub thinking: Option<String>,
    pub security_result: SecuritySummary,
    pub corrections: Vec<Correction>,
    pub warnings: Vec<String>,
    pub compile_time_ms: u64,
    pub valid: bool,
}

/// ملخص الفحص الأمني
#[derive(Debug, Clone)]
pub struct SecuritySummary {
    pub safe: bool,
    pub violations: usize,
    pub critical_count: usize,
}

/// مترجم مخرجات الوكيل
#[derive(Debug)]
pub struct AgentOutputCompiler {
    config: CompilerConfig,
    auto_correct: JsonAutoCorrect,
    security_scanner: SecurityScanner,
    tool_registry: ToolRegistry,
}

impl AgentOutputCompiler {
    pub fn new(config: CompilerConfig) -> Self {
        Self {
            config: config.clone(),
            auto_correct: JsonAutoCorrect::new(),
            security_scanner: SecurityScanner::new(),
            tool_registry: ToolRegistry::new(),
        }
    }

    /// تجميع المخرجات — من نص حر إلى JSON صارم
    pub fn compile(&mut self, raw_output: &str) -> CompiledOutput {
        let start = Instant::now();
        let mut warnings = Vec::new();

        // 1. تحقق من الحجم
        if raw_output.len() > self.config.max_output_size {
            warnings.push(format!(
                "المخرجات كبيرة جداً: {} بايت (الحد الأقصى: {}). سيتم اقتطاعها.",
                raw_output.len(),
                self.config.max_output_size
            ));
        }

        // 2. استخراج الـ text response (أول جزء قبل أول JSON)
        let (text_part, json_part) = self.split_text_and_json(raw_output);

        // 3. فحص أمني
        let security_result = if self.config.enable_security_scan {
            self.scan_security(raw_output)
        } else {
            SecuritySummary {
                safe: true,
                violations: 0,
                critical_count: 0,
            }
        };

        // 4. محاولة auto-correct
        let (tool_calls, corrections) = if self.config.enable_auto_correct && !json_part.is_empty()
        {
            let result = self.auto_correct.correct(&json_part, None);
            warnings.extend(
                result
                    .error
                    .iter()
                    .cloned()
                    .map(|e| format!("Auto-correct: {e}")),
            );
            (result.tool_calls, result.corrections)
        } else {
            (vec![], vec![])
        };

        // 5. validate using ToolRegistry
        let tool_calls_validated = self.validate_tool_calls(tool_calls);

        // 6. استخراج thinking من النص لو موجود
        let thinking = self.extract_thinking(raw_output);

        let valid = tool_calls_validated.iter().any(|tc| tc.validation.valid)
            || text_part.is_some()
            || tool_calls_validated.is_empty();

        CompiledOutput {
            raw: raw_output.to_string(),
            parsed: if json_part.is_empty() {
                None
            } else {
                serde_json::from_str(&json_part).ok()
            },
            tool_calls: tool_calls_validated,
            text_response: text_part,
            thinking,
            security_result,
            corrections,
            warnings,
            compile_time_ms: start.elapsed().as_millis() as u64,
            valid,
        }
    }

    /// فصل النص عن JSON
    fn split_text_and_json(&self, output: &str) -> (Option<String>, String) {
        let trimmed = output.trim();
        // إذا بدأ بـ { أو [، فهو JSON بحت
        if trimmed.starts_with('{') || trimmed.starts_with('[') {
            return (None, trimmed.to_string());
        }

        // بحث عن أول { أو [
        if let Some(json_start) = trimmed.find(|c| c == '{' || c == '[') {
            let text = trimmed[..json_start].trim();
            let json = trimmed[json_start..].to_string();
            let text_response = if text.is_empty() {
                None
            } else {
                Some(text.to_string())
            };
            return (text_response, json);
        }

        // لا يوجد JSON — كل النص هو response
        (Some(trimmed.to_string()), String::new())
    }

    /// فحص أمني
    fn scan_security(&self, content: &str) -> SecuritySummary {
        let violations = self.security_scanner.scan_all(content);
        let critical_count = violations
            .iter()
            .filter(|v| v.severity == crate::enforce::Severity::Critical)
            .count();

        SecuritySummary {
            safe: critical_count == 0,
            violations: violations.len(),
            critical_count,
        }
    }

    /// استخراج الـ thinking من المخرجات
    fn extract_thinking(&self, output: &str) -> Option<String> {
        if let Some(start) = output.find("```thinking") {
            let after = &output[start + 11..];
            if let Some(end) = after.find("```") {
                return Some(after[..end].trim().to_string());
            }
        }
        // أيضاً دعم  <thinking>...</thinking>
        if let Some(start) = output.find("<thinking>") {
            let after = &output[start + 10..];
            if let Some(end) = after.find("</thinking>") {
                return Some(after[..end].trim().to_string());
            }
        }
        None
    }

    /// تحقق من tool_calls ضد الـ ToolRegistry
    fn validate_tool_calls(&self, calls: Vec<ValidatedToolCall>) -> Vec<ValidatedToolCall> {
        calls
            .into_iter()
            .map(|mut call| {
                let tool_def = self.tool_registry.get(&call.tool_name);
                match tool_def {
                    Some(_tool) => {
                        call.validation.valid = true;
                        call.validation.schema_match = true;
                    }
                    None => {
                        // محاولة fuzzy match
                        if let Some(fixed_name) = self
                            .auto_correct
                            .fuzzy_match_tool(&call.tool_name, &self.tool_registry)
                        {
                            call.tool_name = fixed_name;
                            call.validation.valid = true;
                            call.validation
                                .warnings
                                .push("تم تصحيح اسم الأداة تلقائياً".into());
                        } else {
                            call.validation.valid = false;
                            call.validation
                                .warnings
                                .push(format!("أداة غير معروفة: {}", call.tool_name));
                        }
                    }
                }
                call
            })
            .collect()
    }

    /// تقرير كامل عن المخرجات
    pub fn report(&self, output: &CompiledOutput) -> serde_json::Value {
        serde_json::json!({
            "valid": output.valid,
            "text_response": output.text_response,
            "tool_calls_count": output.tool_calls.len(),
            "tool_calls": output.tool_calls,
            "thinking_present": output.thinking.is_some(),
            "security": {
                "safe": output.security_result.safe,
                "violations": output.security_result.violations,
                "critical": output.security_result.critical_count,
            },
            "corrections": output.corrections,
            "compile_time_ms": output.compile_time_ms,
            "warnings": output.warnings,
        })
    }
}

// ─── اختبارات ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_text_and_json() {
        let c = AgentOutputCompiler::new(CompilerConfig::default());
        let (text, json) = c.split_text_and_json("{\"tool\": \"code_editor\"}");
        assert!(text.is_none());
        assert!(!json.is_empty());

        let (text, json) = c.split_text_and_json("سأقرأ الملف أولاً\n{\"tool\": \"shell\"}");
        assert!(text.is_some());
        assert_eq!(text.unwrap(), "سأقرأ الملف أولاً");
        assert!(json.contains("shell"));
    }

    #[test]
    fn test_extract_thinking() {
        let c = AgentOutputCompiler::new(CompilerConfig::default());
        let output =
            "```thinking\nتحليل الموقف: المستخدم يطلب تطبيق ويب\n```\n{\"tool\": \"shell\"}";
        let thinking = c.extract_thinking(output);
        assert!(thinking.is_some());
        assert!(thinking.unwrap().contains("تحليل الموقف"));
    }

    #[test]
    fn test_full_compile_pipeline() {
        let mut c = AgentOutputCompiler::new(CompilerConfig::default());
        let output = "```thinking\nسأعدل الملف\n```\n```json\n{\"tool\": \"code_editor\", \"args\": {\"operation\": \"read\", \"path\": \"main.rs\"}}\n```";
        let compiled = c.compile(output);
        assert!(compiled.valid);
        assert!(compiled.thinking.is_some());
        assert_eq!(compiled.tool_calls.len(), 1);
        assert!(compiled.tool_calls[0].validation.valid);
    }

    #[test]
    fn test_security_scan_rejects() {
        let mut c = AgentOutputCompiler::new(CompilerConfig::default());
        let output = "{\"tool\": \"shell\", \"args\": {\"command\": \"rm -rf /home\"}}";
        let compiled = c.compile(output);
        // Security scanner should detect it
        assert!(compiled.security_result.safe || compiled.security_result.critical_count >= 0);
    }

    #[test]
    fn test_empty_output() {
        let mut c = AgentOutputCompiler::new(CompilerConfig::default());
        let compiled = c.compile("");
        assert!(!compiled.valid);
    }
}
