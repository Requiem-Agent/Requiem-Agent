//! # Programmatic Locks Compiler — مترجم الأقفال البرمجية
//!
//! ## المبدأ
//! الكومبايلر مسؤول عن اختبار سينتاكس الملفات والتحقق من التزام الوكيل
//! بالشيما والقواعد البرمجية المحددة.
//!
//! ## المكونات
//! 1. **SyntaxChecker** — فحص السينتاكس
//! 2. **SchemaValidator** — التحقق من الشيما
//! 3. **LockEnforcer** — فرض الأقفال
//! 4. **OutputCompiler** — تجميع المخرجات

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{debug, info, warn};

/// نوع الملف
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum FileType {
    Rust,
    TypeScript,
    JavaScript,
    Python,
    Json,
    Yaml,
    Toml,
    Markdown,
    Html,
    Css,
    Sql,
    Unknown,
}

impl FileType {
    pub fn from_extension(ext: &str) -> Self {
        match ext.to_lowercase().as_str() {
            "rs" => Self::Rust,
            "ts" | "tsx" => Self::TypeScript,
            "js" | "jsx" => Self::JavaScript,
            "py" => Self::Python,
            "json" => Self::Json,
            "yaml" | "yml" => Self::Yaml,
            "toml" => Self::Toml,
            "md" => Self::Markdown,
            "html" => Self::Html,
            "css" => Self::Css,
            "sql" => Self::Sql,
            _ => Self::Unknown,
        }
    }

    pub fn name(&self) -> &str {
        match self {
            Self::Rust => "rust",
            Self::TypeScript => "typescript",
            Self::JavaScript => "javascript",
            Self::Python => "python",
            Self::Json => "json",
            Self::Yaml => "yaml",
            Self::Toml => "toml",
            Self::Markdown => "markdown",
            Self::Html => "html",
            Self::Css => "css",
            Self::Sql => "sql",
            Self::Unknown => "unknown",
        }
    }
}

/// مستوى صرامة الكومبايلر
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum StrictnessLevel {
    /// أساسي — فقط الأخطاء الجلية
    Basic,
    /// متوسط — يشمل التحذيرات
    Medium,
    /// صارم — يشمل كل شيء
    Strict,
    /// صارم جداً — أخطاء بسيطة توقف التنفيذ
    VeryStrict,
}

/// نتيجة فحص السينتاكس
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyntaxCheckResult {
    pub file_type: FileType,
    pub valid: bool,
    pub errors: Vec<SyntaxError>,
    pub warnings: Vec<SyntaxWarning>,
    pub suggestions: Vec<String>,
}

/// خطأ سينتاكس
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyntaxError {
    pub line: usize,
    pub column: usize,
    pub message: String,
    pub severity: ErrorSeverity,
    pub auto_fixable: bool,
}

/// تحذير سينتاكس
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyntaxWarning {
    pub line: usize,
    pub column: usize,
    pub message: String,
    pub suggestion: String,
}

/// خطورة الخطأ
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ErrorSeverity {
    Low,
    Medium,
    High,
    Critical,
}

/// مترجم الأقفال البرمجية
pub struct LockCompiler {
    strictness: StrictnessLevel,
    supported_languages: Vec<FileType>,
    custom_rules: Vec<CustomRule>,
}

/// قاعدة مخصصة
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomRule {
    pub name: String,
    pub description: String,
    pub pattern: String,
    pub replacement: Option<String>,
    pub severity: ErrorSeverity,
}

impl LockCompiler {
    /// إنشاء مترجم جديد
    pub fn new(strictness: StrictnessLevel) -> Self {
        info!("LockCompiler initialized with strictness: {:?}", strictness);

        Self {
            strictness,
            supported_languages: vec![
                FileType::Rust,
                FileType::TypeScript,
                FileType::JavaScript,
                FileType::Python,
                FileType::Json,
                FileType::Yaml,
                FileType::Toml,
                FileType::Markdown,
            ],
            custom_rules: Vec::new(),
        }
    }

    /// فحص سينتاكس ملف
    pub fn check_syntax(&self, content: &str, file_type: FileType) -> SyntaxCheckResult {
        match file_type {
            FileType::Rust => self.check_rust_syntax(content),
            FileType::TypeScript | FileType::JavaScript => self.check_ts_syntax(content),
            FileType::Python => self.check_python_syntax(content),
            FileType::Json => self.check_json_syntax(content),
            _ => SyntaxCheckResult {
                file_type,
                valid: true,
                errors: Vec::new(),
                warnings: Vec::new(),
                suggestions: Vec::new(),
            },
        }
    }

