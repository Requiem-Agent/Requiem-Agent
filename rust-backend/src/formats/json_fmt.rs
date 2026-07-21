//! # JSON Format Handler

use crate::formats::FormatHandler;

pub struct JsonHandler;

impl FormatHandler for JsonHandler {
    fn name(&self) -> &'static str { "json" }
    fn extensions(&self) -> Vec<&'static str> { vec!["json"] }

    fn validate(&self, content: &str) -> Result<String, String> {
        let _: serde_json::Value = serde_json::from_str(content)
            .map_err(|e| format!("JSON غير صالح: {e}"))?;
        Ok("✅ JSON صالح".into())
    }

    fn format(&self, content: &str) -> Result<String, String> {
        let val: serde_json::Value = serde_json::from_str(content)
            .map_err(|e| format!("JSON غير صالح: {e}"))?;
        serde_json::to_string_pretty(&val)
            .map_err(|e| format!("تنسيق JSON: {e}"))
    }

    fn convert_to_json(&self, content: &str) -> Result<String, String> {
        // JSON → JSON (نفس الشيء)
        self.format(content)
    }
}
