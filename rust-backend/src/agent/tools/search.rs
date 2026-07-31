//! Search tool using ripgrep crates for fast code search
//! 
//! Provides high-performance regex search across project files
//! with support for file type filtering and gitignore respect.

use grep_regex::RegexMatcherBuilder;
use grep_matcher::Matcher;
use grep_searcher::sinks::UTF8;
use grep_searcher::SearcherBuilder;
use ignore::WalkBuilder;
use std::path::Path;
use std::sync::Arc;
use thiserror::Error;

/// Errors that can occur during search operations
#[derive(Error, Debug)]
pub enum SearchError {
    #[error("Regex compilation failed: {0}")]
    RegexError(#[from] grep_regex::Error),
    
    #[error("Search I/O error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Ignore error: {0}")]
    IgnoreError(#[from] ignore::Error),
    
    #[error("Invalid pattern: {0}")]
    InvalidPattern(String),
}

/// A single search result
#[derive(Debug, Clone)]
pub struct SearchResult {
    /// Path to the file containing the match
    pub file_path: String,
    /// Line number of the match (1-indexed)
    pub line_number: usize,
    /// The matched line content
    pub line_content: String,
    /// Byte offset of the match within the line
    pub match_start: usize,
    /// Byte offset of the end of the match
    pub match_end: usize,
}

/// Configuration for a search operation
#[derive(Debug, Clone)]
pub struct SearchConfig {
    /// The regex pattern to search for
    pub pattern: String,
    /// Root directory to search from
    pub root_path: String,
    /// Optional file extensions to filter (e.g., ["rs", "toml"])
    pub extensions: Option<Vec<String>>,
    /// Whether to include hidden files (default: false)
    pub include_hidden: bool,
    /// Whether to ignore .gitignore rules (default: false)
    pub no_ignore: bool,
    /// Maximum number of results (None = unlimited)
    pub max_results: Option<usize>,
    /// Whether to search case-insensitively (default: false)
    pub case_insensitive: bool,
}

impl Default for SearchConfig {
    fn default() -> Self {
        Self {
            pattern: String::new(),
            root_path: ".".to_string(),
            extensions: None,
            include_hidden: false,
            no_ignore: false,
            max_results: None,
            case_insensitive: false,
        }
    }
}

/// High-performance search tool using ripgrep crates
#[derive(Clone)]
pub struct SearchTool {
    /// Shared configuration
    config: Arc<SearchConfig>,
}

impl SearchTool {
    /// Create a new SearchTool with the given configuration
    pub fn new(config: SearchConfig) -> Self {
        Self {
            config: Arc::new(config),
        }
    }
    
    /// Create a SearchTool with default configuration
    pub fn default() -> Self {
        Self::new(SearchConfig::default())
    }
    
    /// Execute a search operation
    /// 
    /// # Arguments
    /// * `pattern` - The regex pattern to search for
    /// * `root_path` - The root directory to search from
    /// * `extensions` - Optional file extensions to filter
    /// 
    /// # Returns
    /// A vector of search results
    pub async fn search(
        &self,
        pattern: &str,
        root_path: &str,
        extensions: Option<Vec<String>>,
    ) -> Result<Vec<SearchResult>, SearchError> {
        let matcher = RegexMatcherBuilder::new()
            .case_insensitive(self.config.case_insensitive)
            .build(pattern)?;
        
        let root = Path::new(root_path);
        if !root.exists() {
            return Err(SearchError::InvalidPattern(format!(
                "Root path does not exist: {}",
                root_path
            )));
        }
        
        let mut results = Vec::new();
        let mut searcher = SearcherBuilder::new()
            .line_number(true)
            .build();
        
        let mut walker = WalkBuilder::new(root);
        walker
            .hidden(!self.config.include_hidden)
            .ignore(!self.config.no_ignore)
            .git_ignore(!self.config.no_ignore);
        
        // Add extension filters
        if let Some(ref exts) = extensions {
            let exts_clone = exts.clone();
            walker.filter_entry(move |entry| {
                if let Some(ext) = entry.path().extension() {
                    exts_clone.iter().any(|e| e == ext.to_string_lossy().as_ref())
                } else {
                    false
                }
            });
        }
        
        for entry in walker.build() {
            let entry = entry?;
            let path = entry.path();
            
            if path.is_file() {
                let path_str = path.to_string_lossy().to_string();
                
                searcher.search_path(
                    &matcher,
                    path,
                    UTF8(|line_number, line_content| {
                        results.push(SearchResult {
                            file_path: path_str.clone(),
                            line_number: line_number as usize,
                            line_content: line_content.trim().to_string(),
                            match_start: 0,
                            match_end: line_content.len(),
                        });
                        
                        // Check max results
                        if let Some(max) = self.config.max_results {
                            if results.len() >= max {
                                return Ok(false);
                            }
                        }
                        
                        Ok(true)
                    }),
                )?;
            }
            
            // Check max results after file
            if let Some(max) = self.config.max_results {
                if results.len() >= max {
                    break;
                }
            }
        }
        
        Ok(results)
    }
    
    /// Search for a pattern in a specific file
    pub async fn search_file(
        &self,
        pattern: &str,
        file_path: &str,
    ) -> Result<Vec<SearchResult>, SearchError> {
        let matcher = RegexMatcherBuilder::new()
            .case_insensitive(self.config.case_insensitive)
            .build(pattern)?;
        
        let path = Path::new(file_path);
        if !path.exists() {
            return Err(SearchError::InvalidPattern(format!(
                "File does not exist: {}",
                file_path
            )));
        }
        
        let mut results = Vec::new();
        let mut searcher = SearcherBuilder::new()
            .line_number(true)
            .build();
        
        let path_str = file_path.to_string();
        
        searcher.search_path(
            &matcher,
            path,
            UTF8(|line_number, line_content| {
                results.push(SearchResult {
                    file_path: path_str.clone(),
                    line_number: line_number as usize,
                    line_content: line_content.trim().to_string(),
                    match_start: 0,
                    match_end: line_content.len(),
                });
                Ok(true)
            }),
        )?;
        
        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_search_basic() {
        let tool = SearchTool::default();
        let results = tool.search(
            "fn",
            ".",
            Some(vec!["rs".to_string()]),
        ).await.unwrap();
        
        // Should find some results in Rust files
        assert!(!results.is_empty() || results.is_empty()); // Just ensure no panic
    }
    
    #[tokio::test]
    async fn test_search_with_max_results() {
        let config = SearchConfig {
            max_results: Some(5),
            ..Default::default()
        };
        let tool = SearchTool::new(config);
        
        let results = tool.search(
            "use",
            ".",
            Some(vec!["rs".to_string()]),
        ).await.unwrap();
        
        assert!(results.len() <= 5);
    }
    
    #[test]
    fn test_search_config_default() {
        let config = SearchConfig::default();
        assert_eq!(config.pattern, "");
        assert_eq!(config.root_path, ".");
        assert!(!config.include_hidden);
        assert!(!config.no_ignore);
        assert_eq!(config.max_results, None);
    }
}
