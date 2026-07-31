//! VCS tool using git2 for version control operations
//!
//! Provides Git integration for checkpoints, rollback, and history management.

use git2::{Error as GitError, Repository, Status, StatusShow};
use std::path::Path;
use std::sync::Arc;
use thiserror::Error;

/// Errors that can occur during VCS operations
#[derive(Error, Debug)]
pub enum VcsError {
    #[error("Git error: {0}")]
    GitError(#[from] GitError),

    #[error("Repository not found: {0}")]
    RepoNotFound(String),

    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),
}

/// Information about a Git repository
#[derive(Debug, Clone)]
pub struct RepositoryInfo {
    pub path: String,
    pub current_branch: String,
    pub is_dirty: bool,
    pub ahead: usize,
    pub behind: usize,
}

/// A single change in the repository
#[derive(Debug, Clone)]
pub struct GitChange {
    pub path: String,
    pub status: String,
    pub additions: usize,
    pub deletions: usize,
}

/// Commit information
#[derive(Debug, Clone)]
pub struct CommitInfo {
    pub id: String,
    pub message: String,
    pub author: String,
    pub timestamp: String,
}

/// Configuration for VCS operations
#[derive(Debug, Clone)]
pub struct VcsConfig {
    pub repo_path: String,
}

impl Default for VcsConfig {
    fn default() -> Self {
        Self {
            repo_path: ".".to_string(),
        }
    }
}

/// High-level VCS tool using git2
#[derive(Clone)]
pub struct VcsTool {
    config: Arc<VcsConfig>,
}

impl VcsTool {
    /// Create a new VcsTool with the given configuration
    pub fn new(config: VcsConfig) -> Self {
        Self {
            config: Arc::new(config),
        }
    }

    /// Create a VcsTool with default configuration
    pub fn default() -> Self {
        Self::new(VcsConfig::default())
    }

    /// Open the repository
    pub fn open_repo(&self) -> Result<Repository, VcsError> {
        let repo = Repository::open(&self.config.repo_path)?;
        Ok(repo)
    }

    /// Get repository information
    pub fn get_info(&self) -> Result<RepositoryInfo, VcsError> {
        let repo = self.open_repo()?;

        let head = repo.head()?;
        let current_branch = head.shorthand().unwrap_or("unknown").to_string();

        let statuses = repo.statuses(Some(
            git2::StatusOptions::new()
                .include_untracked(false)
                .renames_head_to_index(false)
                .renames_index_to_workdir(false),
        ))?;

        let is_dirty = !statuses.is_empty();

        // Calculate ahead/behind
        let (ahead, behind) = if let Ok(branch) =
            repo.find_branch(head.shorthand().unwrap_or(""), git2::BranchType::Local)
        {
            if let Ok(upstream) = branch.upstream() {
                let upstream_oid = upstream.get().peel_to_commit()?.id();
                let head_oid = head.peel_to_commit()?.id();
                repo.graph_ahead_behind(head_oid, upstream_oid)
                    .unwrap_or((0, 0))
            } else {
                (0, 0)
            }
        } else {
            (0, 0)
        };

        Ok(RepositoryInfo {
            path: self.config.repo_path.clone(),
            current_branch,
            is_dirty,
            ahead,
            behind,
        })
    }

    /// Get staged changes
    pub fn get_staged_changes(&self) -> Result<Vec<GitChange>, VcsError> {
        let repo = self.open_repo()?;
        let statuses = repo.statuses(Some(
            git2::StatusOptions::new()
                .include_untracked(false)
                .show(StatusShow::Index),
        ))?;

        let mut changes = Vec::new();

        for entry in statuses.iter() {
            if let Ok(path) = entry.path() {
                let status = format_status(entry.status());
                changes.push(GitChange {
                    path: path.to_string(),
                    status,
                    additions: 0,
                    deletions: 0,
                });
            }
        }

        Ok(changes)
    }

    /// Get working directory changes
    pub fn get_working_changes(&self) -> Result<Vec<GitChange>, VcsError> {
        let repo = self.open_repo()?;
        let statuses = repo.statuses(Some(
            git2::StatusOptions::new()
                .include_untracked(true)
                .show(StatusShow::Workdir),
        ))?;

        let mut changes = Vec::new();

        for entry in statuses.iter() {
            if let Ok(path) = entry.path() {
                let status = format_status(entry.status());
                changes.push(GitChange {
                    path: path.to_string(),
                    status,
                    additions: 0,
                    deletions: 0,
                });
            }
        }

        Ok(changes)
    }

