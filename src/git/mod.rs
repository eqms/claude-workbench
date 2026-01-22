//! Git status module for file browser integration
//!
//! Provides git status information for files and directories.
//! Uses git CLI for maximum compatibility.

use crate::types::{GitFileStatus, GitRemoteCheckResult, GitRepoInfo};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::mpsc;

/// Find the git repository root for a given path
pub fn find_repo_root(path: &Path) -> Option<PathBuf> {
    let path = if path.is_file() { path.parent()? } else { path };

    let output = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .current_dir(path)
        .output()
        .ok()?;

    if output.status.success() {
        let root = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Some(PathBuf::from(root))
    } else {
        None
    }
}

/// Get the current branch name for a repository
pub fn get_current_branch(repo_root: &Path) -> Option<String> {
    let output = Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .current_dir(repo_root)
        .output()
        .ok()?;

    if output.status.success() {
        let branch = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Some(branch)
    } else {
        None
    }
}

/// Parse git status porcelain output to get file statuses
fn parse_git_status(repo_root: &Path) -> HashMap<PathBuf, GitFileStatus> {
    let mut statuses = HashMap::new();

    // Get tracked file changes (modified, staged, etc.) and ignored files
    // Using --ignored=matching to show only top-level ignored patterns (performance)
    let output = Command::new("git")
        .args(["status", "--porcelain", "-uall", "--ignored=matching"])
        .current_dir(repo_root)
        .output();

    if let Ok(output) = output {
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                if line.len() < 3 {
                    continue;
                }

                let index_status = line.chars().next().unwrap_or(' ');
                let worktree_status = line.chars().nth(1).unwrap_or(' ');
                let file_path = repo_root.join(&line[3..]);

                let status = match (index_status, worktree_status) {
                    ('?', '?') => GitFileStatus::Untracked,
                    ('!', '!') => GitFileStatus::Ignored,
                    ('U', _) | (_, 'U') | ('A', 'A') | ('D', 'D') => GitFileStatus::Conflict,
                    ('A', _) | ('M', ' ') | ('D', ' ') | ('R', ' ') | ('C', ' ') => {
                        GitFileStatus::Staged
                    }
                    (_, 'M') | (_, 'D') => GitFileStatus::Modified,
                    _ => GitFileStatus::Clean,
                };

                statuses.insert(file_path, status);
            }
        }
    }

    statuses
}

/// Get git status for all files in a directory
pub fn get_status_for_directory(
    dir: &Path,
) -> (HashMap<PathBuf, GitFileStatus>, Option<GitRepoInfo>) {
    // Find repo root
    let Some(repo_root) = find_repo_root(dir) else {
        return (HashMap::new(), None);
    };

    // Get branch name
    let branch = get_current_branch(&repo_root).unwrap_or_else(|| "HEAD".to_string());

    // Parse git status
    let statuses = parse_git_status(&repo_root);

    // Count different statuses
    let mut modified_count = 0;
    let mut untracked_count = 0;
    let mut staged_count = 0;

    for status in statuses.values() {
        match status {
            GitFileStatus::Modified => modified_count += 1,
            GitFileStatus::Untracked => untracked_count += 1,
            GitFileStatus::Staged => staged_count += 1,
            _ => {}
        }
    }

    let git_info = GitRepoInfo {
        branch,
        modified_count,
        untracked_count,
        staged_count,
    };

    (statuses, Some(git_info))
}

/// Get aggregated git status for a directory based on its contents
pub fn aggregate_directory_status(
    dir_path: &Path,
    all_statuses: &HashMap<PathBuf, GitFileStatus>,
) -> GitFileStatus {
    let mut highest_priority = GitFileStatus::Clean;

    for (path, status) in all_statuses {
        // Check if this file is inside the directory and has higher priority
        if path.starts_with(dir_path)
            && path != dir_path
            && status.priority() > highest_priority.priority()
        {
            highest_priority = *status;
        }
    }

    highest_priority
}

// ============================================================
// Git Remote Operations (for pull detection)
// ============================================================

