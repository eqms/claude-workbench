use std::borrow::Cow;
use std::path::Path;

use shell_escape::escape;

use crate::config::Config;
use crate::terminal::PseudoTerminal;
use crate::types::{ClaudePermissionMode, PaneId};

use super::App;

impl App {
    pub(super) fn build_claude_command(config: &Config, mode: ClaudePermissionMode) -> Vec<String> {
        let mut cmd = if config.pty.claude_command.is_empty() {
            // Default: use the same shell as Terminal pane
            let mut shell_cmd = vec![config.terminal.shell_path.clone()];
            shell_cmd.extend(config.terminal.shell_args.clone());
            shell_cmd
        } else {
            config.pty.claude_command.clone()
        };

        // Only add flags if using claude command (not shell)
        if !config.pty.claude_command.is_empty() {
            if mode.is_yolo() {
                // YOLO mode: --dangerously-skip-permissions flag
                if !cmd
                    .iter()
                    .any(|a| a.contains("--dangerously-skip-permissions"))
                {
                    cmd.push("--dangerously-skip-permissions".to_string());
                }
            } else if let Some(flag_value) = mode.cli_flag() {
                // Normal modes: --permission-mode flag
                if !cmd.iter().any(|a| a.contains("--permission-mode")) {
                    cmd.push("--permission-mode".to_string());
                    cmd.push(flag_value.to_string());
                }
            }
        }

        cmd
    }

    /// Initialize Claude PTY with the selected permission mode
    pub(super) fn init_claude_pty(&mut self, mode: ClaudePermissionMode) {
        self.claude_permission_mode = mode;
        self.claude_pty_pending = false;

        let claude_cmd = Self::build_claude_command(&self.config, mode);
        self.claude_command_used = claude_cmd.join(" ");

        let cwd = self.file_browser.current_dir.clone();
        let rows = 24;
        let cols = 80;

        match PseudoTerminal::new(&claude_cmd, rows, cols, &cwd) {
            Ok(pty) => {
                self.terminals.insert(PaneId::Claude, pty);
                self.claude_error = None;
                // Schedule /remote-control if enabled
                if self.config.claude.remote_control {
                    self.remote_control_send_time = Some(std::time::Instant::now());
                }
            }
            Err(e) => {
                self.claude_error = Some(format!(
                    "Failed to start shell\n\nCommand: {}\n\nError: {}",
                    self.claude_command_used, e
                ));
            }
        }
    }

    /// Initialize Claude PTY after wizard completion
    /// Shows permission mode dialog if configured, otherwise starts Claude directly
    pub(super) fn init_claude_after_wizard(&mut self) {
        // Remove existing Claude PTY (started with pre-wizard config)
        self.terminals.remove(&PaneId::Claude);
        self.claude_error = None;

        let should_show_permission_dialog = self.config.claude.show_permission_dialog;

        if should_show_permission_dialog {
            self.claude_pty_pending = true;
            self.permission_mode_dialog.open_with_default(
                self.config.claude.default_permission_mode,
                self.config.claude.remote_control,
            );
        } else {
            let mode = self
                .config
                .claude
                .default_permission_mode
                .unwrap_or(ClaudePermissionMode::Default);
            self.init_claude_pty(mode);
            self.active_pane = PaneId::Claude;
        }
    }

    #[allow(dead_code)]
    pub(super) fn sync_terminals_initial(&mut self) {
        let path_str = self.file_browser.current_dir.to_string_lossy();
        let escaped = escape(Cow::Borrowed(&path_str));
        let cmd = format!("cd {}\r", escaped);

        // Only sync to Terminal, NOT Claude (Claude needs time to start)
        if let Some(pty) = self.terminals.get_mut(&PaneId::Terminal) {
            let _ = pty.write_input(cmd.as_bytes());
        }
    }

    /// Sync directory to Terminal pane only (not Claude - Claude only gets cd at startup)
    pub(super) fn sync_terminals(&mut self) {
        let path_str = self.file_browser.current_dir.to_string_lossy();
        let escaped = escape(Cow::Borrowed(&path_str));
        let cmd = format!("cd {}\r", escaped);

        // Only sync to Terminal, not Claude (Claude should keep its initial directory)
        if let Some(pty) = self.terminals.get_mut(&PaneId::Terminal) {
            let _ = pty.write_input(cmd.as_bytes());
        }
    }

    /// Send cd command to a specific terminal pane
    pub(super) fn sync_terminal_to_current_dir(&mut self, pane: PaneId) {
        let path_str = self.file_browser.current_dir.to_string_lossy();
        let escaped = escape(Cow::Borrowed(&path_str));
        let cmd = format!("cd {}\r", escaped);

        if let Some(pty) = self.terminals.get_mut(&pane) {
            let _ = pty.write_input(cmd.as_bytes());
        }
    }

