//! Installation wizard state and logic

use crate::config::Config;
use crate::setup::dependency_checker::DependencyReport;
use crate::setup::templates::{Template, get_builtin_templates};

/// Wizard step enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WizardStep {
    #[default]
    Welcome,
    Dependencies,
    ShellSelection,
    ClaudeConfig,
    TemplateSelection,
    Confirmation,
    Complete,
}

impl WizardStep {
    pub fn next(&self) -> Self {
        match self {
            WizardStep::Welcome => WizardStep::Dependencies,
            WizardStep::Dependencies => WizardStep::ShellSelection,
            WizardStep::ShellSelection => WizardStep::ClaudeConfig,
            WizardStep::ClaudeConfig => WizardStep::TemplateSelection,
            WizardStep::TemplateSelection => WizardStep::Confirmation,
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
            WizardStep::TemplateSelection => WizardStep::ClaudeConfig,
            WizardStep::Confirmation => WizardStep::TemplateSelection,
            WizardStep::Complete => WizardStep::Confirmation,
        }
    }

    pub fn step_number(&self) -> u8 {
        match self {
            WizardStep::Welcome => 1,
            WizardStep::Dependencies => 2,
            WizardStep::ShellSelection => 3,
            WizardStep::ClaudeConfig => 4,
            WizardStep::TemplateSelection => 5,
            WizardStep::Confirmation => 6,
            WizardStep::Complete => 6,
        }
    }

    pub fn total_steps() -> u8 {
        6
    }

    pub fn title(&self) -> &'static str {
        match self {
            WizardStep::Welcome => "Welcome",
            WizardStep::Dependencies => "Dependency Check",
            WizardStep::ShellSelection => "Shell Selection",
            WizardStep::ClaudeConfig => "Tool Configuration",
            WizardStep::TemplateSelection => "Choose Template",
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
    pub selected_template_idx: usize,

    // Available options
    pub available_shells: Vec<String>,
    pub available_templates: Vec<Template>,

    // Input state for path editing
    pub editing_field: Option<WizardField>,
    pub input_buffer: String,

    // Current field focus (for ClaudeConfig step)
    pub focused_field: usize,
}

impl Default for WizardState {
    fn default() -> Self {
        let deps = DependencyReport::check();
        let templates = get_builtin_templates();

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
        let default_shell = available_shells.first().cloned().unwrap_or_else(|| "/bin/bash".to_string());

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
            selected_template_idx: 0,
            available_shells: if available_shells.is_empty() {
                vec![default_shell]
            } else {
                available_shells
            },
            available_templates: templates
                .into_iter()
                .filter(|t| t.category == "workflow")
                .collect(),
            editing_field: None,
            input_buffer: String::new(),
            focused_field: 0,
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
    }

    pub fn prev_step(&mut self) {
        self.step = self.step.prev();
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

    /// Get the selected template
    pub fn selected_template(&self) -> Option<&Template> {
        self.available_templates.get(self.selected_template_idx)
    }

    /// Generate final config from wizard selections
    pub fn generate_config(&self) -> Config {
        let mut config = Config::default();

        // Apply template first (provides base layout/pty settings)
        if let Some(template) = self.selected_template() {
            crate::setup::templates::apply_template(&mut config, template);
        }

        // Then apply wizard-specific selections (overrides template)
        config.terminal.shell_path = self.selected_shell();
        config.pty.claude_command = vec![self.claude_path.clone()];
        config.pty.lazygit_command = vec![self.lazygit_path.clone()];

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
            WizardStep::TemplateSelection => true,
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
}
