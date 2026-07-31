//! # CSV Format Handler

use crate::formats::FormatHandler;

pub struct CsvHandler;

impl FormatHandler for CsvHandler {
    fn name(&self) -> &'static str {
        "csv"
    }
    fn extensions(&self) -> Vec<&'static str> {
        vec!["csv", "tsv"]
    }

    fn validate(&self, content: &str) -> Result<String, String> {
        let mut reader = csv::ReaderBuilder::new()
            .flexible(true)
            .from_reader(content.as_bytes());
        let headers = reader
            .headers()
            .map_err(|e| format!("CSV رؤوس غير صالحة: {e}"))?
            .clone();
        let mut row_count = 0;
        for result in reader.records() {
            result.map_err(|e| format!("CSV صف {}: {e}", row_count + 1))?;
            row_count += 1;
        }
        Ok(format!(
            "✅ CSV صالح — {} أعمدة، {} صف",
            headers.len(),
            row_count
        ))
    }

    fn format(&self, content: &str) -> Result<String, String> {
        // إعادة تنسيق CSV (تطبيع الفواصل والمسافات)
        let mut reader = csv::ReaderBuilder::new()
            .flexible(true)
            .from_reader(content.as_bytes());
        let headers = reader.headers().map_err(|e| format!("CSV: {e}"))?.clone();
        let mut records = Vec::new();
        for r in reader.records() {
            records.push(r.map_err(|e| format!("CSV: {e}"))?);
        }
        let mut wtr = csv::Writer::from_writer(Vec::new());
        wtr.write_record(&headers)
            .map_err(|e| format!("CSV: {e}"))?;
        for r in &records {
            wtr.write_record(r).map_err(|e| format!("CSV: {e}"))?;
        }
        String::from_utf8(wtr.into_inner().map_err(|e| format!("CSV: {e}"))?)
            .map_err(|e| format!("CSV: {e}"))
    }

    fn convert_to_json(&self, content: &str) -> Result<String, String> {
        let mut reader = csv::ReaderBuilder::new()
            .flexible(true)
            .from_reader(content.as_bytes());
        let headers: Vec<String> = reader
            .headers()
            .map_err(|e| format!("CSV: {e}"))?
            .iter()
            .map(|s| s.to_string())
            .collect();
        let mut rows = Vec::new();
        for result in reader.records() {
            let record = result.map_err(|e| format!("CSV: {e}"))?;
            let mut map = serde_json::Map::new();
            for (i, field) in record.iter().enumerate() {
                let key = headers
                    .get(i)
                    .cloned()
                    .unwrap_or_else(|| format!("col_{i}"));
                map.insert(key, serde_json::Value::String(field.to_string()));
            }
            rows.push(serde_json::Value::Object(map));
        }
        serde_json::to_string_pretty(&serde_json::json!({
            "columns": headers,
            "rows": rows,
            "row_count": rows.len(),
            "column_count": headers.len(),
        }))
        .map_err(|e| format!("JSON: {e}"))
    }
}

/// إنشاء HTML table من CSV
pub fn csv_to_html_table(content: &str, title: &str) -> Result<String, String> {
    let mut reader = csv::ReaderBuilder::new()
        .flexible(true)
        .from_reader(content.as_bytes());
    let headers = reader.headers().map_err(|e| format!("CSV: {e}"))?;
    let mut html = format!(
        r#"<div class="csv-table"><h3>{}</h3><table><thead><tr>"#,
        title
    );
    for h in headers.iter() {
        html.push_str(&format!("<th>{}</th>", escape_html(h)));
    }
    html.push_str("</tr></thead><tbody>");
    for r in reader.records() {
        let record = r.map_err(|e| format!("CSV: {e}"))?;
        html.push_str("<tr>");
        for field in record.iter() {
            html.push_str(&format!("<td>{}</td>", escape_html(field)));
        }
        html.push_str("</tr>");
    }
    html.push_str("</tbody></table></div>");
    Ok(html)
}

fn escape_html(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            '&' => "&amp;".to_string(),
            '<' => "&lt;".to_string(),
            '>' => "&gt;".to_string(),
            '"' => "&quot;".to_string(),
            _ => c.to_string(),
        })
        .collect()
}
