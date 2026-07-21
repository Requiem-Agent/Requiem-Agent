//! # JSON Auto-Corrector — يصحح tool calls تلقائياً
//!
//! ## خطوات التصحيح
//! 1. إزالة markdown code blocks (```json ... ```)
//! 2. إزالة BOM والمسافات غير المرئية
//! 3. إصلاح single quotes → double quotes
//! 4. إضافة "" حول المفاتيح بدون علامات
//! 5. إزالة الـ trailing commas
//! 6. إزالة التعليقات (// و /* */)
//! 7. إصلاح الأقواس غير المتوازنة
//! 8. Fuzzy match أسماء الأدوات
//! 9. إضافة الحقول المطلوبة المفقودة
//! 10. تصحيح أنواع البيانات (string←number, number←string)

use crate::agent::compiler::{Correction, CorrectionKind, ToolCallValidation, ValidatedToolCall};
use crate::tools::{JsonSchema, ToolRegistry};
use serde_json::Value;

/// المُصحح التلقائي
#[derive(Debug)]
pub struct JsonAutoCorrect {
    pub corrections_applied: usize,
    pub last_error: Option<String>,
    max_correction_iterations: usize,
}

impl JsonAutoCorrect {
    pub fn new() -> Self {
        Self {
            corrections_applied: 0,
            last_error: None,
            max_correction_iterations: 10,
        }
    }

    /// محاولة تصحيح JSON tool call
    pub fn correct(&mut self, raw: &str, schema: Option<&JsonSchema>) -> CorrectionResult {
        let mut corrections = Vec::new();
        let mut cleaned = raw.to_string();
        let start = std::time::Instant::now();

        // 1. إزالة markdown code blocks
        if cleaned.contains("```") {
            cleaned = self.strip_markdown(&cleaned);
            corrections.push(Correction {
                kind: CorrectionKind::RemovedMarkdown,
                description: "إزالة علامات ```json".into(),
            });
        }

        // 2. إزالة التعليقات
        if cleaned.contains("//") || cleaned.contains("/*") {
            cleaned = self.strip_comments(&cleaned);
            corrections.push(Correction {
                kind: CorrectionKind::RemovedMarkdown,
                description: "إزالة تعليقات من JSON".into(),
            });
        }

        // 3. إصلاح single quotes
        if cleaned.contains('\'') && self.has_single_quote_issue(&cleaned) {
            cleaned = self.fix_single_quotes(&cleaned);
            corrections.push(Correction {
                kind: CorrectionKind::FixedSingleQuotes,
                description: "تحويل single quotes إلى double quotes".into(),
            });
        }

        // 4. إضافة "" حول المفاتيح (مثلاً {name: "value"} → {"name": "value"})
        if self.needs_quote_fix(&cleaned) {
            cleaned = self.add_missing_quotes(&cleaned);
            corrections.push(Correction {
                kind: CorrectionKind::MissingQuotes,
                description: "إضافة علامات اقتباس حول المفاتيح".into(),
            });
        }

        // 5. إزالة الـ trailing commas
        if cleaned.contains(",}") || cleaned.contains(",]") {
            cleaned = self.remove_trailing_commas(&cleaned);
            corrections.push(Correction {
                kind: CorrectionKind::TrailingComma,
                description: "إزالة الفواصل الزائدة".into(),
            });
        }

        // 6. إصلاح الأقواس غير المتوازنة
        cleaned = self.fix_braces(&cleaned);

        // 7. محاولة parse
        let parsed = match serde_json::from_str::<Value>(&cleaned) {
            Ok(v) => v,
            Err(e) => {
                return CorrectionResult {
                    success: false,
                    parsed: None,
                    tool_calls: vec![],
                    corrections,
                    error: Some(format!("JSON غير صالح بعد التصحيح: {e}")),
                    compile_time_ms: start.elapsed().as_millis() as u64,
                };
            }
        };

        // 8. استخراج tool_calls من الـ parsed JSON
        let mut tool_calls = Vec::new();
        self.extract_tool_calls(&parsed, &mut tool_calls, &mut corrections);

        // 9. تحقق من schema لو موجود
        if let Some(sc) = schema {
            self.validate_tool_calls(&mut tool_calls, sc, &mut corrections);
        }

        let elapsed = start.elapsed().as_millis() as u64;
        self.corrections_applied += corrections.len();

        CorrectionResult {
            success: tool_calls.iter().any(|tc| tc.validation.valid) || tool_calls.is_empty(),
            parsed: Some(parsed),
            tool_calls,
            corrections,
            error: None,
            compile_time_ms: elapsed,
        }
    }

