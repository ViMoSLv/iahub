//! Mega Brain V0 — Git Operations (Phase 0, Gap: Workspace Provisioning)
//!
//! Manages git worktree lifecycle for isolated per-Attempt workspaces.
//! Each Attempt gets its own worktree with a dedicated branch, preventing
//! cross-contamination between concurrent agents.
//!
//! Flow:
//! 1. `provision_worktree` — creates branch + worktree for an Attempt
//! 2. Agent works in the worktree directory (PTY cwd)
//! 3. `remove_worktree` — cleans up after verification/merge or failure

use std::path::{Path, PathBuf};
use std::process::Command;

/// Error from git operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GitError {
    /// Git command failed with non-zero exit code.
    CommandFailed { command: String, stderr: String },
    /// Worktree path already exists.
    PathExists { path: String },
    /// Repository path is not a valid git repo.
    NotARepository { path: String },
    /// IO error during filesystem operations.
    IoError { message: String },
}

impl std::fmt::Display for GitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CommandFailed { command, stderr } => {
                write!(f, "git command '{}' failed: {}", command, stderr)
            }
            Self::PathExists { path } => write!(f, "path already exists: {}", path),
            Self::NotARepository { path } => write!(f, "not a git repository: {}", path),
            Self::IoError { message } => write!(f, "IO error: {}", message),
        }
    }
}

impl std::error::Error for GitError {}

/// Result of provisioning a worktree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorktreeInfo {
    /// Absolute path to the worktree directory.
    pub worktree_path: PathBuf,
    /// Branch name created for this worktree.
    pub branch_name: String,
    /// The base commit SHA the worktree was created from.
    pub base_commit: String,
}

/// Git worktree manager for isolated per-Attempt workspaces.
pub struct WorktreeManager {
    /// Path to the main repository (.git directory or working tree root).
    repo_path: PathBuf,
    /// Base directory where worktrees are created.
    worktree_root: PathBuf,
}

impl WorktreeManager {
    /// Create a new worktree manager for the given repository.
    ///
    /// # Arguments
    /// * `repo_path` — Path to the git repository root
    /// * `worktree_root` — Base directory for worktree storage (e.g., `<repo>/.ia-hub/attempts/`)
    pub fn new(repo_path: &Path, worktree_root: &Path) -> Self {
        Self {
            repo_path: repo_path.to_path_buf(),
            worktree_root: worktree_root.to_path_buf(),
        }
    }

    /// Provision a new worktree for an Attempt.
    ///
    /// Creates a new branch `ia-hub/attempt-<attempt_id>` from HEAD and
    /// checks it out into a worktree at `<worktree_root>/<attempt_id>/`.
    ///
    /// # Arguments
    /// * `attempt_id` — Unique identifier for this attempt (used in branch name and path)
    ///
    /// # Returns
    /// `WorktreeInfo` with the path, branch name, and base commit SHA.
    pub fn provision_worktree(&self, attempt_id: &str) -> Result<WorktreeInfo, GitError> {
        let branch_name = format!("ia-hub/attempt-{}", attempt_id);
        let worktree_path = self.worktree_root.join(attempt_id);

        // Check if path already exists
        if worktree_path.exists() {
            return Err(GitError::PathExists {
                path: worktree_path.to_string_lossy().to_string(),
            });
        }

        // Ensure worktree root exists
        std::fs::create_dir_all(&self.worktree_root).map_err(|e| GitError::IoError {
            message: format!("failed to create worktree root: {}", e),
        })?;

        // Get current HEAD commit SHA
        let base_commit = self.git_output(&["rev-parse", "HEAD"])?;

        // Create branch and worktree in one command
        // git worktree add -b <branch> <path> <start-point>
        self.git_run(&[
            "worktree",
            "add",
            "-b",
            &branch_name,
            &worktree_path.to_string_lossy(),
            &base_commit,
        ])?;

        Ok(WorktreeInfo {
            worktree_path,
            branch_name,
            base_commit,
        })
    }

    /// Remove a worktree and its associated branch.
    ///
    /// # Arguments
    /// * `attempt_id` — The attempt whose worktree should be removed
    /// * `force` — If true, remove even if there are uncommitted changes
    pub fn remove_worktree(&self, attempt_id: &str, force: bool) -> Result<(), GitError> {
        let worktree_path = self.worktree_root.join(attempt_id);
        let branch_name = format!("ia-hub/attempt-{}", attempt_id);

        // Remove the worktree
        let mut args = vec!["worktree", "remove"];
        if force {
            args.push("--force");
        }
        let path_str = worktree_path.to_string_lossy().to_string();
        args.push(&path_str);
        self.git_run(&args)?;

        // Delete the branch
        let mut branch_args = vec!["branch", "-D"];
        branch_args.push(&branch_name);
        let _ = self.git_run(&branch_args); // Best effort — branch may already be deleted

        Ok(())
    }

