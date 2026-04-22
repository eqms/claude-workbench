use std::borrow::Cow;
use std::path::Path;

use shell_escape::escape;

use crate::config::Config;
use crate::terminal::PseudoTerminal;
use crate::types::{ClaudeEffort, ClaudeModel, ClaudePermissionMode, PaneId};

use super::App;

/// Bundled Claude Code startup options, assembled from dialog state + config.
#[derive(Debug, Clone, Default)]
pub(crate) struct StartupOptions {
    pub permission_mode: ClaudePermissionMode,
    pub model: ClaudeModel,
    pub effort: ClaudeEffort,
    pub session_name: String,
    pub worktree: String,
    pub remote_control: bool,
}

impl App {
    pub(super) fn build_claude_command(config: &Config, opts: &StartupOptions) -> Vec<String> {
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
            // Permission mode
            if opts.permission_mode.is_yolo() {
                if !cmd
                    .iter()
                    .any(|a| a.contains("--dangerously-skip-permissions"))
                {
                    cmd.push("--dangerously-skip-permissions".to_string());
                }
            } else if let Some(flag_value) = opts.permission_mode.cli_flag() {
                if !cmd.iter().any(|a| a.contains("--permission-mode")) {
                    cmd.push("--permission-mode".to_string());
                    cmd.push(flag_value.to_string());
                }
            }

            // Model
            if let Some(model) = opts.model.cli_flag() {
                if !cmd.iter().any(|a| a == "--model") {
                    cmd.push("--model".to_string());
                    cmd.push(model.to_string());
                }
            }

            // Effort
            if let Some(effort) = opts.effort.cli_flag() {
                if !cmd.iter().any(|a| a == "--effort") {
                    cmd.push("--effort".to_string());
                    cmd.push(effort.to_string());
                }
            }

            // Session name (--name)
            if !opts.session_name.is_empty() && !cmd.iter().any(|a| a == "--name") {
                cmd.push("--name".to_string());
                cmd.push(opts.session_name.clone());
            }

            // Worktree (--worktree)
            if !opts.worktree.is_empty() && !cmd.iter().any(|a| a == "--worktree") {
                cmd.push("--worktree".to_string());
                cmd.push(opts.worktree.clone());
            }

            // Remote control flag (replaces former slash-command hack)
            if opts.remote_control && !cmd.iter().any(|a| a == "--remote-control" || a == "--rc") {
                cmd.push("--remote-control".to_string());
            }
        }

