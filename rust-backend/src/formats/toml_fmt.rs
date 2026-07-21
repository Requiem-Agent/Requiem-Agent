//! # TOML Format Handler

use crate::formats::FormatHandler;

pub struct TomlHandler;

impl FormatHandler for TomlHandler {
    fn name(&self) -> &'static str { "toml" }
    fn extensions(&self) -> Vec<&'static str> { vec!["toml"] }

    fn validate(&self, content: &str) -> Result<String, String> {
        let _: toml::Value = toml::from_str(content)
            .map_err(|e| format!("TOML غير صالح: {e}"))?;
        Ok("✅ TOML صالح".into())
    }

    fn format(&self, content: &str) -> Result<String, String> {
        let val: toml::Value = toml::from_str(content)
            .map_err(|e| format!("TOML غير صالح: {e}"))?;
        toml::to_string_pretty(&val)
            .map_err(|e| format!("تنسيق TOML: {e}"))
    }

    fn convert_to_json(&self, content: &str) -> Result<String, String> {
        let val: toml::Value = toml::from_str(content)
            .map_err(|e| format!("TOML غير صالح: {e}"))?;
        let json_val = toml_to_json(&val);
        serde_json::to_string_pretty(&json_val)
            .map_err(|e| format!("تحويل TOML→JSON: {e}"))
    }
}

fn toml_to_json(v: &toml::Value) -> serde_json::Value {
    match v {
        toml::Value::String(s) => serde_json::Value::String(s.clone()),
        toml::Value::Integer(i) => serde_json::Value::Number((*i).into()),
        toml::Value::Float(f) => serde_json::json!(f),
        toml::Value::Boolean(b) => serde_json::Value::Bool(*b),
        toml::Value::Array(arr) => serde_json::Value::Array(arr.iter().map(toml_to_json).collect()),
        toml::Value::Table(tbl) => {
            let mut map = serde_json::Map::new();
            for (k, v) in tbl { map.insert(k.clone(), toml_to_json(v)); }
            serde_json::Value::Object(map)
        }
        toml::Value::Datetime(dt) => serde_json::Value::String(dt.to_string()),
    }
}
