//! Installation wizard state and logic

use crate::config::Config;
use crate::setup::dependency_checker::DependencyReport;

/// Wizard step enumeration. The `SshImagePaste` step is conditionally shown
/// only when `crate::clipboard::is_ssh_session()` reports true — see
/// `WizardState::next_step` / `WizardState::prev_step` which skip it when
/// not relevant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WizardStep {
    #[default]
    Welcome,
    Dependencies,
    ShellSelection,
    ClaudeConfig,
    SshImagePaste,
    Confirmation,
    Complete,
}

impl WizardStep {
    pub fn next(&self) -> Self {
        match self {
            WizardStep::Welcome => WizardStep::Dependencies,
            WizardStep::Dependencies => WizardStep::ShellSelection,
            WizardStep::ShellSelection => WizardStep::ClaudeConfig,
            WizardStep::ClaudeConfig => WizardStep::SshImagePaste,
            WizardStep::SshImagePaste => WizardStep::Confirmation,
            WizardStep::Confirmation => WizardStep::Complete,
            WizardStep::Complete => WizardStep::Complete,
        }
    }

    pub fn prev(&self) -> Self {
        match self {
            WizardStep::Welcome => WizardStep::Welcome,
            WizardStep::Dependencies => WizardStep::Welcome,
            WizardStep::ShellSelection => WizardStep::Dependencies,
            WizardStep::ClaudeConfig => WizardStep::ShellSelection,
            WizardStep::SshImagePaste => WizardStep::ClaudeConfig,
            WizardStep::Confirmation => WizardStep::SshImagePaste,
            WizardStep::Complete => WizardStep::Confirmation,
        }
    }

    pub fn title(&self) -> &'static str {
        match self {
            WizardStep::Welcome => "Welcome",
            WizardStep::Dependencies => "Dependency Check",
            WizardStep::ShellSelection => "Shell Selection",
            WizardStep::ClaudeConfig => "Tool Configuration",
            WizardStep::SshImagePaste => "SSH Image Paste",
            WizardStep::Confirmation => "Confirmation",
            WizardStep::Complete => "Complete",
        }
    }
}

/// Which field is being edited
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WizardField {
    ClaudePath,
    LazygitPath,
    ShellPath,
}

/// Wizard runtime state
#[derive(Debug, Clone)]
pub struct WizardState {
    pub visible: bool,
    pub step: WizardStep,
    pub deps: DependencyReport,

    // User selections
    pub selected_shell_idx: usize,
    pub claude_path: String,
    pub lazygit_path: String,

    // Available options
    pub available_shells: Vec<String>,

    // Input state for path editing
    pub editing_field: Option<WizardField>,
    pub input_buffer: String,

    // Current field focus (for ClaudeConfig step)
    pub focused_field: usize,

    /// Detected `cc-clip` binary on the remote host (`None` = not on PATH).
    /// Populated lazily when the wizard enters the `SshImagePaste` step so
    /// the user sees a green check or a yellow "not found" line.
    pub cc_clip_path: Option<std::path::PathBuf>,
    /// User pressed "Mark as configured" on the SSH step → persisted as
    /// `config.ssh.notification_dismissed = true` by `generate_config()`.
    pub ssh_image_paste_marked_configured: bool,
}

impl Default for WizardState {
    fn default() -> Self {
        let deps = DependencyReport::check();

        let available_shells: Vec<String> = deps
            .shells
            .iter()
            .map(|s| {
                s.path
                    .as_ref()
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_else(|| format!("/bin/{}", s.name))
            })
            .collect();

        // Default to first available shell, or /bin/bash
        let default_shell = available_shells
            .first()
            .cloned()
            .unwrap_or_else(|| "/bin/bash".to_string());

        Self {
            visible: false,
            step: WizardStep::Welcome,
            deps: deps.clone(),
            selected_shell_idx: 0,
            claude_path: deps
                .claude_cli
                .path
                .as_ref()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|| "claude".to_string()),
            lazygit_path: deps
                .lazygit
                .path
                .as_ref()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|| "lazygit".to_string()),
            available_shells: if available_shells.is_empty() {
                vec![default_shell]
            } else {
                available_shells
            },
            editing_field: None,
            input_buffer: String::new(),
            focused_field: 0,
            cc_clip_path: crate::clipboard::which("cc-clip"),
            ssh_image_paste_marked_configured: false,
        }
    }
}