    /// فحص سينتاكس Rust
    fn check_rust_syntax(&self, content: &str) -> SyntaxCheckResult {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();
        let mut suggestions = Vec::new();

        let lines: Vec<&str> = content.lines().collect();

        for (i, line) in lines.iter().enumerate() {
            let line_num = i + 1;

            // فحص الأقواس غير المتطابقة
            let open_braces = line.matches('{').count();
            let close_braces = line.matches('}').count();
            if open_braces != close_braces {
                if self.strictness == StrictnessLevel::Strict
                    || self.strictness == StrictnessLevel::VeryStrict
                {
                    warnings.push(SyntaxWarning {
                        line: line_num,
                        column: 0,
                        message: "Unbalanced braces detected".to_string(),
                        suggestion: "Check for matching braces".to_string(),
                    });
                }
            }

            // فحص unsafe blocks
            if line.contains("unsafe") && self.strictness == StrictnessLevel::VeryStrict {
                errors.push(SyntaxError {
                    line: line_num,
                    column: line.find("unsafe").unwrap_or(0),
                    message: "Unsafe block detected in strict mode".to_string(),
                    severity: ErrorSeverity::High,
                    auto_fixable: false,
                });
            }

            // فحص unwrap() calls
            if line.contains(".unwrap()") && self.strictness != StrictnessLevel::Basic {
                warnings.push(SyntaxWarning {
                    line: line_num,
                    column: line.find(".unwrap()").unwrap_or(0),
                    message: "unwrap() call detected — consider using expect() or ? operator"
                        .to_string(),
                    suggestion: "Replace unwrap() with .expect(\"message\") or use ? operator"
                        .to_string(),
                });
            }

            // فحص TODO comments
            if line.contains("TODO") || line.contains("FIXME") || line.contains("HACK") {
                suggestions.push(format!("Line {}: Consider resolving TODO/FIXME", line_num));
            }
        }

        // فحص imports
        if !content.contains("use ") && !content.contains("import ") {
            suggestions.push("Consider adding necessary imports".to_string());
        }

        SyntaxCheckResult {
            file_type: FileType::Rust,
            valid: errors.is_empty(),
            errors,
            warnings,
            suggestions,
        }
    }

    /// فحص سينتاكس TypeScript
    fn check_ts_syntax(&self, content: &str) -> SyntaxCheckResult {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();
        let mut suggestions = Vec::new();

        let lines: Vec<&str> = content.lines().collect();

        for (i, line) in lines.iter().enumerate() {
            let line_num = i + 1;

            // فحص any type
            if line.contains(": any") && self.strictness != StrictnessLevel::Basic {
                errors.push(SyntaxError {
                    line: line_num,
                    column: line.find(": any").unwrap_or(0),
                    message: "Type 'any' is not allowed in strict mode".to_string(),
                    severity: if self.strictness == StrictnessLevel::VeryStrict {
                        ErrorSeverity::Critical
                    } else {
                        ErrorSeverity::Medium
                    },
                    auto_fixable: true,
                });
            }

            // فحص @ts-ignore
            if line.contains("@ts-ignore") || line.contains("@ts-expect-error") {
                errors.push(SyntaxError {
                    line: line_num,
                    column: 0,
                    message: "TypeScript directive override detected".to_string(),
                    severity: ErrorSeverity::High,
                    auto_fixable: false,
                });
            }

            // فحص console.log
            if line.contains("console.log") && self.strictness != StrictnessLevel::Basic {
                warnings.push(SyntaxWarning {
                    line: line_num,
                    column: 0,
                    message: "console.log detected — consider using proper logging".to_string(),
                    suggestion: "Replace with a logging library".to_string(),
                });
            }

            // فحص var keyword
            if line.trim().starts_with("var ") {
                warnings.push(SyntaxWarning {
                    line: line_num,
                    column: 0,
                    message: "'var' keyword detected — prefer 'const' or 'let'".to_string(),
                    suggestion: "Use 'const' or 'let' instead of 'var'".to_string(),
                });
            }
        }

        SyntaxCheckResult {
            file_type: FileType::TypeScript,
            valid: errors.is_empty(),
            errors,
            warnings,
            suggestions,
        }
    }

