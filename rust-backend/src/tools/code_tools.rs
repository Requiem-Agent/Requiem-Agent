//! # Code Tools — أدوات تعديل الكود المتقدمة (مستوحاة من Replit Agent 3)
//!
//! ## الميزات
//! - قراءة/كتابة/تعديل ملفات متعددة بالتوازي
//! - تحليل هيكل المشروع وفهم العلاقات بين الملفات
//! - كشف التكرار في الكود
//! - دعم كامل للمسارات الآمنة (path_safety)

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::orchestrator::{FileEditOperation, FileEditResult, MultiFileEditor};
use crate::storage;

// ─── Parallel Code Edit ──────────────────────────────────────────────────────

/// مجموعة عمليات تعديل على ملفات متعددة
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParallelEditRequest {
    pub operations: Vec<FileEditOperation>,
    pub session_id: String,
}

/// نتيجة التعديلات المتوازية
#[derive(Debug, Clone, Serialize)]
pub struct ParallelEditResponse {
    pub results: Vec<FileEditResult>,
    pub success_count: usize,
    pub failure_count: usize,
    pub total_duration_ms: u64,
}

/// تنفيذ مجموعة من التعديلات على ملفات متعددة بالتوازي
pub async fn execute_parallel_edit(
    operations: Vec<FileEditOperation>,
    user_id: &str,
    session_id: &str,
) -> ParallelEditResponse {
    let start = std::time::Instant::now();

    let results = MultiFileEditor::execute_parallel(operations, user_id, session_id).await;

    let success_count = results.iter().filter(|r| r.success).count();
    let failure_count = results.iter().filter(|r| !r.success).count();

    ParallelEditResponse {
        results,
        success_count,
        failure_count,
        total_duration_ms: start.elapsed().as_millis() as u64,
    }
}

// ─── Replit-Style Multi-File Generation ──────────────────────────────────────

/// إنشاء هيكل مشروع كامل من وصف (Replit Agent 3 style)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectTemplate {
    pub name: String,
    pub description: String,
    pub files: Vec<(String, String)>, // (path, content)
}

/// إنشاء مشروع كامل من القالب
pub async fn generate_project(
    template: ProjectTemplate,
    user_id: &str,
    session_id: &str,
) -> Result<ParallelEditResponse, String> {
    // كتابة جميع الملفات بالتوازي
    let file_count = template.files.len();
    let results = MultiFileEditor::write_multiple(template.files, user_id, session_id).await;

    let success_count = results.iter().filter(|r| r.success).count();
    let failure_count = results.iter().filter(|r| !r.success).count();

    Ok(ParallelEditResponse {
        results,
        success_count,
        failure_count,
        total_duration_ms: 0,
    })
}

// ─── Code Analysis ──────────────────────────────────────────────────────────

/// تحليل الكود — إحصائيات وأسئلة
#[derive(Debug, Clone, Serialize)]
pub struct CodeAnalysis {
    pub file_count: usize,
    pub total_lines: usize,
    pub total_size_bytes: usize,
    pub languages: HashMap<String, usize>,
    pub largest_files: Vec<(String, usize)>,
    pub potential_issues: Vec<String>,
}

/// تحليل جودة الكود في مشروع المستخدم
pub async fn analyze_code_quality(
    user_id: &str,
    session_id: &str,
) -> Result<CodeAnalysis, String> {
    let files = storage::list_files(user_id, session_id).await?;

    let mut total_lines = 0usize;
    let mut total_size = 0usize;
    let mut languages: HashMap<String, usize> = HashMap::new();
    let mut largest_files: Vec<(String, usize)> = Vec::new();
    let mut issues: Vec<String> = Vec::new();

    for fname in &files {
        // تقدير اللغة
        if let Some(ext) = fname.rsplit('.').next() {
            *languages.entry(ext.to_string()).or_insert(0) += 1;
        }

        // اقرأ الملف
        if let Ok(content) = storage::read_file(user_id, session_id, fname).await {
            let lines = content.lines().count();
            let size = content.len();

            total_lines += lines;
            total_size += size;

            largest_files.push((fname.clone(), lines));

            // كشف مشاكل محتملة
            if lines > 500 {
                issues.push(format!("{fname}: ملف كبير جداً ({lines} سطر)"));
            }
            if content.contains("TODO") || content.contains("FIXME") {
                issues.push(format!("{fname}: يحتوي على TODO/FIXME"));
            }
            if content.contains("dbg!") || content.contains("println!") {
                let count = content.matches("dbg!").count() + content.matches("println!").count();
                if count > 3 {
                    issues.push(format!("{fname}: {count} استدعاء طباعة للتصحيح"));
                }
            }
            if content.contains("unwrap(") {
                let count = content.matches("unwrap(").count();
                if count > 5 {
                    issues.push(format!("{fname}: {count} استدعاء unwrap() — قد يسبب panic"));
                }
            }
        }
    }

    // رتب أكبر الملفات
    largest_files.sort_by(|a, b| b.1.cmp(&a.1));
    largest_files.truncate(10);

    Ok(CodeAnalysis {
        file_count: files.len(),
        total_lines,
        total_size_bytes: total_size,
        languages,
        largest_files,
        potential_issues: issues,
    })
}

// ─── Batch File Operations ───────────────────────────────────────────────────

/// قراءة ملفات متعددة بالتوازي مع تصفية حسب الامتداد
pub async fn read_files_by_extension(
    extension: &str,
    user_id: &str,
    session_id: &str,
) -> Result<Vec<FileEditResult>, String> {
    let all_files = storage::list_files(user_id, session_id).await?;

    // تصفية حسب الامتداد
    let matching: Vec<String> = all_files
        .into_iter()
        .filter(|f| f.ends_with(extension))
        .collect();

    if matching.is_empty() {
        return Ok(Vec::new());
    }

    Ok(MultiFileEditor::read_multiple(matching, user_id, session_id).await)
}

/// البحث عن نص في جميع ملفات المشروع
pub async fn search_in_files(
    query: &str,
    user_id: &str,
    session_id: &str,
) -> Result<Vec<(String, Vec<usize>)>, String> {
    let files = storage::list_files(user_id, session_id).await?;
    let mut results = Vec::new();

    for fname in &files {
        if let Ok(content) = storage::read_file(user_id, session_id, fname).await {
            let lines: Vec<usize> = content
                .lines()
                .enumerate()
                .filter(|(_, line)| line.contains(query))
                .map(|(idx, _)| idx + 1)
                .collect();

            if !lines.is_empty() {
                results.push((fname.clone(), lines));
            }
        }
    }

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parallel_edit_request() {
        let req = ParallelEditRequest {
            operations: vec![
                FileEditOperation {
                    path: "main.rs".into(),
                    operation: FileOp::Read,
                    content: None,
                    old_str: None,
                    new_str: None,
                },
            ],
            session_id: "test-session".into(),
        };
        assert_eq!(req.operations.len(), 1);
    }

    #[test]
    fn test_project_template() {
        let template = ProjectTemplate {
            name: "test".into(),
            description: "Test project".into(),
            files: vec![
                ("main.rs".into(), "fn main() {}".into()),
                ("lib.rs".into(), "pub fn hello() {}".into()),
            ],
        };
        assert_eq!(template.files.len(), 2);
    }
}