        cmd
    }

    /// Initialize Claude PTY with the given startup options
    pub(super) fn init_claude_pty(&mut self, opts: StartupOptions) {
        self.claude_permission_mode = opts.permission_mode;
        self.claude_pty_pending = false;

        let claude_cmd = Self::build_claude_command(&self.config, &opts);
        self.claude_command_used = claude_cmd.join(" ");

        let cwd = self.file_browser.current_dir.clone();
        let rows = 24;
        let cols = 80;

        match PseudoTerminal::new(&claude_cmd, rows, cols, &cwd) {
            Ok(pty) => {
                self.terminals.insert(PaneId::Claude, pty);
                self.claude_error = None;
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
            self.permission_mode_dialog.open_with_defaults(
                self.config.claude.default_permission_mode,
                self.config.claude.default_model,
                self.config.claude.default_effort,
                &self.config.claude.default_session_name,
                &self.config.claude.default_worktree,
                self.config.claude.remote_control,
            );
        } else {
            let opts = StartupOptions {
                permission_mode: self
                    .config
                    .claude
                    .default_permission_mode
                    .unwrap_or(ClaudePermissionMode::Default),
                model: self.config.claude.default_model,
                effort: self.config.claude.default_effort,
                session_name: self.config.claude.default_session_name.clone(),
                worktree: self.config.claude.default_worktree.clone(),
                remote_control: self.config.claude.remote_control,
            };
            self.init_claude_pty(opts);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;

    fn config_with_claude_command() -> Config {
        let mut cfg = Config::default();
        cfg.pty.claude_command = vec!["claude".to_string()];
        cfg
    }

    fn base_opts() -> StartupOptions {
        StartupOptions {
            permission_mode: ClaudePermissionMode::Default,
            model: ClaudeModel::Unset,
            effort: ClaudeEffort::Unset,
            session_name: String::new(),
            worktree: String::new(),
            remote_control: false,
        }
    }

    #[test]
    fn test_build_command_shell_fallback_adds_no_flags() {
        // When claude_command is empty, no flags should be appended (shell path)
        let cfg = Config::default();
        let mut opts = base_opts();
        opts.permission_mode = ClaudePermissionMode::Auto;
        opts.model = ClaudeModel::Sonnet;
        let cmd = App::build_claude_command(&cfg, &opts);
        assert!(!cmd.iter().any(|a| a == "--permission-mode"));
        assert!(!cmd.iter().any(|a| a == "--model"));
    }

    #[test]
    fn test_build_command_auto_mode() {
        let cfg = config_with_claude_command();
        let mut opts = base_opts();
        opts.permission_mode = ClaudePermissionMode::Auto;
        let cmd = App::build_claude_command(&cfg, &opts);
        assert!(cmd.contains(&"--permission-mode".to_string()));
        assert!(cmd.contains(&"auto".to_string()));
    }

    #[test]
    fn test_build_command_with_model() {
        let cfg = config_with_claude_command();
        let mut opts = base_opts();
        opts.model = ClaudeModel::Sonnet;
        let cmd = App::build_claude_command(&cfg, &opts);
        assert!(cmd.contains(&"--model".to_string()));
        assert!(cmd.contains(&"sonnet".to_string()));
    }

    #[test]
    fn test_build_command_with_effort() {
        let cfg = config_with_claude_command();
        let mut opts = base_opts();
        opts.effort = ClaudeEffort::High;
        let cmd = App::build_claude_command(&cfg, &opts);
        assert!(cmd.contains(&"--effort".to_string()));
        assert!(cmd.contains(&"high".to_string()));
    }

    #[test]
    fn test_build_command_with_session_name() {
        let cfg = config_with_claude_command();
        let mut opts = base_opts();
        opts.session_name = "test-session".to_string();
        let cmd = App::build_claude_command(&cfg, &opts);
        assert!(cmd.contains(&"--name".to_string()));
        assert!(cmd.contains(&"test-session".to_string()));
    }

    #[test]
    fn test_build_command_with_worktree() {
        let cfg = config_with_claude_command();
        let mut opts = base_opts();
        opts.worktree = "feature-x".to_string();
        let cmd = App::build_claude_command(&cfg, &opts);
        assert!(cmd.contains(&"--worktree".to_string()));
        assert!(cmd.contains(&"feature-x".to_string()));
    }

    #[test]
    fn test_build_command_remote_control() {
        let cfg = config_with_claude_command();
        let mut opts = base_opts();
        opts.remote_control = true;
        let cmd = App::build_claude_command(&cfg, &opts);
        assert!(cmd.contains(&"--remote-control".to_string()));
    }

    #[test]
    fn test_build_command_yolo_mode_uses_dangerously_skip() {
        let cfg = config_with_claude_command();
        let mut opts = base_opts();
        opts.permission_mode = ClaudePermissionMode::DangerouslySkipPermissions;
        let cmd = App::build_claude_command(&cfg, &opts);
        assert!(cmd.contains(&"--dangerously-skip-permissions".to_string()));
        assert!(!cmd.iter().any(|a| a == "--permission-mode"));
    }

    #[test]
    fn test_build_command_empty_values_do_not_emit_flags() {
        let cfg = config_with_claude_command();
        let opts = base_opts(); // all empty/Unset/false
        let cmd = App::build_claude_command(&cfg, &opts);
        assert!(!cmd.iter().any(|a| a == "--model"));
        assert!(!cmd.iter().any(|a| a == "--effort"));
        assert!(!cmd.iter().any(|a| a == "--name"));
        assert!(!cmd.iter().any(|a| a == "--worktree"));
        assert!(!cmd.iter().any(|a| a == "--remote-control"));
    }

    #[test]
    fn test_build_command_all_flags_combined() {
        let cfg = config_with_claude_command();
        let opts = StartupOptions {
            permission_mode: ClaudePermissionMode::Auto,
            model: ClaudeModel::Opus,
            effort: ClaudeEffort::Max,
            session_name: "session1".to_string(),
            worktree: "feat".to_string(),
            remote_control: true,
        };
        let cmd = App::build_claude_command(&cfg, &opts);
        assert_eq!(cmd[0], "claude");
        assert!(cmd.contains(&"--permission-mode".to_string()));
        assert!(cmd.contains(&"auto".to_string()));
        assert!(cmd.contains(&"--model".to_string()));
        assert!(cmd.contains(&"opus".to_string()));
        assert!(cmd.contains(&"--effort".to_string()));
        assert!(cmd.contains(&"max".to_string()));
        assert!(cmd.contains(&"--name".to_string()));
        assert!(cmd.contains(&"session1".to_string()));
        assert!(cmd.contains(&"--worktree".to_string()));
        assert!(cmd.contains(&"feat".to_string()));
        assert!(cmd.contains(&"--remote-control".to_string()));
    }
}
