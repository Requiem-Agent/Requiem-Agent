//! # Diff Tools — أدوات مقارنة الملفات والحجج
//!
//! يوفر مقارنة الملفات واكتشاف التغييرات.

use serde::{Deserialize, Serialize};

/// نوع التغيير في السطر
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DiffLineType {
    Added,
    Removed,
    Modified,
    Context,
}

/// سطر في diffs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffLine {
    pub line_type: DiffLineType,
    pub content: String,
    pub old_line_num: Option<usize>,
    pub new_line_num: Option<usize>,
}

/// نتيجة المقارنة
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffResult {
    pub file: String,
    pub changes: Vec<DiffLine>,
    pub stats: DiffStats,
}

/// إحصائيات التغييرات
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffStats {
    pub lines_added: usize,
    pub lines_removed: usize,
    pub lines_modified: usize,
}

/// مقارنة نصين
pub async fn compare_texts(
    old_text: &str,
    new_text: &str,
) -> Result<DiffResult, String> {
    let old_lines: Vec<&str> = old_text.lines().collect();
    let new_lines: Vec<&str> = new_text.lines().collect();
    let mut changes = Vec::new();
    let mut stats = DiffStats {
        lines_added: 0,
        lines_removed: 0,
        lines_modified: 0,
    };

    let max_lines = old_lines.len().max(new_lines.len());

    for i in 0..max_lines {
        let old_line = old_lines.get(i);
        let new_line = new_lines.get(i);

        match (old_line, new_line) {
            (Some(old), Some(new)) => {
                if old == new {
                    changes.push(DiffLine {
                        line_type: DiffLineType::Context,
                        content: old.to_string(),
                        old_line_num: Some(i + 1),
                        new_line_num: Some(i + 1),
                    });
                } else {
                    changes.push(DiffLine {
                        line_type: DiffLineType::Modified,
                        content: format!("{} → {}", old, new),
                        old_line_num: Some(i + 1),
                        new_line_num: Some(i + 1),
                    });
                    stats.lines_modified += 1;
                }
            }
            (Some(old), None) => {
                changes.push(DiffLine {
                    line_type: DiffLineType::Removed,
                    content: old.to_string(),
                    old_line_num: Some(i + 1),
                    new_line_num: None,
                });
                stats.lines_removed += 1;
            }
            (None, Some(new)) => {
                changes.push(DiffLine {
                    line_type: DiffLineType::Added,
                    content: new.to_string(),
                    old_line_num: None,
                    new_line_num: Some(i + 1),
                });
                stats.lines_added += 1;
            }
            (None, None) => {}
        }
    }

    Ok(DiffResult {
        file: "comparison".to_string(),
        changes,
        stats,
    })
}

/// مقارنة ملفين
pub async fn compare_files(
    old_file: &str,
    new_file: &str,
    file_path: &str,
) -> Result<DiffResult, String> {
    let old_content = std::fs::read_to_string(old_file)
        .map_err(|e| format!("Failed to read old file: {}", e))?;
    let new_content = std::fs::read_to_string(new_file)
        .map_err(|e| format!("Failed to read new file: {}", e))?;

    let mut result = compare_texts(&old_content, &new_content).await?;
    result.file = file_path.to_string();
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_diff_line_type() {
        let line = DiffLine {
            line_type: DiffLineType::Added,
            content: "new line".into(),
            old_line_num: None,
            new_line_num: Some(1),
        };
        assert!(matches!(line.line_type, DiffLineType::Added));
    }

    #[tokio::test]
    async fn test_compare_texts() {
        let old = "line1\nline2\nline3";
        let new = "line1\nline2 modified\nline3\nline4";
        let result = compare_texts(old, new).await.unwrap();
        assert_eq!(result.stats.lines_added, 1);
        assert_eq!(result.stats.lines_modified, 1);
    }
}