//! Diff tool using similar crate for code comparison
//!
//! Provides high-quality diff generation and comparison capabilities.

use similar::{Change, ChangeTag, TextDiff};
use std::path::Path;
use std::sync::Arc;
use thiserror::Error;

/// Errors that can occur during diff operations
#[derive(Error, Debug)]
pub enum DiffError {
    #[error("Failed to read file: {0}")]
    FileReadError(String),

    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),
}

/// A single change in the diff
#[derive(Debug, Clone)]
pub struct DiffChange {
    pub tag: ChangeTag,
    pub content: String,
    pub old_line: Option<usize>,
    pub new_line: Option<usize>,
}

/// Result of a diff operation
#[derive(Debug, Clone)]
pub struct DiffResult {
    pub changes: Vec<DiffChange>,
    pub stats: DiffStats,
}

/// Statistics about the diff
#[derive(Debug, Clone)]
pub struct DiffStats {
    pub lines_added: usize,
    pub lines_removed: usize,
    pub lines_modified: usize,
}

/// Configuration for diff operations
#[derive(Debug, Clone)]
pub struct DiffConfig {
    pub context_lines: usize,
}

impl Default for DiffConfig {
    fn default() -> Self {
        Self { context_lines: 3 }
    }
}

/// High-quality diff tool using similar crate
#[derive(Clone)]
pub struct DiffTool {
    config: Arc<DiffConfig>,
}

impl DiffTool {
    /// Create a new DiffTool with the given configuration
    pub fn new(config: DiffConfig) -> Self {
        Self {
            config: Arc::new(config),
        }
    }

    /// Create a DiffTool with default configuration
    pub fn default() -> Self {
        Self::new(DiffConfig::default())
    }

    /// Compare two texts and return diff result
    pub fn compare_texts(&self, old_text: &str, new_text: &str) -> DiffResult {
        let diff = TextDiff::from_lines(old_text, new_text);
        let mut changes = Vec::new();
        let mut stats = DiffStats {
            lines_added: 0,
            lines_removed: 0,
            lines_modified: 0,
        };

        let mut old_line = 1;
        let mut new_line = 1;

        for op in diff.ops() {
            for change in diff.iter_changes(op) {
                let (tag, content) = match change.tag() {
                    ChangeTag::Delete => {
                        stats.lines_removed += 1;
                        (ChangeTag::Delete, change.value().to_string())
                    }
                    ChangeTag::Insert => {
                        stats.lines_added += 1;
                        (ChangeTag::Insert, change.value().to_string())
                    }
                    ChangeTag::Equal => (ChangeTag::Equal, change.value().to_string()),
                };

                let old = if tag != ChangeTag::Insert {
                    let line = old_line;
                    old_line += 1;
                    Some(line)
                } else {
                    None
                };

                let new = if tag != ChangeTag::Delete {
                    let line = new_line;
                    new_line += 1;
                    Some(line)
                } else {
                    None
                };

                changes.push(DiffChange {
                    tag,
                    content,
                    old_line: old,
                    new_line: new,
                });
            }
        }

        // Calculate modifications (pairs of delete+insert)
        stats.lines_modified = stats.lines_added.min(stats.lines_removed);

        DiffResult { changes, stats }
    }

    /// Compare two files and return diff result
    pub fn compare_files(&self, old_file: &str, new_file: &str) -> Result<DiffResult, DiffError> {
        let old_content = std::fs::read_to_string(old_file)
            .map_err(|e| DiffError::FileReadError(format!("{}: {}", old_file, e)))?;
        let new_content = std::fs::read_to_string(new_file)
            .map_err(|e| DiffError::FileReadError(format!("{}: {}", new_file, e)))?;

        Ok(self.compare_texts(&old_content, &new_content))
    }

    /// Generate unified diff format output
    pub fn unified_diff(
        &self,
        old_text: &str,
        new_text: &str,
        old_name: &str,
        new_name: &str,
    ) -> String {
        let diff = TextDiff::from_lines(old_text, new_text);
        let mut output = String::new();

        output.push_str(&format!("--- {}\n", old_name));
        output.push_str(&format!("+++ {}\n", new_name));

        for op in diff.ops() {
            output.push_str(&format!(
                "@@ -{},{} +{},{} @@\n",
                op.old_range().start + 1,
                op.old_range().len(),
                op.new_range().start + 1,
                op.new_range().len(),
            ));

            for change in diff.iter_changes(op) {
                let prefix = match change.tag() {
                    ChangeTag::Delete => "-",
                    ChangeTag::Insert => "+",
                    ChangeTag::Equal => " ",
                };
                output.push_str(prefix);
                output.push_str(change.value());
                if !change.value().ends_with('\n') {
                    output.push('\n');
                }
            }
        }

        output
    }

    /// Generate context diff format output
    pub fn context_diff(
        &self,
        old_text: &str,
        new_text: &str,
        old_name: &str,
        new_name: &str,
    ) -> String {
        let diff = TextDiff::from_lines(old_text, new_text);
        let mut output = String::new();

        output.push_str(&format!("*** {}\n", old_name));
        output.push_str(&format!("--- {}\n", new_name));

        for op in diff.ops() {
            output.push_str(&format!("***************\n"));
            output.push_str(&format!(
                "*** {},{} ****\n",
                op.old_range().start + 1,
                op.old_range().end,
            ));

            for change in diff.iter_changes(op) {
                match change.tag() {
                    ChangeTag::Delete => {
                        output.push_str(&format!("-{}", change.value()));
                    }
                    ChangeTag::Equal => {
                        output.push_str(&format!(" {}", change.value()));
                    }
                    ChangeTag::Insert => {}
                }
            }

            if op.new_range().len() > 0 {
                output.push_str(&format!(
                    "--- {},{} ----\n",
                    op.new_range().start + 1,
                    op.new_range().end,
                ));

                for change in diff.iter_changes(op) {
                    match change.tag() {
                        ChangeTag::Insert => {
                            output.push_str(&format!("+{}", change.value()));
                        }
                        ChangeTag::Equal => {
                            output.push_str(&format!(" {}", change.value()));
                        }
                        ChangeTag::Delete => {}
                    }
                }
            }
        }

        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compare_texts() {
        let tool = DiffTool::default();
        let old = "line1\nline2\nline3\n";
        let new = "line1\nline2 modified\nline3\nline4\n";

        let result = tool.compare_texts(old, new);
        assert!(result.stats.lines_added > 0 || result.stats.lines_removed > 0);
    }

    #[test]
    fn test_unified_diff() {
        let tool = DiffTool::default();
        let old = "line1\nline2\n";
        let new = "line1\nline2 modified\n";

        let diff = tool.unified_diff(old, new, "old.txt", "new.txt");
        assert!(diff.contains("--- old.txt"));
        assert!(diff.contains("+++ new.txt"));
    }
}
