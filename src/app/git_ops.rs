use crate::git;
use crate::types::{GitRemoteCheckResult, PaneId};
use crate::ui;

use super::App;

impl App {
    pub(super) fn check_repo_change(&mut self) {
        // Find current repo root
        let current_repo = git::find_repo_root(&self.file_browser.current_dir);

        // Check if repo changed
        let repo_changed = match (&current_repo, &self.git_remote.last_repo_root) {
            (Some(curr), Some(last)) => curr != last,
            (Some(_), None) => true,  // Entered a repo
            (None, Some(_)) => false, // Left a repo, don't check
            (None, None) => false,    // Still no repo
        };

        if repo_changed && !self.git_remote.checking {
            if let Some(repo_root) = &current_repo {
                // Get current branch
                if let Some(branch) = git::get_current_branch(repo_root) {
                    // Start async check
                    self.git_remote.checking = true;
                    self.git_check_receiver =
                        Some(git::check_remote_changes_async(repo_root, &branch));
                }
            }
        }

        // Update last repo
        self.git_remote.last_repo_root = current_repo;
    }

    /// Add selected file/folder to .gitignore and open it in editor
    pub(super) fn add_to_gitignore(&mut self, path: &std::path::Path) {
        use std::fs::OpenOptions;
        use std::io::Write;

        // 1. Find .gitignore path (in Git root or current dir)
        let gitignore_path = if let Some(repo_root) = &self.file_browser.repo_root {
            repo_root.join(".gitignore")
        } else {
            self.file_browser.current_dir.join(".gitignore")
        };

        // 2. Compute relative path from repo root (or just filename)
        let relative_path = if let Some(repo_root) = &self.file_browser.repo_root {
            path.strip_prefix(repo_root)
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|_| {
                    path.file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_default()
                })
        } else {
            path.file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default()
        };

        // Skip if empty
        if relative_path.is_empty() {
            return;
        }

        // 3. Append entry to .gitignore
        if let Ok(mut file) = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&gitignore_path)
        {
            // Add newline before entry if file exists and doesn't end with newline
            if gitignore_path.exists() {
                if let Ok(content) = std::fs::read_to_string(&gitignore_path) {
                    if !content.is_empty() && !content.ends_with('\n') {
                        let _ = writeln!(file);
                    }
                }
            }

            // Add the path (with trailing slash for directories)
            if path.is_dir() {
                let _ = writeln!(file, "{}/", relative_path);
            } else {
                let _ = writeln!(file, "{}", relative_path);
            }
        }

        // 4. Open .gitignore in preview and enter edit mode
        self.preview.load_file(gitignore_path, &self.syntax_manager);
        self.preview.enter_edit_mode();
        self.active_pane = PaneId::Preview;

        // 5. Scroll to bottom to show new entry
        if let Some(editor) = &mut self.preview.editor {
            let line_count = editor.lines().len();
            if line_count > 0 {
                editor.move_cursor(tui_textarea::CursorMove::Jump((line_count - 1) as u16, 0));
            }
        }

        // 6. Refresh file browser to update git status
        self.file_browser.refresh();
    }

    /// Poll for git remote check results and show dialog if needed
    pub(super) fn poll_git_check(&mut self) {
        if let Some(ref receiver) = self.git_check_receiver {
            // Non-blocking check for result
            if let Ok(result) = receiver.try_recv() {
                self.git_remote.checking = false;

                match result {
                    GitRemoteCheckResult::RemoteAhead {
                        commits_ahead,
                        branch,
                    } => {
                        // Show pull confirmation dialog
                        if let Some(repo_root) = self.git_remote.last_repo_root.clone() {
                            use ui::dialog::{DialogAction, DialogType};
                            self.dialog.dialog_type = DialogType::Confirm {
                                title: "Git Pull".to_string(),
                                message: format!(
                                    "Branch '{}' is {} commit{} behind remote. Pull now?",
                                    branch,
                                    commits_ahead,
                                    if commits_ahead == 1 { "" } else { "s" }
                                ),
                                action: DialogAction::GitPull { repo_root },
                            };
                        }
                    }
                    GitRemoteCheckResult::UpToDate => {
                        // No action needed
                    }
                    GitRemoteCheckResult::Error(_err) => {
                        // Silently ignore errors (no network, no remote, etc.)
                    }
                }

                // Clear receiver
                self.git_check_receiver = None;
            }
        }
    }
}