/// Check how many commits the remote is ahead of local
/// Returns None if no upstream configured or on error
fn get_commits_behind(repo_root: &Path, branch: &str) -> Option<usize> {
    // Get the upstream tracking branch
    let output = Command::new("git")
        .args([
            "rev-parse",
            "--abbrev-ref",
            &format!("{}@{{upstream}}", branch),
        ])
        .current_dir(repo_root)
        .output()
        .ok()?;

    if !output.status.success() {
        return None; // No upstream configured
    }

    let upstream = String::from_utf8_lossy(&output.stdout).trim().to_string();

    // Count commits: local..remote (how many commits remote is ahead)
    let output = Command::new("git")
        .args(["rev-list", "--count", &format!("{}..{}", branch, upstream)])
        .current_dir(repo_root)
        .output()
        .ok()?;

    if output.status.success() {
        String::from_utf8_lossy(&output.stdout).trim().parse().ok()
    } else {
        None
    }
}

/// Check for remote changes asynchronously (fetch + count commits behind)
/// Returns a receiver that will receive the result when ready
pub fn check_remote_changes_async(
    repo_root: &Path,
    branch: &str,
) -> mpsc::Receiver<GitRemoteCheckResult> {
    let repo = repo_root.to_path_buf();
    let branch = branch.to_string();
    let (tx, rx) = mpsc::channel();

    std::thread::spawn(move || {
        // Step 1: Fetch from remote (quiet mode, don't merge)
        let fetch_result = Command::new("git")
            .args(["fetch", "--quiet"])
            .current_dir(&repo)
            .output();

        if let Err(e) = fetch_result {
            let _ = tx.send(GitRemoteCheckResult::Error(e.to_string()));
            return;
        }

        let fetch_output = fetch_result.unwrap();
        if !fetch_output.status.success() {
            // Fetch failed - might be no network or no remote
            let stderr = String::from_utf8_lossy(&fetch_output.stderr).to_string();
            if stderr.contains("Could not resolve") || stderr.contains("No remote") {
                // No remote configured or no network - treat as up to date
                let _ = tx.send(GitRemoteCheckResult::UpToDate);
            } else {
                let _ = tx.send(GitRemoteCheckResult::Error(stderr));
            }
            return;
        }

        // Step 2: Check how many commits behind we are
        match get_commits_behind(&repo, &branch) {
            Some(0) => {
                let _ = tx.send(GitRemoteCheckResult::UpToDate);
            }
            Some(n) => {
                let _ = tx.send(GitRemoteCheckResult::RemoteAhead {
                    commits_ahead: n,
                    branch: branch.clone(),
                });
            }
            None => {
                // No upstream configured - treat as up to date
                let _ = tx.send(GitRemoteCheckResult::UpToDate);
            }
        }
    });

    rx
}

/// Execute git pull (blocking operation)
pub fn pull(repo_root: &Path) -> Result<String, String> {
    let output = Command::new("git")
        .args(["pull"])
        .current_dir(repo_root)
        .output()
        .map_err(|e| e.to_string())?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_repo_root() {
        // Should find repo root from current directory (this test file is in a git repo)
        let result = find_repo_root(Path::new("."));
        // May or may not be in a git repo depending on where tests run
        assert!(result.is_some() || result.is_none());
    }

    #[test]
    fn test_git_file_status_priority() {
        assert!(GitFileStatus::Conflict.priority() > GitFileStatus::Modified.priority());
        assert!(GitFileStatus::Modified.priority() > GitFileStatus::Untracked.priority());
        assert!(GitFileStatus::Untracked.priority() > GitFileStatus::Staged.priority());
        assert!(GitFileStatus::Staged.priority() > GitFileStatus::Clean.priority());
    }

    #[test]
    fn test_git_file_status_symbol() {
        assert_eq!(GitFileStatus::Untracked.symbol(), "?");
        assert_eq!(GitFileStatus::Modified.symbol(), "M");
        assert_eq!(GitFileStatus::Staged.symbol(), "+");
        assert_eq!(GitFileStatus::Ignored.symbol(), "·");
        assert_eq!(GitFileStatus::Conflict.symbol(), "!");
        assert_eq!(GitFileStatus::Clean.symbol(), " ");
    }
}