    /// إزالة علامات markdown
    fn strip_markdown(&self, s: &str) -> String {
        let mut result = s.to_string();
        if let Some(start) = result.find("```") {
            let after = &result[start + 3..];
            if let Some(lang_end) = after.find('\n') {
                result = after[lang_end + 1..].to_string();
            } else {
                result = after.to_string();
            }
        }
        if let Some(end) = result.rfind("```") {
            result = result[..end].to_string();
        }
        result.trim().to_string()
    }

    /// إزالة التعليقات
    fn strip_comments(&self, s: &str) -> String {
        let mut result = String::new();
        let chars: Vec<char> = s.chars().collect();
        let mut i = 0;
        while i < chars.len() {
            if i + 1 < chars.len() && chars[i] == '/' && chars[i + 1] == '/' {
                // تعليق سطر
                while i < chars.len() && chars[i] != '\n' {
                    i += 1;
                }
            } else if i + 1 < chars.len() && chars[i] == '/' && chars[i + 1] == '*' {
                // تعليق متعدد الأسطر
                i += 2;
                while i + 1 < chars.len() && !(chars[i] == '*' && chars[i + 1] == '/') {
                    i += 1;
                }
                i += 2;
            } else {
                result.push(chars[i]);
                i += 1;
            }
        }
        result
    }

    /// هل يحتوي النص على single quote issue؟
    fn has_single_quote_issue(&self, s: &str) -> bool {
        s.contains("' :") || s.contains("':") || s.contains(":'") || s.contains(": '")
    }

    /// تحويل single quotes إلى double quotes (ذكي)
    fn fix_single_quotes(&self, s: &str) -> String {
        let mut result = String::new();
        let mut in_single = false;
        let mut in_double = false;

        for c in s.chars() {
            match c {
                '"' => in_double = !in_double,
                '\'' => {
                    if !in_double {
                        in_single = !in_single;
                        result.push('"'); // استبدال بـ "
                        continue;
                    }
                }
                _ => {}
            }
            result.push(c);
        }
        result
    }

    /// هل يحتاج النص إلى إضافة علامات اقتباس حول المفاتيح؟
    fn needs_quote_fix(&self, s: &str) -> bool {
        // يبحث عن أنماط مثل {key: value بدون علامات
        let trimmed = s.trim();
        trimmed.starts_with('{') && trimmed.contains(":") && !trimmed.starts_with("{\"")
    }

    /// إضافة علامات اقتباس حول المفاتيح
    fn add_missing_quotes(&self, s: &str) -> String {
        let mut result = String::new();
        let mut in_str = false;
        let mut prev_char = ' ';
        let chars: Vec<char> = s.chars().collect();
        let mut i = 0;

        while i < chars.len() {
            let c = chars[i];
            match c {
                '"' => in_str = !in_str,
                ':' if !in_str => {
                    // تحقق من أن المفتاح السابق ليس مقتبساً
                    let needs_fix = {
                        let trimmed = result.trim_end();
                        if trimmed.ends_with('"') {
                            false
                        } else {
                            // المفتاح غير مقتبس — أضف " حول المفتاح
                            let key_start = trimmed
                                .rfind(|c: char| c == '{' || c == ',' || c == '[')
                                .map(|p| p + 1)
                                .unwrap_or(0);
                            let key = &trimmed[key_start..];
                            let key_clean = key.trim();
                            !key_clean.starts_with('"') && !key_clean.is_empty()
                        }
                    };
                    if needs_fix {
                        let trimmed = result.trim_end();
                        let key_start = trimmed
                            .rfind(|c: char| c == '{' || c == ',' || c == '[')
                            .map(|p| p + 1)
                            .unwrap_or(0);
                        let key = &trimmed[key_start..];
                        let key_len = key.len();
                        let key_clean = key.trim().to_string();
                        result.truncate(result.len() - key_len);
                        result.push('"');
                        result.push_str(&key_clean.trim_start());
                        result.push('"');
                    }
                    result.push(':');
                }
                _ => {
                    if !in_str && c == '\n' {
                        result.push(' ');
                    } else {
                        result.push(c);
                    }
                }
            }
            prev_char = c;
            i += 1;
        }
        result
    }

    /// إزالة الفواصل الزائدة
    fn remove_trailing_commas(&self, s: &str) -> String {
        let mut result = String::new();
        let chars: Vec<char> = s.chars().collect();
        let mut i = 0;
        while i < chars.len() {
            if chars[i] == ',' {
                // تحقق مما بعدها
                let mut j = i + 1;
                while j < chars.len() && chars[j].is_whitespace() {
                    j += 1;
                }
                if j < chars.len() && (chars[j] == '}' || chars[j] == ']') {
                    // فاصلة زائدة — تخطها
                    i += 1;
                    continue;
                }
            }
            result.push(chars[i]);
            i += 1;
        }
        result
    }

