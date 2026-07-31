//! # Agent Compiler — مترجم خاص للوكيل
//!
//! يصحح JSON tool calls تلقائياً، يتحقق من المخرجات، يرفض المهلة.
//!
//! ## المكونات
//! - `auto_correct.rs` — JSON Auto-Corrector + Fuzzy Tool Matching
//! - `output.rs` — AgentOutputCompiler مع validation صارم
//! - `locks.rs` — Programmatic Locks Compiler
//! - `interpreter.rs` — Lock Interpreter for compliance checking

pub mod auto_correct;
pub mod interpreter;
pub mod locks;
pub mod output;

use serde::{Deserialize, Serialize};

/// نتيجة التجميع
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompileResult {
    pub success: bool,
    pub parsed: Option<serde_json::Value>,
    pub tool_calls: Vec<ValidatedToolCall>,
    pub corrections: Vec<Correction>,
    pub warnings: Vec<String>,
    pub errors: Vec<CompileError>,
    pub compile_time_ms: u64,
}

/// استدعاء أداة مُتحقق منه
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidatedToolCall {
    pub tool_name: String,
    pub arguments: serde_json::Value,
    pub original: String,
    pub corrected: bool,
    pub validation: ToolCallValidation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallValidation {
    pub valid: bool,
    pub schema_match: bool,
    pub security_ok: bool,
    pub warnings: Vec<String>,
}

/// تصحيح تم تطبيقه
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Correction {
    pub kind: CorrectionKind,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CorrectionKind {
    MissingQuotes,      // أضفنا "" حول المفاتيح
    TrailingComma,      // أزلنا فاصلة زائدة
    FixedSingleQuotes,  // حوّلنا ' إلى "
    AddedMissingField,  // أضفنا حقل مطلوب
    ToolNameFuzzyMatch, // صححنا اسم الأداة
    RemovedMarkdown,    // أزلنا ```json
    FixedTypes,         // صححنا أنواع (string→number)
    RemovedExtraFields, // أزلنا حقول غير معروفة
}

/// خطأ تجميع
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompileError {
    pub code: CompileErrorCode,
    pub message: String,
    pub position: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CompileErrorCode {
    ParseError,
    SchemaViolation,
    UnknownTool,
    SecurityViolation,
    EmptyOutput,
    TooManyCalls,
    Timeout,
}
