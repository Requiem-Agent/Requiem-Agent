//! # Markdown Format Handler — عرض Markdown كـ HTML

use crate::formats::FormatHandler;

pub struct MarkdownHandler;

impl FormatHandler for MarkdownHandler {
    fn name(&self) -> &'static str { "markdown" }
    fn extensions(&self) -> Vec<&'static str> { vec!["md", "markdown", "mdown"] }

    fn validate(&self, content: &str) -> Result<String, String> {
        if content.trim().is_empty() {
            return Err("محتوى Markdown فارغ".into());
        }
        Ok(format!("✅ Markdown صالح — {} سطر، {} حرف",
            content.lines().count(), content.len()))
    }

    fn format(&self, content: &str) -> Result<String, String> {
        // تطبيع: إزالة المسافات الزائدة في نهاية السطور
        let formatted: String = content.lines()
            .map(|l| l.trim_end())
            .collect::<Vec<_>>()
            .join("\n");
        Ok(formatted)
    }

    fn convert_to_json(&self, content: &str) -> Result<String, String> {
        // Markdown → HTML
        let html = render_markdown_to_html(content);
        serde_json::to_string_pretty(&serde_json::json!({
            "html": html,
            "raw": content,
            "line_count": content.lines().count(),
        })).map_err(|e| format!("JSON: {e}"))
    }
}

/// تحويل Markdown مبسط إلى HTML (بدون مكتبات خارجية)
/// يدعم: العناوين، القوائم، الكود، الروابط، التأكيد
pub fn render_markdown_to_html(md: &str) -> String {
    let mut html = String::from("<div class=\"markdown-body\">\n");
    let mut in_code_block = false;
    let mut in_list = false;

    for line in md.lines() {
        // Code block
        if line.trim().starts_with("```") {
            if in_code_block {
                html.push_str("</code></pre>\n");
                in_code_block = false;
            } else {
                let lang = line.trim().trim_start_matches("```");
                html.push_str(&format!("<pre><code class=\"language-{}\">", lang));
                in_code_block = true;
            }
            continue;
        }
        if in_code_block {
            html.push_str(&escape_html(line));
            html.push('\n');
            continue;
        }

        let trimmed = line.trim();

        // Empty line
        if trimmed.is_empty() {
            if in_list { html.push_str("</ul>\n"); in_list = false; }
            continue;
        }

        // Headers
        if trimmed.starts_with("### ") {
            if in_list { html.push_str("</ul>\n"); in_list = false; }
            html.push_str(&format!("<h3>{}</h3>\n", escape_html(&trimmed[4..])));
        } else if trimmed.starts_with("## ") {
            if in_list { html.push_str("</ul>\n"); in_list = false; }
            html.push_str(&format!("<h2>{}</h2>\n", escape_html(&trimmed[3..])));
        } else if trimmed.starts_with("# ") {
            if in_list { html.push_str("</ul>\n"); in_list = false; }
            html.push_str(&format!("<h1>{}</h1>\n", escape_html(&trimmed[2..])));
        }
        // List items
        else if trimmed.starts_with("- ") || trimmed.starts_with("* ") {
            if !in_list { html.push_str("<ul>\n"); in_list = true; }
            html.push_str(&format!("<li>{}</li>\n", escape_html(&trimmed[2..])));
        }
        // Numbered list
        else if trimmed.starts_with(|c: char| c.is_ascii_digit()) && trimmed.contains(". ") {
            let content = trimmed.splitn(2, ". ").nth(1).unwrap_or(trimmed);
            if !in_list { html.push_str("<ol>\n"); in_list = true; }
            html.push_str(&format!("<li>{}</li>\n", escape_html(content)));
        }
        // Horizontal rule
        else if trimmed == "---" || trimmed == "***" || trimmed == "___" {
            if in_list { html.push_str("</ul>\n"); in_list = false; }
            html.push_str("<hr>\n");
        }
        // Paragraph (with inline formatting)
        else {
            if in_list { html.push_str("</ul>\n"); in_list = false; }
            let processed = process_inline(trimmed);
            html.push_str(&format!("<p>{}</p>\n", processed));
        }
    }

    if in_code_block { html.push_str("</code></pre>\n"); }
    if in_list { html.push_str("</ul>\n"); }
    html.push_str("</div>\n");
    html
}

fn process_inline(text: &str) -> String {
    let mut s = escape_html(text);
    // **bold**
    s = s.replace("**", "<strong>").replacen("<strong>", "**", 1); // rough
    // *italic*
    s = s.replace("*", "<em>").replacen("<em>", "*", 1);
    // `code`
    let mut result = String::new();
    let mut in_backtick = false;
    for chunk in s.split('`') {
        if in_backtick {
            result.push_str("<code>");
            result.push_str(chunk);
            result.push_str("</code>");
        } else {
            result.push_str(chunk);
        }
        in_backtick = !in_backtick;
    }
    // [text](url)
    let mut final_result = String::new();
    let mut rest = &result[..];
    while let Some(start) = rest.find("[") {
        final_result.push_str(&rest[..start]);
        rest = &rest[start..];
        if let Some(mid) = rest.find("](") {
            if let Some(end) = rest.find(")") {
                let text = &rest[1..mid];
                let url = &rest[mid+2..end];
                final_result.push_str(&format!("<a href=\"{}\">{}</a>", url, text));
                rest = &rest[end+1..];
                continue;
            }
        }
        final_result.push_str(rest);
        rest = "";
    }
    final_result.push_str(rest);
    final_result
}

fn escape_html(s: &str) -> String {
    s.chars().map(|c| match c {
        '&' => "&amp;".to_string(),
        '<' => "&lt;".to_string(),
        '>' => "&gt;".to_string(),
        '"' => "&quot;".to_string(),
        _ => c.to_string(),
    }).collect()
}
