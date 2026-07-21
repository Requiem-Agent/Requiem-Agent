//! # Formats Module — دعم كل التنسيقات والرسوم البيانية
//!
//! ## التنسيقات المدعومة
//! - JSON: تحقق، تنسيق، ضغط
//! - YAML: تحليل ← JSON، تنسيق
//! - TOML: تحليل ← JSON، تنسيق
//! - CSV: تحليل ← JSON/HTML table، تصدير
//! - SQL: تحقق، تنفيذ عبر libsql
//! - Markdown: عرض ← HTML
//! - SVG Charts: Bar, Line, Pie charts

pub mod json_fmt;
pub mod yaml_fmt;
pub mod toml_fmt;
pub mod csv_fmt;
pub mod sql_fmt;
pub mod markdown_fmt;
// pub mod svg_charts;  // TODO: Fix raw string issues with # characters

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ─── FormatHandler trait ───────────────────────────────────────────────────

pub trait FormatHandler: Send + Sync {
    fn name(&self) -> &'static str;
    fn extensions(&self) -> Vec<&'static str>;
    fn validate(&self, content: &str) -> Result<String, String>;
    fn format(&self, content: &str) -> Result<String, String>;
    fn convert_to_json(&self, content: &str) -> Result<String, String>;
}

// ─── FormatRegistry ────────────────────────────────────────────────────────

pub struct FormatRegistry {
    handlers: HashMap<&'static str, Box<dyn FormatHandler>>,
}

impl FormatRegistry {
    pub fn new() -> Self {
        let mut handlers: HashMap<&'static str, Box<dyn FormatHandler>> = HashMap::new();
        handlers.insert("json", Box::new(json_fmt::JsonHandler));
        handlers.insert("yaml", Box::new(yaml_fmt::YamlHandler));
        handlers.insert("toml", Box::new(toml_fmt::TomlHandler));
        handlers.insert("csv", Box::new(csv_fmt::CsvHandler));
        handlers.insert("sql", Box::new(sql_fmt::SqlHandler));
        handlers.insert("markdown", Box::new(markdown_fmt::MarkdownHandler));
        Self { handlers }
    }

    pub fn get(&self, name: &str) -> Option<&Box<dyn FormatHandler>> {
        self.handlers.get(name)
    }

    pub fn detect(&self, filename: &str) -> Option<&Box<dyn FormatHandler>> {
        let ext = filename.rsplit('.').next().unwrap_or("").to_lowercase();
        self.handlers.values().find(|h| h.extensions().contains(&ext.as_str()))
    }

    pub fn list(&self) -> Vec<FormatInfo> {
        self.handlers.iter().map(|(name, h)| FormatInfo {
            name: name.to_string(),
            description: format!("{} handler", name),
            extensions: h.extensions().iter().map(|e| e.to_string()).collect(),
            can_validate: true,
            can_format: true,
            can_convert: name != &"sql",
        }).collect()
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FormatInfo {
    pub name: String,
    pub description: String,
    pub extensions: Vec<String>,
    pub can_validate: bool,
    pub can_format: bool,
    pub can_convert: bool,
}
