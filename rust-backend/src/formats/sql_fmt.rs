//! # SQL Format Handler — التحقق + التنفيذ عبر libsql

use crate::formats::FormatHandler;

pub struct SqlHandler;

impl FormatHandler for SqlHandler {
    fn name(&self) -> &'static str { "sql" }
    fn extensions(&self) -> Vec<&'static str> { vec!["sql", "db"] }

    fn validate(&self, content: &str) -> Result<String, String> {
        if content.trim().is_empty() {
            return Err("استعلام SQL فارغ".into());
        }
        // تحقق أساسي: يجب أن يبدأ بـ SELECT/INSERT/UPDATE/DELETE/CREATE
        let upper = content.trim().to_uppercase();
        if upper.starts_with("SELECT") || upper.starts_with("INSERT")
            || upper.starts_with("UPDATE") || upper.starts_with("DELETE")
            || upper.starts_with("CREATE") || upper.starts_with("DROP")
            || upper.starts_with("ALTER") || upper.starts_with("PRAGMA")
            || upper.starts_with("EXPLAIN")
        {
            Ok(format!("✅ SQL صالح شكلياً — {} حرف", content.len()))
        } else {
            Err("⚠️ SQL غير معروف — يجب أن يبدأ بـ SELECT/INSERT/UPDATE/DELETE/CREATE".into())
        }
    }

    fn format(&self, content: &str) -> Result<String, String> {
        // تنسيق بسيط: تطبيع المسافات البيضاء
        let normalized = content
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        // إضافة سطر جديد بعد كل ; (لكل جملة)
        let formatted = normalized.replace(";", ";\n");
        Ok(formatted.trim().to_string())
    }

    fn convert_to_json(&self, _content: &str) -> Result<String, String> {
        Err("SQL لا يُحول إلى JSON مباشرة. استخدم /api/formats/sql/exec".into())
    }
}

/// تنفيذ استعلام SQL عبر libsql وإرجاع النتائج كـ JSON
pub async fn execute_sql(
    db_url: &str,
    db_token: Option<&str>,
    query: &str,
) -> Result<serde_json::Value, String> {
    let db = if let Some(token) = db_token {
        libsql::Builder::new_remote(db_url.to_string(), token.to_string())
            .build().await
            .map_err(|e| format!("اتصال بقاعدة البيانات: {e}"))?
    } else {
        libsql::Builder::new_local(db_url.to_string())
            .build().await
            .map_err(|e| format!("فتح قاعدة البيانات: {e}"))?
    };

    let conn = db.connect().map_err(|e| format!("اتصال: {e}"))?;

    let upper = query.trim().to_uppercase();
    let is_query = upper.starts_with("SELECT") || upper.starts_with("PRAGMA") || upper.starts_with("EXPLAIN");

    if is_query {
        let mut rows = conn.query(query, libsql::params![]).await
            .map_err(|e| format!("تنفيذ الاستعلام: {e}"))?;

        let mut columns: Vec<String> = Vec::new();
        let mut results = Vec::new();
        let mut first = true;
        while let Some(row) = rows.next().await
            .map_err(|e| format!("قراءة النتائج: {e}"))? {
            let mut map = serde_json::Map::new();
            let count = row.column_count();
            for i in 0..count {
                let col_name = row.column_name(i).unwrap_or(&format!("col_{i}")).to_string();
                if first {
                    columns.push(col_name.clone());
                }
                let val: serde_json::Value = row.get_value(i)
                    .map(|v| match v {
                        libsql::Value::Integer(n) => serde_json::json!(n),
                        libsql::Value::Real(f) => serde_json::json!(f),
                        libsql::Value::Text(s) => serde_json::Value::String(s),
                        libsql::Value::Blob(b) => serde_json::Value::String(format!("<blob {} bytes>", b.len())),
                        libsql::Value::Null => serde_json::Value::Null,
                    })
                    .unwrap_or(serde_json::Value::Null);
                map.insert(col_name, val);
            }
            first = false;
            results.push(serde_json::Value::Object(map));
        }

        Ok(serde_json::json!({
            "columns": columns,
            "rows": results,
            "row_count": results.len(),
            "column_count": columns.len(),
        }))
    } else {
        // INSERT/UPDATE/DELETE
        let affected = conn.execute(query, libsql::params![]).await
            .map_err(|e| format!("تنفيذ: {e}"))?;
        Ok(serde_json::json!({
            "affected_rows": affected,
            "query": "executed",
        }))
    }
}