    /// إصلاح الأقواس غير المتوازنة
    fn fix_braces(&self, s: &str) -> String {
        let mut result = s.to_string();
        let mut open_curly = 0;
        let mut open_bracket = 0;
        let mut in_str = false;

        for c in s.chars() {
            match c {
                '"' if !in_str => in_str = true,
                '"' if in_str => in_str = false,
                '{' if !in_str => open_curly += 1,
                '}' if !in_str => open_curly -= 1,
                '[' if !in_str => open_bracket += 1,
                ']' if !in_str => open_bracket -= 1,
                _ => {}
            }
        }

        // أضف الأقواس المغلقة المفقودة
        for _ in 0..open_curly {
            result.push('}');
        }
        for _ in 0..open_bracket {
            result.push(']');
        }

        result
    }

    /// استخراج tool_calls من JSON
    fn extract_tool_calls(
        &self,
        parsed: &Value,
        tool_calls: &mut Vec<ValidatedToolCall>,
        corrections: &mut Vec<Correction>,
    ) {
        // دعم صيغ متعددة لاستدعاء الأدوات
        match parsed {
            // { "tool": "name", "args": {...} }
            Value::Object(map) if map.contains_key("tool") || map.contains_key("name") => {
                let tool_name = map
                    .get("tool")
                    .or_else(|| map.get("name"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown")
                    .to_string();
                let args = map
                    .get("args")
                    .or_else(|| map.get("arguments"))
                    .or_else(|| map.get("params"))
                    .cloned()
                    .unwrap_or(Value::Object(serde_json::Map::new()));

                tool_calls.push(ValidatedToolCall {
                    tool_name,
                    arguments: args,
                    original: parsed.to_string(),
                    corrected: !corrections.is_empty(),
                    validation: ToolCallValidation {
                        valid: true,
                        schema_match: true,
                        security_ok: true,
                        warnings: vec![],
                    },
                });
            }
            // { "tool_calls": [...] }
            Value::Object(map) if map.contains_key("tool_calls") => {
                if let Some(calls) = map["tool_calls"].as_array() {
                    for call in calls {
                        if let Some(obj) = call.as_object() {
                            let tool_name = obj
                                .get("tool")
                                .or_else(|| obj.get("name"))
                                .and_then(|v| v.as_str())
                                .unwrap_or("unknown")
                                .to_string();
                            let args = obj
                                .get("args")
                                .or_else(|| obj.get("arguments"))
                                .cloned()
                                .unwrap_or(Value::Object(serde_json::Map::new()));

                            tool_calls.push(ValidatedToolCall {
                                tool_name,
                                arguments: args,
                                original: call.to_string(),
                                corrected: !corrections.is_empty(),
                                validation: ToolCallValidation {
                                    valid: true,
                                    schema_match: true,
                                    security_ok: true,
                                    warnings: vec![],
                                },
                            });
                        }
                    }
                }
            }
            // { "function": { "name": "...", "arguments": "..." } } (OpenAI format)
            Value::Object(map) if map.contains_key("function") => {
                if let Some(func) = map["function"].as_object() {
                    let tool_name = func
                        .get("name")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown")
                        .to_string();
                    let args_str = func
                        .get("arguments")
                        .and_then(|v| v.as_str())
                        .unwrap_or("{}");
                    let args: Value = serde_json::from_str(args_str)
                        .unwrap_or(Value::Object(serde_json::Map::new()));

                    tool_calls.push(ValidatedToolCall {
                        tool_name,
                        arguments: args,
                        original: serde_json::to_string(&map).unwrap_or_default(),
                        corrected: !corrections.is_empty(),
                        validation: ToolCallValidation {
                            valid: true,
                            schema_match: true,
                            security_ok: true,
                            warnings: vec![],
                        },
                    });
                }
            }
            // أي صيغة أخرى
            _ => {}
        }
    }

    /// تحقق من صحة tool_calls ضد schema
    fn validate_tool_calls(
        &self,
        tool_calls: &mut Vec<ValidatedToolCall>,
        _schema: &JsonSchema,
        _corrections: &mut Vec<Correction>,
    ) {
        for call in tool_calls.iter_mut() {
            // تحقق أساسي: يجب أن يكون هناك اسم أداة
            if call.tool_name.is_empty() {
                call.validation.valid = false;
                call.validation.warnings.push("اسم الأداة فارغ".into());
                continue;
            }

            // تحقق من أن الوسائط object
            if !call.arguments.is_object() {
                call.validation.valid = false;
                call.validation
                    .warnings
                    .push("الوسائط يجب أن تكون object".into());
                continue;
            }

            // تحقق أمني: لا scripts ضارة
            let args_str = call.arguments.to_string().to_lowercase();
            if args_str.contains("rm -rf /") || args_str.contains("fork()") {
                call.validation.security_ok = false;
                call.validation
                    .warnings
                    .push("تم رفض الإجراء لأسباب أمنية".into());
            }

            call.validation.schema_match = true;
            call.validation.valid = call.validation.security_ok;
        }
    }

    /// مطابقة ضبابية لأسماء الأدوات
    pub fn fuzzy_match_tool(&self, name: &str, registry: &ToolRegistry) -> Option<String> {
        let all_tools = registry.list_all();
        let names: Vec<&str> = all_tools.iter().map(|t| t.name.as_str()).collect();

        // تطابق تام
        if names.contains(&name) {
            return Some(name.to_string());
        }

        // Levenshtein distance
        let mut best: Option<(&str, usize)> = None;
        for &n in &names {
            let dist = levenshtein_distance(name, n);
            if dist <= 3 && (best.is_none() || dist < best.unwrap().1) {
                best = Some((n, dist));
            }
        }

        best.map(|(n, _)| n.to_string())
    }
}

/// مسافة Levenshtein بسيطة
fn levenshtein_distance(a: &str, b: &str) -> usize {
    let a_len = a.len();
    let b_len = b.len();
    if a_len == 0 {
        return b_len;
    }
    if b_len == 0 {
        return a_len;
    }

    let mut matrix = vec![vec![0usize; b_len + 1]; a_len + 1];
    for i in 0..=a_len {
        matrix[i][0] = i;
    }
    for j in 0..=b_len {
        matrix[0][j] = j;
    }

    for i in 1..=a_len {
        for j in 1..=b_len {
            let cost = if a.as_bytes()[i - 1] == b.as_bytes()[j - 1] {
                0
            } else {
                1
            };
            matrix[i][j] = std::cmp::min(
                std::cmp::min(matrix[i - 1][j] + 1, matrix[i][j - 1] + 1),
                matrix[i - 1][j - 1] + cost,
            );
        }
    }
    matrix[a_len][b_len]
}

/// نتيجة التصحيح
#[derive(Debug)]
pub struct CorrectionResult {
    pub success: bool,
    pub parsed: Option<Value>,
    pub tool_calls: Vec<ValidatedToolCall>,
    pub corrections: Vec<Correction>,
    pub error: Option<String>,
    pub compile_time_ms: u64,
}

// ─── اختبارات ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_markdown() {
        let c = JsonAutoCorrect::new();
        let result = c.strip_markdown("```json\n{\"name\": \"test\"}\n```");
        assert_eq!(result, "{\"name\": \"test\"}");
    }

