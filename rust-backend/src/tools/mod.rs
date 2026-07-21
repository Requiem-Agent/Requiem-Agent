//! # Tool System — نظام الأدوات البرمجي مع JSON Schema
//!
//! كل أداة لها JSON Schema صارم — أي استدعاء بدون schema مرفوض.
//! هذا يضمن أن الوكيل لا يمكنه استخدام الأدوات بطريقة غير متوقعة.
//!
//! ## التصميم (مستوحى من Replit Agent 3 + Mastra)
//! ```
//! Tool Registry ──→ CodeEditorTool
//!               ├── WebSearchTool
//!               ├── WebScrapeTool
//!               ├── ShellTool
//!               ├── ProjectAnalyzerTool
//!               ├── FileTreeTool
//!               ├── MemoryTool
//!               ├── SQLTool
//!               ├── CodeSearchTool (NEW - ripgrep)
//!               ├── FileFinderTool (NEW - fd)
//!               ├── AstParserTool (NEW - tree-sitter)
//!               ├── DiffTool (NEW - similar)
//!               └── VcsTool (NEW - git2)
//! ```

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub mod code_tools;
pub mod search_tools;
pub mod parser_tools;
pub mod diff_tools;
pub mod vcs_tools;

// ─── JSON Schema ─────────────────────────────────────────────────────────────

/// تمثيل JSON Schema مبسط
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonSchema {
    #[serde(rename = "type")]
    pub schema_type: String,
    pub properties: Option<HashMap<String, SchemaProperty>>,
    pub required: Option<Vec<String>>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaProperty {
    #[serde(rename = "type")]
    pub prop_type: String,
    pub description: Option<String>,
    pub enum_values: Option<Vec<String>>,
    pub default: Option<serde_json::Value>,
    pub required: Option<bool>,
}

// ─── Tool Definition ─────────────────────────────────────────────────────────

/// تعريف الأداة — الاسم، الوصف، JSON Schema للمعاملات
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub parameters: JsonSchema,
    pub returns: JsonSchema,
    pub strictness: Strictness,
}

/// مستوى الصرامة للأداة
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Strictness {
    /// تحقق بسيط
    Normal,
    /// تحقق صارم — كل المعاملات مطلوبة
    Strict,
    /// صارم جداً — فحص أمني إضافي
    Critical,
}

// ─── Tool Registry ───────────────────────────────────────────────────────────

/// سجل الأدوات — يحتفظ بجميع الأدوات المتاحة
#[derive(Debug, Clone)]
pub struct ToolRegistry {
    tools: HashMap<String, ToolDefinition>,
}

