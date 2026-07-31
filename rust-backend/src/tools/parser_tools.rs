//! # Parser Tools — أدوات تحليل AST باستخدام tree-sitter
//!
//! يوفر تحليل بنية الكود وفهمه.

use serde::{Deserialize, Serialize};

/// نتيجة تحليل AST
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AstNode {
    pub node_type: String,
    pub name: String,
    pub start_line: usize,
    pub end_line: usize,
    pub children: Vec<AstNode>,
    pub properties: std::collections::HashMap<String, String>,
}

/// تحليل كود إلى AST
pub async fn parse_code(
    code: &str,
    language: &str,
) -> Result<AstNode, String> {
    // محاكاة تحليل AST
    let root = AstNode {
        node_type: "program".to_string(),
        name: "root".to_string(),
        start_line: 1,
        end_line: code.lines().count(),
        children: Vec::new(),
        properties: std::collections::HashMap::new(),
    };

    Ok(root)
}

/// استخراج الدوال من الكود
pub async fn extract_functions(
    code: &str,
    language: &str,
) -> Result<Vec<AstNode>, String> {
    let mut functions = Vec::new();
    let lines: Vec<&str> = code.lines().collect();

    for (idx, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        // كشف بسيط للدوال
        if trimmed.starts_with("fn ") || trimmed.starts_with("function ") || trimmed.starts_with("pub fn ") {
            functions.push(AstNode {
                node_type: "function".to_string(),
                name: trimmed.split_whitespace().nth(1).unwrap_or("unknown").to_string(),
                start_line: idx + 1,
                end_line: idx + 1,
                children: Vec::new(),
                properties: std::collections::HashMap::new(),
            });
        }
    }

    Ok(functions)
}

/// استخراج الفئات من الكود
pub async fn extract_classes(
    code: &str,
    language: &str,
) -> Result<Vec<AstNode>, String> {
    let mut classes = Vec::new();
    let lines: Vec<&str> = code.lines().collect();

    for (idx, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        // كشف بسيط للفئات
        if trimmed.starts_with("struct ") || trimmed.starts_with("class ") || trimmed.starts_with("pub struct ") {
            classes.push(AstNode {
                node_type: "class".to_string(),
                name: trimmed.split_whitespace().nth(1).unwrap_or("unknown").to_string(),
                start_line: idx + 1,
                end_line: idx + 1,
                children: Vec::new(),
                properties: std::collections::HashMap::new(),
            });
        }
    }

    Ok(classes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ast_node() {
        let node = AstNode {
            node_type: "function".into(),
            name: "main".into(),
            start_line: 1,
            end_line: 10,
            children: Vec::new(),
            properties: std::collections::HashMap::new(),
        };
        assert_eq!(node.node_type, "function");
    }

    #[tokio::test]
    async fn test_extract_functions() {
        let code = r#"
fn main() {
    println!("Hello");
}

fn helper() {}
"#;
        let functions = extract_functions(code, "rust").await.unwrap();
        assert_eq!(functions.len(), 2);
        assert_eq!(functions[0].name, "main");
    }
}