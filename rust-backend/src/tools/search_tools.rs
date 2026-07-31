//! # Search Tools — أدوات البحث في الكود باستخدام ripgrep
//!
//! يوفر بحثاً سريعاً وفعلاً في ملفات المشروع.

use serde::{Deserialize, Serialize};
use std::path::Path;

/// نتيجة البحث
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub file: String,
    pub line: usize,
    pub content: String,
    pub column: Option<usize>,
    pub match_type: String,
}

/// إحصائيات البحث
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchStats {
    pub total_matches: usize,
    pub files_searched: usize,
    pub duration_ms: u64,
}

/// بحث في ملفات المشروع
pub async fn search_in_code(
    pattern: &str,
    root_dir: &Path,
    file_patterns: Option<Vec<String>>,
    max_results: Option<usize>,
) -> Result<(Vec<SearchResult>, SearchStats), String> {
    let start = std::time::Instant::now();
    let mut results = Vec::new();
    let mut files_searched = 0;
    let max = max_results.unwrap_or(100);

    // استخدام walkdir للبحث في الملفات
    for entry in walkdir::WalkDir::new(root_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
    {
        let path = entry.path();
        
        // تصفية حسب أنماط الملفات
        if let Some(ref patterns) = file_patterns {
            let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if !patterns.iter().any(|p| filename.contains(p)) {
                continue;
            }
        }

        files_searched += 1;

        // قراءة الملف والبحث عن النمط
        if let Ok(content) = std::fs::read_to_string(path) {
            for (line_num, line) in content.lines().enumerate() {
                if line.contains(pattern) {
                    results.push(SearchResult {
                        file: path.to_string_lossy().to_string(),
                        line: line_num + 1,
                        content: line.to_string(),
                        column: line.find(pattern),
                        match_type: "text".to_string(),
                    });

                    if results.len() >= max {
                        break;
                    }
                }
            }
        }

        if results.len() >= max {
            break;
        }
    }

    let duration = start.elapsed().as_millis() as u64;
    
    let total_matches = results.len();
    Ok((results, SearchStats {
        total_matches,
        files_searched,
        duration_ms: duration,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_result() {
        let result = SearchResult {
            file: "main.rs".into(),
            line: 42,
            content: "fn main() {}".into(),
            column: Some(0),
            match_type: "text".into(),
        };
        assert_eq!(result.line, 42);
    }

    #[test]
    fn test_search_stats() {
        let stats = SearchStats {
            total_matches: 10,
            files_searched: 5,
            duration_ms: 150,
        };
        assert_eq!(stats.total_matches, 10);
    }
}