    /// List all active worktrees managed by this manager.
    pub fn list_worktrees(&self) -> Result<Vec<WorktreeInfo>, GitError> {
        let output = self.git_output(&["worktree", "list", "--porcelain"])?;
        let mut worktrees = Vec::new();
        let mut current_path: Option<PathBuf> = None;
        let mut current_branch: Option<String> = None;

        for line in output.lines() {
            if let Some(path) = line.strip_prefix("worktree ") {
                current_path = Some(PathBuf::from(path));
            } else if let Some(branch) = line.strip_prefix("branch refs/heads/") {
                current_branch = Some(branch.to_string());
            } else if line.is_empty() {
                if let (Some(path), Some(branch)) = (current_path.take(), current_branch.take()) {
                    if branch.starts_with("ia-hub/attempt-") && path.starts_with(&self.worktree_root) {
                        worktrees.push(WorktreeInfo {
                            worktree_path: path,
                            branch_name: branch,
                            base_commit: String::new(), // Not available from porcelain listing
                        });
                    }
                }
            }
        }
        // Handle last entry if no trailing newline
        if let (Some(path), Some(branch)) = (current_path, current_branch) {
            if branch.starts_with("ia-hub/attempt-") && path.starts_with(&self.worktree_root) {
                worktrees.push(WorktreeInfo {
                    worktree_path: path,
                    branch_name: branch,
                    base_commit: String::new(),
                });
            }
        }

        Ok(worktrees)
    }

    /// Get the diff of a worktree against its base commit.
    /// Returns the unified diff as a string.
    pub fn get_worktree_diff(&self, attempt_id: &str) -> Result<String, GitError> {
        let worktree_path = self.worktree_root.join(attempt_id);
        // Run git diff in the worktree directory
        let output = Command::new("git")
            .args(["diff", "HEAD"])
            .current_dir(&worktree_path)
            .output()
            .map_err(|e| GitError::IoError {
                message: format!("failed to run git diff: {}", e),
            })?;

        if !output.status.success() {
            return Err(GitError::CommandFailed {
                command: "git diff HEAD".to_string(),
                stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            });
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    /// Run a git command in the repository and return stdout.
    fn git_output(&self, args: &[&str]) -> Result<String, GitError> {
        let output = Command::new("git")
            .args(args)
            .current_dir(&self.repo_path)
            .output()
            .map_err(|e| GitError::IoError {
                message: format!("failed to run git {}: {}", args.first().unwrap_or(&"?"), e),
            })?;

        if !output.status.success() {
            return Err(GitError::CommandFailed {
                command: format!("git {}", args.join(" ")),
                stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            });
        }

        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    /// Run a git command in the repository (no output capture needed).
    fn git_run(&self, args: &[&str]) -> Result<(), GitError> {
        self.git_output(args)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn worktree_info_fields() {
        let info = WorktreeInfo {
            worktree_path: PathBuf::from("/repo/.ia-hub/attempts/att-1"),
            branch_name: "ia-hub/attempt-att-1".to_string(),
            base_commit: "abc123def".to_string(),
        };
        assert_eq!(info.branch_name, "ia-hub/attempt-att-1");
        assert_eq!(info.base_commit, "abc123def");
        assert!(info.worktree_path.to_string_lossy().contains("att-1"));
    }

    #[test]
    fn git_error_display() {
        let err = GitError::CommandFailed {
            command: "git worktree add".to_string(),
            stderr: "fatal: not a git repository".to_string(),
        };
        let display = format!("{}", err);
        assert!(display.contains("git worktree add"));
        assert!(display.contains("not a git repository"));
    }

    #[test]
    fn git_error_path_exists() {
        let err = GitError::PathExists {
            path: "/some/path".to_string(),
        };
        assert!(format!("{}", err).contains("/some/path"));
    }

    #[test]
    fn branch_name_format() {
        let attempt_id = "uuid-1234-5678";
        let branch = format!("ia-hub/attempt-{}", attempt_id);
        assert_eq!(branch, "ia-hub/attempt-uuid-1234-5678");
        assert!(branch.starts_with("ia-hub/attempt-"));
    }

    #[test]
    fn worktree_path_construction() {
        let manager = WorktreeManager::new(
            Path::new("/repo"),
            Path::new("/repo/.ia-hub/attempts"),
        );
        let expected_path = manager.worktree_root.join("att-42");
        assert_eq!(
            expected_path,
            PathBuf::from("/repo/.ia-hub/attempts/att-42")
        );
    }

    #[test]
    fn provision_fails_if_path_exists() {
        let dir = tempfile::tempdir().unwrap();
        let worktree_root = dir.path().join("worktrees");
        std::fs::create_dir_all(worktree_root.join("existing")).unwrap();

        let manager = WorktreeManager::new(dir.path(), &worktree_root);
        let result = manager.provision_worktree("existing");
        assert!(result.is_err());
        match result.unwrap_err() {
            GitError::PathExists { .. } => {}
            other => panic!("expected PathExists, got {:?}", other),
        }
    }
}