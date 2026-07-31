//! Parser tool using tree-sitter for AST analysis
//!
//! Provides code parsing and structural analysis capabilities.

use std::path::Path;
use std::sync::Arc;
use thiserror::Error;
use tree_sitter::{Language, Node, Parser, Tree};

/// Errors that can occur during parsing operations
#[derive(Error, Debug)]
pub enum ParserError {
    #[error("Failed to initialize parser: {0}")]
    ParserInitError(String),

    #[error("Failed to parse code: {0}")]
    ParseError(String),

    #[error("Unsupported language: {0}")]
    UnsupportedLanguage(String),

    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),
}

/// Supported programming languages
#[derive(Debug, Clone, PartialEq)]
pub enum LanguageType {
    Rust,
    TypeScript,
    Python,
    JavaScript,
    Unknown,
}

impl LanguageType {
    /// Get tree-sitter language for parsing
    fn tree_sitter_language(&self) -> Result<Language, ParserError> {
        match self {
            LanguageType::Rust => Ok(tree_sitter_rust::LANGUAGE.into()),
            LanguageType::TypeScript | LanguageType::JavaScript => {
                Ok(tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into())
            }
            LanguageType::Python => Ok(tree_sitter_python::LANGUAGE.into()),
            LanguageType::Unknown => Err(ParserError::UnsupportedLanguage("unknown".to_string())),
        }
    }
}

/// A node in the AST
#[derive(Debug, Clone)]
pub struct AstNode {
    pub kind: String,
    pub start_line: usize,
    pub end_line: usize,
    pub start_column: usize,
    pub end_column: usize,
    pub text: String,
    pub children: Vec<AstNode>,
}

/// Configuration for parser operations
#[derive(Debug, Clone)]
pub struct ParserConfig {
    pub language: LanguageType,
}

impl Default for ParserConfig {
    fn default() -> Self {
        Self {
            language: LanguageType::Rust,
        }
    }
}

/// High-performance parser tool using tree-sitter
#[derive(Clone)]
pub struct ParserTool {
    config: Arc<ParserConfig>,
}

impl ParserTool {
    /// Create a new ParserTool with the given configuration
    pub fn new(config: ParserConfig) -> Self {
        Self {
            config: Arc::new(config),
        }
    }

    /// Create a ParserTool with default configuration
    pub fn default() -> Self {
        Self::new(ParserConfig::default())
    }

    /// Parse code and return AST root node
    pub fn parse(&self, code: &str, language: &LanguageType) -> Result<AstNode, ParserError> {
        let lang = language.tree_sitter_language()?;
        let mut parser = Parser::new();

        parser
            .set_language(&lang)
            .map_err(|e| ParserError::ParserInitError(e.to_string()))?;

        let tree = parser
            .parse(code, None)
            .ok_or_else(|| ParserError::ParseError("Failed to parse code".to_string()))?;

        Ok(self.node_to_ast(&tree.root_node(), code))
    }

    /// Parse a file and return AST root node
    pub fn parse_file(&self, file_path: &str) -> Result<AstNode, ParserError> {
        let path = Path::new(file_path);
        let content = std::fs::read_to_string(path)?;
        let language = self.detect_language(file_path)?;
        self.parse(&content, &language)
    }

    /// Detect language from file extension
    pub fn detect_language(&self, file_path: &str) -> Result<LanguageType, ParserError> {
        let ext = Path::new(file_path)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");

        match ext {
            "rs" => Ok(LanguageType::Rust),
            "ts" | "tsx" | "js" | "jsx" => Ok(LanguageType::TypeScript),
            "py" => Ok(LanguageType::Python),
            _ => Ok(LanguageType::Unknown),
        }
    }

    /// Convert tree-sitter Node to our AstNode
    fn node_to_ast(&self, node: &Node, source: &str) -> AstNode {
        let start = node.start_position();
        let end = node.end_position();
        let text = node.utf8_text(source.as_bytes()).unwrap_or("").to_string();

        let mut children = Vec::new();
        let mut cursor = node.walk();

        for child in node.named_children(&mut cursor) {
            children.push(self.node_to_ast(&child, source));
        }

        AstNode {
            kind: node.kind().to_string(),
            start_line: start.row + 1,
            end_line: end.row + 1,
            start_column: start.column,
            end_column: end.column,
            text,
            children,
        }
    }

    /// Extract function definitions from AST
    pub fn extract_functions(&self, ast: &AstNode) -> Vec<AstNode> {
        let mut functions = Vec::new();
        self.find_nodes_by_kind(ast, "function_item", &mut functions);
        functions
    }

    /// Extract struct definitions from AST
    pub fn extract_structs(&self, ast: &AstNode) -> Vec<AstNode> {
        let mut structs = Vec::new();
        self.find_nodes_by_kind(ast, "struct_item", &mut structs);
        structs
    }

    /// Recursively find nodes by kind
    fn find_nodes_by_kind(&self, node: &AstNode, kind: &str, results: &mut Vec<AstNode>) {
        if node.kind == kind {
            results.push(node.clone());
        }
        for child in &node.children {
            self.find_nodes_by_kind(child, kind, results);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_rust_function() {
        let tool = ParserTool::default();
        let code = r#"
fn main() {
    println!("Hello, world!");
}
"#;
        let ast = tool.parse(code, &LanguageType::Rust).unwrap();
        assert_eq!(ast.kind, "source_file");
    }

    #[test]
    fn test_detect_language() {
        let tool = ParserTool::default();
        assert_eq!(tool.detect_language("main.rs").unwrap(), LanguageType::Rust);
        assert_eq!(
            tool.detect_language("app.ts").unwrap(),
            LanguageType::TypeScript
        );
        assert_eq!(
            tool.detect_language("script.py").unwrap(),
            LanguageType::Python
        );
    }
}