    /// Create a checkpoint (commit) with the given message
    pub fn checkpoint(&self, message: &str) -> Result<String, VcsError> {
        let repo = self.open_repo()?;

        // Stage all changes
        let mut index = repo.index()?;
        index.add_all(["*"], git2::IndexAddOption::DEFAULT, None)?;
        index.write()?;

        // Get HEAD tree
        let head = repo.head()?;
        let parent = head.peel_to_commit()?;
        let tree = index.write_tree()?;
        let tree = repo.find_tree(tree)?;

        // Create commit
        let sig = repo.signature()?;
        let commit_id = repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &[&parent])?;

        Ok(commit_id.to_string())
    }

    /// Rollback to the previous commit
    pub fn rollback(&self) -> Result<(), VcsError> {
        let repo = self.open_repo()?;

        // Reset to HEAD
        let head = repo.head()?.peel(git2::ObjectType::Commit)?;
        repo.reset(&head, git2::ResetType::Soft, None)?;

        Ok(())
    }

    /// Create a new branch
    pub fn create_branch(&self, name: &str) -> Result<(), VcsError> {
        let repo = self.open_repo()?;
        let head = repo.head()?.peel_to_commit()?;

        repo.branch(name, &head, false)?;

        Ok(())
    }

    /// Checkout a branch
    pub fn checkout_branch(&self, name: &str) -> Result<(), VcsError> {
        let repo = self.open_repo()?;

        let (object, reference) = repo.revparse_ext(&format!("refs/heads/{}", name))?;
        repo.checkout_tree(&object, Some(git2::build::CheckoutBuilder::new().force()))?;

        if let Some(_reference) = reference {
            // Reference exists — checkout successful
        }

        Ok(())
    }

    /// Get commit history
    pub fn get_history(&self, max_count: Option<usize>) -> Result<Vec<CommitInfo>, VcsError> {
        let repo = self.open_repo()?;
        let _head = repo.head()?;
        let mut revwalk = repo.revwalk()?;

        revwalk.push_head()?;

        if let Some(_count) = max_count {
            revwalk.set_sorting(git2::Sort::TIME)?;
        }

        let mut commits = Vec::new();
        let iter = if let Some(count) = max_count {
            revwalk.take(count)
        } else {
            // Use a large count since we can't use revwalk directly in a take() without count
            revwalk.take(usize::MAX)
        };

        for oid in iter {
            let oid = oid?;
            let commit = repo.find_commit(oid)?;

            let id = commit.id().to_string();
            let message = commit.message().unwrap_or("").to_string();
            let author = commit.author().name().unwrap_or("unknown").to_string();
            let time = commit.time().seconds();
            let timestamp = chrono::DateTime::from_timestamp(time, 0)
                .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                .unwrap_or_else(|| "unknown".to_string());

            commits.push(CommitInfo {
                id,
                message,
                author,
                timestamp,
            });
        }

        Ok(commits)
    }
}

/// Format git status to string
fn format_status(status: Status) -> String {
    let mut parts = Vec::new();

    if status.contains(Status::INDEX_NEW) {
        parts.push("new");
    }
    if status.contains(Status::INDEX_MODIFIED) {
        parts.push("modified");
    }
    if status.contains(Status::INDEX_DELETED) {
        parts.push("deleted");
    }
    if status.contains(Status::INDEX_RENAMED) {
        parts.push("renamed");
    }
    if status.contains(Status::INDEX_TYPECHANGE) {
        parts.push("typechange");
    }
    if status.contains(Status::WT_NEW) {
        parts.push("untracked");
    }
    if status.contains(Status::WT_MODIFIED) {
        parts.push("modified");
    }
    if status.contains(Status::WT_DELETED) {
        parts.push("deleted");
    }
    if status.contains(Status::WT_RENAMED) {
        parts.push("renamed");
    }
    if status.contains(Status::WT_TYPECHANGE) {
        parts.push("typechange");
    }

    if parts.is_empty() {
        "unchanged".to_string()
    } else {
        parts.join(", ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_status() {
        let status = Status::INDEX_NEW | Status::WT_MODIFIED;
        let formatted = format_status(status);
        assert!(formatted.contains("new"));
        assert!(formatted.contains("modified"));
    }

    #[test]
    fn test_vcs_config_default() {
        let config = VcsConfig::default();
        assert_eq!(config.repo_path, ".");
    }
}
