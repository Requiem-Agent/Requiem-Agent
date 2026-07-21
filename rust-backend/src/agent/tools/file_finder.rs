//! File finder tool using ignore crate for fast file discovery
//!
//! Provides fast file discovery with support for .gitignore rules.

use ignore::WalkBuilder;
use std::path::Path;
use std::sync::Arc;
use thiserror::Error;

/// Errors that can occur during file finding operations
#[derive(Error, Debug)]
pub enum FileFinderError {
    #[error("Path does not exist: {0}")]
    PathNotFound(String),

    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Ignore error: {0}")]
    IgnoreError(#[from] ignore::Error),
}

/// A found file
#[derive(Debug, Clone)]
pub struct FoundFile {
    pub path: String,
    pub is_directory: bool,
    pub size: Option<u64>,
    pub extension: Option<String>,
}

/// Configuration for file finding operations
#[derive(Debug, Clone)]
pub struct FileFinderConfig {
    pub include_hidden: bool,
    pub no_ignore: bool,
    pub max_depth: Option<usize>,
}

impl Default for FileFinderConfig {
    fn default() -> Self {
        Self {
            include_hidden: false,
            no_ignore: false,
            max_depth: None,
        }
    }
}

/// Fast file finder using ignore crate
#[derive(Clone)]
pub struct FileFinderTool {
    config: Arc<FileFinderConfig>,
}

impl FileFinderTool {
    /// Create a new FileFinderTool with the given configuration
    pub fn new(config: FileFinderConfig) -> Self {
        Self {
            config: Arc::new(config),
        }
    }

    /// Create a FileFinderTool with default configuration
    pub fn default() -> Self {
        Self::new(FileFinderConfig::default())
    }

    /// Find all files in a directory
    pub fn find_files(&self, root_path: &str) -> Result<Vec<FoundFile>, FileFinderError> {
        let root = Path::new(root_path);
        if !root.exists() {
            return Err(FileFinderError::PathNotFound(root_path.to_string()));
        }

        let mut walker = WalkBuilder::new(root);
        walker
            .hidden(!self.config.include_hidden)
            .ignore(!self.config.no_ignore)
            .git_ignore(!self.config.no_ignore);

        if let Some(depth) = self.config.max_depth {
            walker.max_depth(Some(depth));
        }

        let mut files = Vec::new();

        for entry in walker.build() {
            let entry = entry?;
            let path = entry.path();

            let metadata = entry.metadata()?;
            let is_directory = metadata.is_dir();

            let extension = path
                .extension()
                .and_then(|e| e.to_str())
                .map(|s| s.to_string());

            files.push(FoundFile {
                path: path.to_string_lossy().to_string(),
                is_directory,
                size: if is_directory {
                    None
                } else {
                    Some(metadata.len())
                },
                extension,
            });
        }

        Ok(files)
    }

    /// Find files by extension
    pub fn find_by_extension(
        &self,
        root_path: &str,
        extension: &str,
    ) -> Result<Vec<FoundFile>, FileFinderError> {
        let all_files = self.find_files(root_path)?;

        Ok(all_files
            .into_iter()
            .filter(|f| !f.is_directory && f.extension.as_deref() == Some(extension))
            .collect())
    }

    /// Find files by name pattern
    pub fn find_by_name(
        &self,
        root_path: &str,
        pattern: &str,
    ) -> Result<Vec<FoundFile>, FileFinderError> {
        let all_files = self.find_files(root_path)?;

        Ok(all_files
            .into_iter()
            .filter(|f| f.path.contains(pattern))
            .collect())
    }

    /// Find directories only
    pub fn find_directories(&self, root_path: &str) -> Result<Vec<FoundFile>, FileFinderError> {
        let all_files = self.find_files(root_path)?;

        Ok(all_files.into_iter().filter(|f| f.is_directory).collect())
    }

    /// Count files by extension
    pub fn count_by_extension(
        &self,
        root_path: &str,
    ) -> Result<std::collections::HashMap<String, usize>, FileFinderError> {
        let all_files = self.find_files(root_path)?;
        let mut counts = std::collections::HashMap::new();

        for file in all_files {
            if !file.is_directory {
                let ext = file.extension.unwrap_or_else(|| "unknown".to_string());
                *counts.entry(ext).or_insert(0) += 1;
            }
        }

        Ok(counts)
    }

    /// Get file size statistics
    pub fn get_size_stats(&self, root_path: &str) -> Result<SizeStats, FileFinderError> {
        let all_files = self.find_files(root_path)?;
        let mut total_size = 0;
        let mut file_count = 0;
        let mut dir_count = 0;
        let mut largest_file = (String::new(), 0u64);
        let mut smallest_file = (String::new(), u64::MAX);

        for file in &all_files {
            if file.is_directory {
                dir_count += 1;
            } else {
                file_count += 1;
                if let Some(size) = file.size {
                    total_size += size;

                    if size > largest_file.1 {
                        largest_file = (file.path.clone(), size);
                    }

                    if size < smallest_file.1 {
                        smallest_file = (file.path.clone(), size);
                    }
                }
            }
        }

        // Handle case where no files were found
        if smallest_file.1 == u64::MAX {
            smallest_file = (String::new(), 0);
        }

        Ok(SizeStats {
            total_size,
            file_count,
            dir_count,
            average_size: if file_count > 0 {
                total_size / file_count as u64
            } else {
                0
            },
            largest_file,
            smallest_file,
        })
    }
}

/// Statistics about file sizes
#[derive(Debug, Clone)]
pub struct SizeStats {
    pub total_size: u64,
    pub file_count: usize,
    pub dir_count: usize,
    pub average_size: u64,
    pub largest_file: (String, u64),
    pub smallest_file: (String, u64),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_finder_config_default() {
        let config = FileFinderConfig::default();
        assert!(!config.include_hidden);
        assert!(!config.no_ignore);
        assert_eq!(config.max_depth, None);
    }

    #[test]
    fn test_find_files() {
        let tool = FileFinderTool::default();
        let result = tool.find_files(".");
        assert!(result.is_ok());
    }
}