impl ToolRegistry {
    /// إنشاء سجل جديد مع جميع الأدوات الافتراضية
    pub fn new() -> Self {
        let mut tools = HashMap::new();

        // Code Editor
        tools.insert("code_editor".to_string(), ToolDefinition {
            name: "code_editor".into(),
            description: "قراءة، كتابة، تعديل، أو حذف ملفات في مشروع المستخدم. يدعم عمليات متعددة بالتوازي.".into(),
            parameters: JsonSchema {
                schema_type: "object".into(),
                properties: Some(HashMap::from([
                    ("operation".into(), SchemaProperty {
                        prop_type: "string".into(),
                        description: Some("العملية: read, write, edit, delete, create".into()),
                        enum_values: Some(vec![
                            "read".into(), "write".into(), "edit".into(),
                            "delete".into(), "create".into(),
                        ]),
                        default: None,
                        required: Some(true),
                    }),
                    ("path".into(), SchemaProperty {
                        prop_type: "string".into(),
                        description: Some("مسار الملف (نسبي إلى جذر المستخدم)".into()),
                        enum_values: None,
                        default: None,
                        required: Some(true),
                    }),
                    ("content".into(), SchemaProperty {
                        prop_type: "string".into(),
                        description: Some("محتوى الملف (لـ write/create)".into()),
                        enum_values: None,
                        default: None,
                        required: Some(false),
                    }),
                    ("old_str".into(), SchemaProperty {
                        prop_type: "string".into(),
                        description: Some("النص القديم للاستبدال (لـ edit)".into()),
                        enum_values: None,
                        default: None,
                        required: Some(false),
                    }),
                    ("new_str".into(), SchemaProperty {
                        prop_type: "string".into(),
                        description: Some("النص الجديد (لـ edit)".into()),
                        enum_values: None,
                        default: None,
                        required: Some(false),
                    }),
                ])),
                required: Some(vec!["operation".into(), "path".into()]),
                description: Some("عمليات تعديل الملفات".to_string()),
            },
            returns: JsonSchema {
                schema_type: "object".into(),
                properties: None,
                required: None,
                description: Some("نتيجة العملية: { success, path, content?, error? }".to_string()),
            },
            strictness: Strictness::Strict,
        });

        // Web Search
        tools.insert("web_search".to_string(), ToolDefinition {
            name: "web_search".into(),
            description: "بحث في الإنترنت باستخدام Tavily API. للحصول على معلومات حديثة.".into(),
            parameters: JsonSchema {
                schema_type: "object".into(),
                properties: Some(HashMap::from([
                    ("query".into(), SchemaProperty {
                        prop_type: "string".into(),
                        description: Some("استعلام البحث".into()),
                        enum_values: None,
                        default: None,
                        required: Some(true),
                    }),
                    ("max_results".into(), SchemaProperty {
                        prop_type: "integer".into(),
                        description: Some("عدد النتائج (1-10)".into()),
                        enum_values: None,
                        default: Some(serde_json::json!(5)),
                        required: Some(false),
                    }),
                ])),
                required: Some(vec!["query".into()]),
                description: Some("البحث في الإنترنت".to_string()),
            },
            returns: JsonSchema {
                schema_type: "array".into(),
                properties: None,
                required: None,
                description: Some("قائمة النتائج: [{ title, url, content }]".to_string()),
            },
            strictness: Strictness::Strict,
        });

        // Web Scrape
        tools.insert("web_scrape".to_string(), ToolDefinition {
            name: "web_scrape".into(),
            description: "جلب محتوى صفحة ويب. يستخدم proxy المستخدم للطلبات.".into(),
            parameters: JsonSchema {
                schema_type: "object".into(),
                properties: Some(HashMap::from([
                    ("url".into(), SchemaProperty {
                        prop_type: "string".into(),
                        description: Some("رابط الصفحة".into()),
                        enum_values: None,
                        default: None,
                        required: Some(true),
                    }),
                ])),
                required: Some(vec!["url".into()]),
                description: Some("جلب محتوى صفحة ويب".to_string()),
            },
            returns: JsonSchema {
                schema_type: "object".into(),
                properties: None,
                required: None,
                description: Some("{ url, title, content, status }".to_string()),
            },
            strictness: Strictness::Normal,
        });

        // Shell
        tools.insert("shell".to_string(), ToolDefinition {
            name: "shell".into(),
            description: "تنفيذ أوامر shell في بيئة المستخدم المعزولة.".into(),
            parameters: JsonSchema {
                schema_type: "object".into(),
                properties: Some(HashMap::from([
                    ("command".into(), SchemaProperty {
                        prop_type: "string".into(),
                        description: Some("الأمر المراد تنفيذه".into()),
                        enum_values: None,
                        default: None,
                        required: Some(true),
                    }),
                    ("timeout".into(), SchemaProperty {
                        prop_type: "integer".into(),
                        description: Some("مهلة التنفيذ بالثواني".into()),
                        enum_values: None,
                        default: Some(serde_json::json!(30)),
                        required: Some(false),
                    }),
                ])),
                required: Some(vec!["command".into()]),
                description: Some("تنفيذ أوامر shell".to_string()),
            },
            returns: JsonSchema {
                schema_type: "object".into(),
                properties: None,
                required: None,
                description: Some("{ stdout, stderr, exit_code }".to_string()),
            },
            strictness: Strictness::Critical, // خطير — فحص أمني إضافي
        });

        // File Tree
        tools.insert("file_tree".to_string(), ToolDefinition {
            name: "file_tree".into(),
            description: "عرض هيكل المجلدات والملفات في مشروع المستخدم.".into(),
            parameters: JsonSchema {
                schema_type: "object".into(),
                properties: Some(HashMap::from([
                    ("path".into(), SchemaProperty {
                        prop_type: "string".into(),
                        description: Some("المسار (اختياري — الافتراضي هو الجذر)".into()),
                        enum_values: None,
                        default: Some(serde_json::json!("")),
                        required: Some(false),
                    }),
                    ("depth".into(), SchemaProperty {
                        prop_type: "integer".into(),
                        description: Some("عمق العرض (1-5)".into()),
                        enum_values: None,
                        default: Some(serde_json::json!(3)),
                        required: Some(false),
                    }),
                ])),
                required: Some(vec![]),
                description: Some("عرض هيكل المشروع".to_string()),
            },
            returns: JsonSchema {
                schema_type: "array".into(),
                properties: None,
                required: None,
                description: Some("قائمة الملفات والمجلدات مع الأحجام".to_string()),
            },
            strictness: Strictness::Normal,
        });

        // Project Analyze
        tools.insert("project_analyze".to_string(), ToolDefinition {
            name: "project_analyze".into(),
            description: "تحليل هيكل المشروع بالكامل — اللغات، الملفات الرئيسية، الإحصائيات.".into(),
            parameters: JsonSchema {
                schema_type: "object".into(),
                properties: Some(HashMap::new()),
                required: Some(vec![]),
                description: Some("لا معاملات مطلوبة".to_string()),
            },
            returns: JsonSchema {
                schema_type: "object".into(),
                properties: None,
                required: None,
                description: Some("{ files, file_count, language_summary, main_files }".to_string()),
            },
            strictness: Strictness::Normal,
        });

        // Model Switch
        tools.insert("model_switch".to_string(), ToolDefinition {
            name: "model_switch".into(),
            description: "التبديل بين النماذج المتاحة. لكل مهمة النموذج الأنسب.".into(),
            parameters: JsonSchema {
                schema_type: "object".into(),
                properties: Some(HashMap::from([
                    ("model".into(), SchemaProperty {
                        prop_type: "string".into(),
                        description: Some("معرف النموذج (deepseek-v4-flash-free, mimo-v2.5-free, hy3-free, ...)".into()),
                        enum_values: Some(vec![
                            "deepseek-v4-flash-free".into(),
                            "big-pickle".into(),
                            "mimo-v2.5-free".into(),
                            "hy3-free".into(),
                            "north-mini-code-free".into(),
                            "nemotron-3-ultra-free".into(),
                        ]),
                        default: None,
                        required: Some(true),
                    }),
                    ("reason".into(), SchemaProperty {
                        prop_type: "string".into(),
                        description: Some("سبب التبديل".into()),
                        enum_values: None,
                        default: None,
                        required: Some(false),
                    }),
                ])),
                required: Some(vec!["model".into()]),
                description: Some("التبديل بين النماذج".to_string()),
            },
            returns: JsonSchema {
                schema_type: "object".into(),
                properties: None,
                required: None,
                description: Some("{ success, previous_model, current_model }".to_string()),
            },
            strictness: Strictness::Strict,
        });

        Self { tools }
    }