    /// Restart LazyGit PTY in current directory
    pub(super) fn restart_lazygit_in_current_dir(&mut self) {
        let cwd = self.file_browser.current_dir.clone();
        // Use default size, will be resized on first draw
        let rows = 24;
        let cols = 80;

        // Get lazygit command from config
        let lazygit_cmd = if self.config.pty.lazygit_command.is_empty() {
            vec!["lazygit".to_string()]
        } else {
            self.config.pty.lazygit_command.clone()
        };

        // Remove old PTY
        self.terminals.remove(&PaneId::LazyGit);

        // Create new PTY in current directory
        if let Ok(pty) = PseudoTerminal::new(&lazygit_cmd, rows, cols, &cwd) {
            self.terminals.insert(PaneId::LazyGit, pty);
        }
    }

    pub(super) fn check_and_restart_exited_ptys(&mut self) {
        // Skip if auto-restart is disabled
        if !self.config.pty.auto_restart {
            return;
        }

        let cwd = self.file_browser.current_dir.clone();
        let rows = 24;
        let cols = 80;

        // Check each terminal PTY
        let panes_to_restart: Vec<PaneId> = self
            .terminals
            .iter()
            .filter(|(_, pty)| pty.has_exited())
            .map(|(id, _)| *id)
            .collect();

        for pane_id in panes_to_restart {
            // Remove the old PTY
            self.terminals.remove(&pane_id);

            // Determine the command to restart based on pane type
            let cmd = match pane_id {
                PaneId::Claude => {
                    if self.config.pty.claude_command.is_empty() {
                        let mut cmd = vec![self.config.terminal.shell_path.clone()];
                        cmd.extend(self.config.terminal.shell_args.clone());
                        cmd
                    } else {
                        self.config.pty.claude_command.clone()
                    }
                }
                PaneId::LazyGit => {
                    if self.config.pty.lazygit_command.is_empty() {
                        vec!["lazygit".to_string()]
                    } else {
                        self.config.pty.lazygit_command.clone()
                    }
                }
                PaneId::Terminal => {
                    let mut cmd = vec![self.config.terminal.shell_path.clone()];
                    cmd.extend(self.config.terminal.shell_args.clone());
                    cmd
                }
                _ => continue, // Skip non-terminal panes
            };

            // Start a fresh shell/process
            if let Ok(new_pty) = PseudoTerminal::new(&cmd, rows, cols, &cwd) {
                self.terminals.insert(pane_id, new_pty);
            }
        }
    }

    /// Restart a single PTY (manual restart when auto_restart is disabled)
    pub(super) fn restart_single_pty(&mut self, pane_id: PaneId) {
        let cwd = self.file_browser.current_dir.clone();
        let rows = 24;
        let cols = 80;

        // Remove the old PTY
        self.terminals.remove(&pane_id);

        // Determine the command to restart based on pane type
        let cmd = match pane_id {
            PaneId::Claude => {
                if self.config.pty.claude_command.is_empty() {
                    let mut cmd = vec![self.config.terminal.shell_path.clone()];
                    cmd.extend(self.config.terminal.shell_args.clone());
                    cmd
                } else {
                    self.config.pty.claude_command.clone()
                }
            }
            PaneId::LazyGit => {
                if self.config.pty.lazygit_command.is_empty() {
                    vec!["lazygit".to_string()]
                } else {
                    self.config.pty.lazygit_command.clone()
                }
            }
            PaneId::Terminal => {
                let mut cmd = vec![self.config.terminal.shell_path.clone()];
                cmd.extend(self.config.terminal.shell_args.clone());
                cmd
            }
            _ => return, // Skip non-terminal panes
        };

        // Start a fresh shell/process
        if let Ok(new_pty) = PseudoTerminal::new(&cmd, rows, cols, &cwd) {
            self.terminals.insert(pane_id, new_pty);
        }
    }

    /// Insert file path at cursor in target terminal pane
    pub(super) fn insert_path_at_cursor(&mut self, target: PaneId, path: &Path) {
        if let Some(pty) = self.terminals.get_mut(&target) {
            let path_str = path.to_string_lossy();
            // Use shell-escape crate for proper escaping of special characters
            let escaped = escape(Cow::Borrowed(&path_str));

            // Write to PTY (no newline - just insert the path)
            let _ = pty.write_input(escaped.as_bytes());
        }
    }
}
