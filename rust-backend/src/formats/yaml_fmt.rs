//! # YAML Format Handler

use crate::formats::FormatHandler;

pub struct YamlHandler;

impl FormatHandler for YamlHandler {
    fn name(&self) -> &'static str { "yaml" }
    fn extensions(&self) -> Vec<&'static str> { vec!["yaml", "yml"] }

    fn validate(&self, content: &str) -> Result<String, String> {
        let _: serde_yaml::Value = serde_yaml::from_str(content)
            .map_err(|e| format!("YAML غير صالح: {e}"))?;
        Ok("✅ YAML صالح".into())
    }

    fn format(&self, content: &str) -> Result<String, String> {
        let val: serde_yaml::Value = serde_yaml::from_str(content)
            .map_err(|e| format!("YAML غير صالح: {e}"))?;
        serde_yaml::to_string(&val)
            .map_err(|e| format!("تنسيق YAML: {e}"))
    }

    fn convert_to_json(&self, content: &str) -> Result<String, String> {
        let val: serde_yaml::Value = serde_yaml::from_str(content)
            .map_err(|e| format!("YAML غير صالح: {e}"))?;
        serde_json::to_string_pretty(&val)
            .map_err(|e| format!("تحويل YAML→JSON: {e}"))
    }
}