impl WizardState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn open(&mut self) {
        *self = Self::default();
        self.visible = true;
    }

    pub fn close(&mut self) {
        self.visible = false;
    }

    pub fn next_step(&mut self) {
        self.step = self.step.next();
        self.skip_inactive_step_forward();
    }

    pub fn prev_step(&mut self) {
        self.step = self.step.prev();
        self.skip_inactive_step_backward();
    }

    /// Total visible steps for the current session. The SSH step is hidden
    /// when not in an SSH session, leaving 5 steps; otherwise 6.
    pub fn total_steps(&self) -> u8 {
        if crate::clipboard::is_ssh_session() {
            6
        } else {
            5
        }
    }

    /// Position of the current step in the visible sequence. Used for the
    /// "Step N/M" header — keeps numbering contiguous when the SSH step
    /// is hidden.
    pub fn current_step_number(&self) -> u8 {
        let in_ssh = crate::clipboard::is_ssh_session();
        match self.step {
            WizardStep::Welcome => 1,
            WizardStep::Dependencies => 2,
            WizardStep::ShellSelection => 3,
            WizardStep::ClaudeConfig => 4,
            WizardStep::SshImagePaste => 5, // only reachable when in_ssh == true
            WizardStep::Confirmation => {
                if in_ssh {
                    6
                } else {
                    5
                }
            }
            WizardStep::Complete => {
                if in_ssh {
                    6
                } else {
                    5
                }
            }
        }
    }

    /// If the current step is `SshImagePaste` but not in an SSH session,
    /// advance once more so the user does not see an irrelevant step.
    fn skip_inactive_step_forward(&mut self) {
        if matches!(self.step, WizardStep::SshImagePaste) && !crate::clipboard::is_ssh_session() {
            self.step = self.step.next();
        }
    }

    /// Mirror of `skip_inactive_step_forward` for backward navigation.
    fn skip_inactive_step_backward(&mut self) {
        if matches!(self.step, WizardStep::SshImagePaste) && !crate::clipboard::is_ssh_session() {
            self.step = self.step.prev();
        }
    }

    pub fn start_editing(&mut self, field: WizardField) {
        self.editing_field = Some(field);
        self.input_buffer = match field {
            WizardField::ClaudePath => self.claude_path.clone(),
            WizardField::LazygitPath => self.lazygit_path.clone(),
            WizardField::ShellPath => self
                .available_shells
                .get(self.selected_shell_idx)
                .cloned()
                .unwrap_or_default(),
        };
    }

    pub fn finish_editing(&mut self) {
        if let Some(field) = self.editing_field.take() {
            let value = self.input_buffer.clone();
            match field {
                WizardField::ClaudePath => self.claude_path = value,
                WizardField::LazygitPath => self.lazygit_path = value,
                WizardField::ShellPath => {
                    // Add custom shell to list if not present
                    if !self.available_shells.contains(&value) {
                        self.available_shells.push(value.clone());
                    }
                    self.selected_shell_idx = self
                        .available_shells
                        .iter()
                        .position(|s| s == &value)
                        .unwrap_or(0);
                }
            }
            self.input_buffer.clear();
        }
    }

    pub fn cancel_editing(&mut self) {
        self.editing_field = None;
        self.input_buffer.clear();
    }

    /// Get the selected shell path
    pub fn selected_shell(&self) -> String {
        self.available_shells
            .get(self.selected_shell_idx)
            .cloned()
            .unwrap_or_else(|| "/bin/bash".to_string())
    }

    /// Generate final config from wizard selections
    pub fn generate_config(&self) -> Config {
        let mut config = Config::default();

        // Apply wizard selections
        config.terminal.shell_path = self.selected_shell();
        config.pty.claude_command = vec![self.claude_path.clone()];
        config.pty.lazygit_command = vec![self.lazygit_path.clone()];

        // SSH-image-paste: persist detected helper path and dismissed flag
        // so the runtime check in `handle_terminal_pane_key` skips the hint
        // for users who already configured cc-clip.
        if let Some(p) = &self.cc_clip_path {
            config.ssh.image_paste_helper = Some(p.to_string_lossy().to_string());
        }
        if self.ssh_image_paste_marked_configured {
            config.ssh.notification_dismissed = true;
        }

        // Mark wizard as complete
        config.setup.wizard_completed = true;
        config.setup.wizard_version = 1;

        config
    }

    /// Check if can proceed to next step
    pub fn can_proceed(&self) -> bool {
        match self.step {
            WizardStep::Welcome => true,
            WizardStep::Dependencies => self.deps.all_required_met(),
            WizardStep::ShellSelection => !self.available_shells.is_empty(),
            WizardStep::ClaudeConfig => true, // Paths can be defaults
            WizardStep::SshImagePaste => true, // informational, always proceed
            WizardStep::Confirmation => true,
            WizardStep::Complete => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wizard_step_navigation() {
        let mut step = WizardStep::Welcome;
        step = step.next();
        assert_eq!(step, WizardStep::Dependencies);
        step = step.prev();
        assert_eq!(step, WizardStep::Welcome);
    }

    #[test]
    fn test_wizard_state_default() {
        let state = WizardState::default();
        assert!(!state.visible);
        assert_eq!(state.step, WizardStep::Welcome);
    }

    #[test]
    fn test_generate_config() {
        let mut state = WizardState::default();
        state.claude_path = "/usr/local/bin/claude".to_string();
        let config = state.generate_config();
        assert!(config.setup.wizard_completed);
        assert_eq!(config.pty.claude_command, vec!["/usr/local/bin/claude"]);
    }

    #[test]
    fn test_ssh_step_in_chain() {
        // Verify the static next/prev sequence includes SshImagePaste.
        let mut step = WizardStep::ClaudeConfig;
        step = step.next();
        assert_eq!(step, WizardStep::SshImagePaste);
        step = step.next();
        assert_eq!(step, WizardStep::Confirmation);
        step = step.prev();
        assert_eq!(step, WizardStep::SshImagePaste);
    }

    #[test]
    fn test_mark_as_configured_persisted_to_config() {
        let mut state = WizardState::default();
        state.ssh_image_paste_marked_configured = true;
        let config = state.generate_config();
        assert!(config.ssh.notification_dismissed);
    }

    #[test]
    fn test_default_ssh_marked_not_persisted() {
        let state = WizardState::default();
        let config = state.generate_config();
        assert!(!config.ssh.notification_dismissed);
    }
}