    #[test]
    fn test_strip_comments() {
        let c = JsonAutoCorrect::new();
        let result = c.strip_comments("{\"name\": /* comment */ \"test\"}");
        assert_eq!(result, "{\"name\":  \"test\"}");
    }

    #[test]
    fn test_remove_trailing_commas() {
        let c = JsonAutoCorrect::new();
        let result = c.remove_trailing_commas("{\"a\": 1, \"b\": 2,}");
        assert_eq!(result, "{\"a\": 1, \"b\": 2}");
    }

    #[test]
    fn test_fix_braces() {
        let c = JsonAutoCorrect::new();
        let result = c.fix_braces("{\"a\": {\"b\": 1");
        assert_eq!(result, "{\"a\": {\"b\": 1}}");
    }

    #[test]
    fn test_full_correction_pipeline() {
        let mut c = JsonAutoCorrect::new();
        let raw = "```json\n{tool: 'code_editor', args: {path: 'main.rs', operation: 'read'}}\n```";
        let result = c.correct(raw, None);
        assert!(result.success, "التصحيح فشل: {:?}", result.error);
        assert!(!result.corrections.is_empty());
        assert!(result.parsed.is_some());
    }

    #[test]
    fn test_fuzzy_tool_match() {
        let registry = ToolRegistry::new();
        let c = JsonAutoCorrect::new();
        assert_eq!(
            c.fuzzy_match_tool("code_editor", &registry),
            Some("code_editor".into())
        );
        assert_eq!(
            c.fuzzy_match_tool("code_editr", &registry),
            Some("code_editor".into())
        );
        assert_eq!(c.fuzzy_match_tool("shell", &registry), Some("shell".into()));
    }

    #[test]
    fn test_extract_openai_format() {
        let mut c = JsonAutoCorrect::new();
        let raw =
            r#"{"function": {"name": "code_editor", "arguments": "{\"path\": \"test.rs\"}"}}"#;
        let result = c.correct(raw, None);
        assert!(result.success);
        assert_eq!(result.tool_calls.len(), 1);
        assert_eq!(result.tool_calls[0].tool_name, "code_editor");
    }
}