    /// الحصول على تعريف أداة
    pub fn get(&self, name: &str) -> Option<&ToolDefinition> {
        self.tools.get(name)
    }

    /// الحصول على قائمة بجميع الأدوات (لـ JSON)
    pub fn list_all(&self) -> Vec<&ToolDefinition> {
        self.tools.values().collect()
    }

    /// عدد الأدوات المسجلة
    pub fn count(&self) -> usize {
        self.tools.len()
    }

    /// التحقق من صحة معاملات الأداة ضد JSON Schema
    pub fn validate_params(&self, tool_name: &str, params: &serde_json::Value) -> Result<(), String> {
        let tool = self.tools.get(tool_name)
            .ok_or_else(|| format!("Tool '{tool_name}' not found"))?;

        let schema = &tool.parameters;

        // تحقق من الحقول المطلوبة
        if let Some(required) = &schema.required {
            for field in required {
                if params.get(field).is_none() && !field.is_empty() {
                    return Err(format!("Missing required parameter: '{field}' for tool '{tool_name}'"));
                }
            }
        }

        // تحقق من الأنواع
        if let Some(properties) = &schema.properties {
            for (key, prop) in properties {
                if let Some(value) = params.get(key) {
                    let is_valid = match prop.prop_type.as_str() {
                        "string" => value.is_string(),
                        "integer" | "number" => value.is_number(),
                        "boolean" => value.is_boolean(),
                        "array" => value.is_array(),
                        "object" => value.is_object(),
                        _ => true,
                    };
                    if !is_valid {
                        return Err(format!(
                            "Invalid type for parameter '{key}' in tool '{tool_name}': expected {}, got {}",
                            prop.prop_type, value
                        ));
                    }

                    // تحقق من الـ enum
                    if let Some(enum_values) = &prop.enum_values {
                        if let Some(s) = value.as_str() {
                            if !enum_values.contains(&s.to_string()) {
                                return Err(format!(
                                    "Invalid value for '{key}': '{s}'. Must be one of: {:?}",
                                    enum_values
                                ));
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// تحويل السجل إلى JSON Schema قائمة (لـ OpenAI/Anthropic tool format)
    pub fn to_openai_format(&self) -> Vec<serde_json::Value> {
        self.tools.values().map(|tool| {
            serde_json::json!({
                "type": "function",
                "function": {
                    "name": tool.name,
                    "description": tool.description,
                    "parameters": {
                        "type": tool.parameters.schema_type,
                        "properties": tool.parameters.properties,
                        "required": tool.parameters.required,
                    }
                }
            })
        }).collect()
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_has_tools() {
        let registry = ToolRegistry::new();
        assert!(registry.count() >= 5);
        assert!(registry.get("code_editor").is_some());
        assert!(registry.get("web_search").is_some());
        assert!(registry.get("shell").is_some());
    }

    #[test]
    fn test_validate_code_editor_params() {
        let registry = ToolRegistry::new();

        // صحيح
        let valid = serde_json::json!({
            "operation": "read",
            "path": "src/main.rs"
        });
        assert!(registry.validate_params("code_editor", &valid).is_ok());

        // ناقص path
        let invalid = serde_json::json!({
            "operation": "read"
        });
        assert!(registry.validate_params("code_editor", &invalid).is_err());
    }

    #[test]
    fn test_validate_web_search_params() {
        let registry = ToolRegistry::new();

        // صحيح
        let valid = serde_json::json!({
            "query": "Rust programming"
        });
        assert!(registry.validate_params("web_search", &valid).is_ok());
    }

    #[test]
    fn test_validate_shell_critical() {
        let registry = ToolRegistry::new();
        let tool = registry.get("shell").unwrap();
        assert_eq!(tool.strictness, Strictness::Critical);
    }

    #[test]
    fn test_to_openai_format() {
        let registry = ToolRegistry::new();
        let formatted = registry.to_openai_format();
        assert!(formatted.len() >= 5);
        // كل أداة لها اسم
        for tool in &formatted {
            assert!(tool["function"]["name"].is_string());
        }
    }
}