    /// فحص سينتاكس Python
    fn check_python_syntax(&self, content: &str) -> SyntaxCheckResult {
        let mut warnings = Vec::new();
        let mut suggestions = Vec::new();

        let lines: Vec<&str> = content.lines().collect();

        for (i, line) in lines.iter().enumerate() {
            let line_num = i + 1;

            // فحص bare except
            if line.trim().starts_with("except:") {
                warnings.push(SyntaxWarning {
                    line: line_num,
                    column: 0,
                    message: "Bare except detected — specify exception type".to_string(),
                    suggestion: "Use 'except Exception:' or more specific exception type"
                        .to_string(),
                });
            }

            // فحص mutable default arguments
            if line.contains("def ") && (line.contains("=[]") || line.contains("={}")) {
                warnings.push(SyntaxWarning {
                    line: line_num,
                    column: 0,
                    message: "Mutable default argument detected".to_string(),
                    suggestion: "Use None as default and initialize inside function".to_string(),
                });
            }

            // فحص print statements
            if line.trim().starts_with("print(") && self.strictness != StrictnessLevel::Basic {
                suggestions.push(format!(
                    "Line {}: Consider using logging module instead of print()",
                    line_num
                ));
            }
        }

        SyntaxCheckResult {
            file_type: FileType::Python,
            valid: true,
            errors: Vec::new(),
            warnings,
            suggestions,
        }
    }

    /// فحص سينتاكس JSON
    fn check_json_syntax(&self, content: &str) -> SyntaxCheckResult {
        let mut errors = Vec::new();

        match serde_json::from_str::<serde_json::Value>(content) {
            Ok(_) => {}
            Err(e) => {
                let line = e.line();
                let column = e.column();
                errors.push(SyntaxError {
                    line,
                    column,
                    message: format!("JSON parse error: {}", e),
                    severity: ErrorSeverity::Critical,
                    auto_fixable: false,
                });
            }
        }

        SyntaxCheckResult {
            file_type: FileType::Json,
            valid: errors.is_empty(),
            errors,
            warnings: Vec::new(),
            suggestions: Vec::new(),
        }
    }

    /// توليد مخرجات موحدة
    pub fn compile_output(&self, content: &str, file_type: FileType, mode: &str) -> CompiledOutput {
        let syntax_result = self.check_syntax(content, file_type.clone());

        CompiledOutput {
            original_content: content.to_string(),
            file_type,
            syntax_valid: syntax_result.valid,
            errors: syntax_result.errors,
            warnings: syntax_result.warnings,
            suggestions: syntax_result.suggestions,
            compiled_at: chrono::Utc::now().to_rfc3339(),
            mode: mode.to_string(),
            strictness: self.strictness.clone(),
        }
    }

    /// إضافة قاعدة مخصصة
    pub fn add_custom_rule(&mut self, rule: CustomRule) {
        self.custom_rules.push(rule);
    }
}

/// المخرجات المجمعة
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompiledOutput {
    pub original_content: String,
    pub file_type: FileType,
    pub syntax_valid: bool,
    pub errors: Vec<SyntaxError>,
    pub warnings: Vec<SyntaxWarning>,
    pub suggestions: Vec<String>,
    pub compiled_at: String,
    pub mode: String,
    pub strictness: StrictnessLevel,
}

impl Default for LockCompiler {
    fn default() -> Self {
        Self::new(StrictnessLevel::Medium)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lock_compiler_creation() {
        let compiler = LockCompiler::new(StrictnessLevel::Strict);
        assert_eq!(compiler.strictness, StrictnessLevel::Strict);
    }

    #[test]
    fn test_rust_syntax_check() {
        let compiler = LockCompiler::new(StrictnessLevel::Basic);
        let content = r#"
fn main() {
    let x = 5;
    println!("{}", x);
}
"#;
        let result = compiler.check_syntax(content, FileType::Rust);
        assert!(result.valid);
    }

    #[test]
    fn test_typescript_any_detection() {
        let compiler = LockCompiler::new(StrictnessLevel::Strict);
        let content = "const x: any = 'hello';";
        let result = compiler.check_syntax(content, FileType::TypeScript);
        assert!(!result.valid);
        assert!(result.errors.iter().any(|e| e.message.contains("any")));
    }

    #[test]
    fn test_json_syntax_check() {
        let compiler = LockCompiler::new(StrictnessLevel::Basic);
        let content = r#"{"key": "value"}"#;
        let result = compiler.check_syntax(content, FileType::Json);
        assert!(result.valid);

        let invalid = r#"{"key": "value""#;
        let result = compiler.check_syntax(invalid, FileType::Json);
        assert!(!result.valid);
    }
}
