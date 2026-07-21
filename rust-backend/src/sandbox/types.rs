//! # Types — أنواع البيانات الأساسية للساندبوكس

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ─── Language ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Language {
    Rust, Python, JavaScript, TypeScript, Bash, Go,
}

impl Language {
    pub fn extension(&self) -> &'static str {
        match self { Self::Rust => "rs", Self::Python => "py", Self::JavaScript | Self::TypeScript => "js", Self::Bash => "sh", Self::Go => "go" }
    }
    pub fn needs_compilation(&self) -> bool { matches!(self, Self::Rust | Self::Go) }
}

pub fn detect_language(code: &str) -> Language {
    let t = code.trim();
    if (t.starts_with("fn ") || code.contains("println!") || code.contains("let mut "))
        && !code.contains("def ") && !code.contains("print(") { return Language::Rust; }
    if t.starts_with("package main") || code.contains("fmt.Println") { return Language::Go; }
    if t.starts_with("def ") || t.starts_with("import ") || code.contains("print(")
        || (code.contains("import ") && code.contains("from ")) { return Language::Python; }
    if code.contains("console.") || code.contains("require(") || code.contains("=>")
        { return Language::JavaScript; }
    if t.starts_with("#!/") || t.starts_with("echo ") { return Language::Bash; }
    Language::Bash
}

// ─── Sandbox Request & Result ──────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxRequest {
    pub code: String,
    pub language: Option<Language>,
    pub timeout_secs: Option<u64>,
    pub env: Option<HashMap<String, String>>,
    pub stream: Option<bool>,
    pub wait_in_queue: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxResult {
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
    pub duration_ms: u64,
    pub language: Language,
    pub compilation_error: Option<String>,
    pub timed_out: bool,
    pub output_truncated: bool,
    pub queued_ms: u64,
    pub had_to_wait: bool,
}

impl SandboxResult {
    pub fn err(language: Language, msg: String) -> Self {
        Self {
            success: false, stdout: String::new(), stderr: msg,
            exit_code: -1, duration_ms: 0, language,
            compilation_error: None, timed_out: false,
            output_truncated: false, queued_ms: 0, had_to_wait: false,
        }
    }
}

// ─── إعدادات الساندبوكس (ثوابت) ───────────────────────────────────────────

pub const MAX_CONCURRENT_COMPILE: usize = 1;
pub const MAX_CONCURRENT_EXEC: usize = 3;
pub const MAX_TOTAL_SANDBOXES: usize = 4;
pub const MAX_PER_USER: usize = 1;
pub const MAX_OUTPUT_SIZE: usize = 524_288;    // 512KB
pub const MAX_TIMEOUT_SECS: u64 = 60;
pub const MAX_COMPILE_TIMEOUT_SECS: u64 = 180;
pub const QUEUE_WAIT_TIMEOUT_SECS: u64 = 30;

// حدود rlimit
pub const RLIMIT_AS_BYTES: u64 = 512 * 1024 * 1024;        // 512MB
pub const RLIMIT_NPROC: u64 = 20;
pub const RLIMIT_NOFILE: u64 = 50;
pub const RLIMIT_FSIZE: u64 = 1 * 1024 * 1024;              // 1MB
pub const RLIMIT_CPU_SECS: u64 = 60;

/// تصنيف الساندبوكس — هل هو للترجمة أم التنفيذ فقط
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SandboxKind {
    /// ترجمة + تشغيل (Rust, Go)
    Heavy,
    /// تشغيل فقط (Python, JS, Bash)
    Light,
}
