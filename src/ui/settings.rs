//! Settings menu UI and state

use crate::config::Config;
use crate::setup::templates::{get_builtin_templates, Template};
use ratatui::{
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
    Frame,
};

/// Settings categories
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SettingsCategory {
    #[default]
    General,
    Layout,
    Paths,
    Templates,
    About,
}

impl SettingsCategory {
    pub fn all() -> &'static [SettingsCategory] {
        &[
            SettingsCategory::General,
            SettingsCategory::Layout,
            SettingsCategory::Paths,
            SettingsCategory::Templates,
            SettingsCategory::About,
        ]
    }

    pub fn title(&self) -> &'static str {
        match self {
            SettingsCategory::General => "General",
            SettingsCategory::Layout => "Layout",
            SettingsCategory::Paths => "Paths",
            SettingsCategory::Templates => "Templates",
            SettingsCategory::About => "About",
        }
    }

    pub fn next(&self) -> Self {
        match self {
            SettingsCategory::General => SettingsCategory::Layout,
            SettingsCategory::Layout => SettingsCategory::Paths,
            SettingsCategory::Paths => SettingsCategory::Templates,
            SettingsCategory::Templates => SettingsCategory::About,
            SettingsCategory::About => SettingsCategory::General,
        }
    }

    pub fn prev(&self) -> Self {
        match self {
            SettingsCategory::General => SettingsCategory::About,
            SettingsCategory::Layout => SettingsCategory::General,
            SettingsCategory::Paths => SettingsCategory::Layout,
            SettingsCategory::Templates => SettingsCategory::Paths,
            SettingsCategory::About => SettingsCategory::Templates,
        }
    }
}

/// Which field is being edited in settings
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SettingsField {
    // General
    ShellPath,
    ScrollbackLines,
    ShowHiddenFiles,
    Autosave,
    AutoRefreshMs,
    CheckForUpdates, // Action item, not editable
    // Layout
    FileBrowserWidth,
    PreviewWidth,
    RightPanelWidth,
    ClaudeHeight,
    // Paths
    ClaudePath,
    LazygitPath,
}

/// Settings menu state
#[derive(Debug, Clone)]
pub struct SettingsState {
    pub visible: bool,
    pub category: SettingsCategory,
    pub selected_idx: usize,
    pub editing: Option<SettingsField>,
    pub input_buffer: String,

    // Cached config values (editable copies)
    pub shell_path: String,
    pub scrollback_lines: usize,
    pub show_hidden_files: bool,
    pub autosave: bool,
    pub auto_refresh_ms: u64,
    pub file_browser_width: u16,
    pub preview_width: u16,
    pub right_panel_width: u16,
    pub claude_height: u16,
    pub claude_path: String,
    pub lazygit_path: String,
    pub selected_template_idx: usize,
    pub available_templates: Vec<Template>,

    // Track if changes were made
    pub has_changes: bool,
}

impl Default for SettingsState {
    fn default() -> Self {
        let templates = get_builtin_templates();
        Self {
            visible: false,
            category: SettingsCategory::General,
            selected_idx: 0,
            editing: None,
            input_buffer: String::new(),
            shell_path: "/bin/bash".to_string(),
            scrollback_lines: 1000,
            show_hidden_files: true, // Show hidden files by default
            autosave: false,
            auto_refresh_ms: 2000,
            file_browser_width: 20,
            preview_width: 50,
            right_panel_width: 30,
            claude_height: 40,
            claude_path: "claude".to_string(),
            lazygit_path: "lazygit".to_string(),
            selected_template_idx: 0,
            available_templates: templates,
            has_changes: false,
        }
    }
}

impl SettingsState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Load settings from config
    pub fn load_from_config(&mut self, config: &Config) {
        self.shell_path = config.terminal.shell_path.clone();
        self.scrollback_lines = config.pty.scrollback_lines;
        self.show_hidden_files = config.file_browser.show_hidden;
        self.autosave = config.ui.autosave;
        self.auto_refresh_ms = config.file_browser.auto_refresh_ms;
        self.file_browser_width = config.layout.file_browser_width_percent;
        self.preview_width = config.layout.preview_width_percent;
        self.right_panel_width = config.layout.right_panel_width_percent;
        self.claude_height = config.layout.claude_height_percent;
        self.claude_path = config
            .pty
            .claude_command
            .first()
            .cloned()
            .unwrap_or_else(|| "claude".to_string());
        self.lazygit_path = config
            .pty
            .lazygit_command
            .first()
            .cloned()
            .unwrap_or_else(|| "lazygit".to_string());
        self.has_changes = false;
    }

    /// Apply settings to config
    pub fn apply_to_config(&self, config: &mut Config) {
        config.terminal.shell_path = self.shell_path.clone();
        config.pty.scrollback_lines = self.scrollback_lines;
        config.file_browser.show_hidden = self.show_hidden_files;
        config.ui.autosave = self.autosave;
        config.file_browser.auto_refresh_ms = self.auto_refresh_ms;
        config.layout.file_browser_width_percent = self.file_browser_width;
        config.layout.preview_width_percent = self.preview_width;
        config.layout.right_panel_width_percent = self.right_panel_width;
        config.layout.claude_height_percent = self.claude_height;
        config.pty.claude_command = vec![self.claude_path.clone()];
        config.pty.lazygit_command = vec![self.lazygit_path.clone()];
    }

    pub fn open(&mut self, config: &Config) {
        self.load_from_config(config);
        self.visible = true;
        self.category = SettingsCategory::General;
        self.selected_idx = 0;
        self.editing = None;
    }

    pub fn close(&mut self) {
        self.visible = false;
        self.editing = None;
        self.input_buffer.clear();
    }

    pub fn next_category(&mut self) {
        self.category = self.category.next();
        self.selected_idx = 0;
    }

    pub fn prev_category(&mut self) {
        self.category = self.category.prev();
        self.selected_idx = 0;
    }

    pub fn item_count(&self) -> usize {
        match self.category {
            SettingsCategory::General => 6, // shell, scrollback, hidden, autosave, auto-refresh, check updates
            SettingsCategory::Layout => 4,  // file_browser, preview, right_panel, claude_height
            SettingsCategory::Paths => 2,   // claude, lazygit
            SettingsCategory::Templates => self.available_templates.len(),
            SettingsCategory::About => 0,
        }
    }

    /// Check if the currently selected item is the "Check for Updates" action
    pub fn is_check_updates_selected(&self) -> bool {
        self.category == SettingsCategory::General && self.selected_idx == 5
    }

    pub fn move_up(&mut self) {
        if self.selected_idx > 0 {
            self.selected_idx -= 1;
        }
    }

    pub fn move_down(&mut self) {
        let max = self.item_count().saturating_sub(1);
        if self.selected_idx < max {
            self.selected_idx += 1;
        }
    }

    /// Start editing the currently selected field
    pub fn start_editing(&mut self) {
        let field = match self.category {
            SettingsCategory::General => match self.selected_idx {
                0 => Some(SettingsField::ShellPath),
                1 => Some(SettingsField::ScrollbackLines),
                2 => Some(SettingsField::ShowHiddenFiles),
                3 => Some(SettingsField::Autosave),
                4 => Some(SettingsField::AutoRefreshMs),
                _ => None,
            },
            SettingsCategory::Layout => match self.selected_idx {
                0 => Some(SettingsField::FileBrowserWidth),
                1 => Some(SettingsField::PreviewWidth),
                2 => Some(SettingsField::RightPanelWidth),
                3 => Some(SettingsField::ClaudeHeight),
                _ => None,
            },
            SettingsCategory::Paths => match self.selected_idx {
                0 => Some(SettingsField::ClaudePath),
                1 => Some(SettingsField::LazygitPath),
                _ => None,
            },
            _ => None,
        };

        if let Some(f) = field {
            self.input_buffer = match &f {
                SettingsField::ShellPath => self.shell_path.clone(),
                SettingsField::ScrollbackLines => self.scrollback_lines.to_string(),
                SettingsField::ShowHiddenFiles => self.show_hidden_files.to_string(),
                SettingsField::Autosave => self.autosave.to_string(),
                SettingsField::AutoRefreshMs => self.auto_refresh_ms.to_string(),
                SettingsField::FileBrowserWidth => self.file_browser_width.to_string(),
                SettingsField::PreviewWidth => self.preview_width.to_string(),
                SettingsField::RightPanelWidth => self.right_panel_width.to_string(),
                SettingsField::ClaudeHeight => self.claude_height.to_string(),
                SettingsField::ClaudePath => self.claude_path.clone(),
                SettingsField::LazygitPath => self.lazygit_path.clone(),
                SettingsField::CheckForUpdates => {
                    unreachable!("CheckForUpdates is an action, not a field")
                }
            };
            self.editing = Some(f);
        }
    }

    /// Toggle boolean field or select template
    pub fn toggle_or_select(&mut self) {
        match self.category {
            SettingsCategory::General => match self.selected_idx {
                2 => {
                    self.show_hidden_files = !self.show_hidden_files;
                    self.has_changes = true;
                }
                3 => {
                    self.autosave = !self.autosave;
                    self.has_changes = true;
                }
                _ => self.start_editing(),
            },
            SettingsCategory::Templates => {
                self.selected_template_idx = self.selected_idx;
                self.has_changes = true;
            }
            _ => self.start_editing(),
        }
    }

    /// Finish editing and apply value
    pub fn finish_editing(&mut self) {
        if let Some(field) = self.editing.take() {
            let value = self.input_buffer.trim().to_string();
            match field {
                SettingsField::ShellPath => self.shell_path = value,
                SettingsField::ScrollbackLines => {
                    if let Ok(v) = value.parse::<usize>() {
                        self.scrollback_lines = v.clamp(100, 100000);
                    }
                }
                SettingsField::ShowHiddenFiles => {
                    self.show_hidden_files = value.to_lowercase() == "true";
                }
                SettingsField::Autosave => {
                    self.autosave = value.to_lowercase() == "true";
                }
                SettingsField::AutoRefreshMs => {
                    if let Ok(v) = value.parse::<u64>() {
                        self.auto_refresh_ms = v.clamp(0, 60000);
                    }
                }
                SettingsField::FileBrowserWidth => {
                    if let Ok(v) = value.parse::<u16>() {
                        self.file_browser_width = v.clamp(10, 50);
                    }
                }
                SettingsField::PreviewWidth => {
                    if let Ok(v) = value.parse::<u16>() {
                        self.preview_width = v.clamp(20, 80);
                    }
                }
                SettingsField::RightPanelWidth => {
                    if let Ok(v) = value.parse::<u16>() {
                        self.right_panel_width = v.clamp(0, 50);
                    }
                }
                SettingsField::ClaudeHeight => {
                    if let Ok(v) = value.parse::<u16>() {
                        self.claude_height = v.clamp(20, 80);
                    }
                }
                SettingsField::ClaudePath => self.claude_path = value,
                SettingsField::LazygitPath => self.lazygit_path = value,
                SettingsField::CheckForUpdates => {} // Action, not a field to edit
            }
            self.has_changes = true;
            self.input_buffer.clear();
        }
    }

    pub fn cancel_editing(&mut self) {
        self.editing = None;
        self.input_buffer.clear();
    }

    /// Get selected template (if any)
    pub fn selected_template(&self) -> Option<&Template> {
        self.available_templates.get(self.selected_template_idx)
    }
}

/// Render the settings menu
pub fn render(frame: &mut Frame, area: Rect, state: &SettingsState) {
    // Create centered popup (70% width, 80% height)
    let popup_width = (area.width as f32 * 0.7) as u16;
    let popup_height = (area.height as f32 * 0.8) as u16;
    let popup_x = (area.width.saturating_sub(popup_width)) / 2;
    let popup_y = (area.height.saturating_sub(popup_height)) / 2;

    let popup_area = Rect::new(popup_x, popup_y, popup_width, popup_height);

    // Clear the popup area
    frame.render_widget(Clear, popup_area);

    // Main container
    let block = Block::default()
        .title(" ⚙ Settings ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));
    let inner = block.inner(popup_area);
    frame.render_widget(block, popup_area);

    // Layout: tabs on top, content below, footer at bottom
    let layout = Layout::vertical([
        Constraint::Length(3), // Category tabs
        Constraint::Min(1),    // Content
        Constraint::Length(3), // Footer
    ])
    .split(inner);

    render_category_tabs(frame, layout[0], state);
    render_category_content(frame, layout[1], state);
    render_footer(frame, layout[2], state);
}

fn render_category_tabs(frame: &mut Frame, area: Rect, state: &SettingsState) {
    let tabs: Vec<Span> = SettingsCategory::all()
        .iter()
        .map(|cat| {
            let style = if *cat == state.category {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            Span::styled(format!(" {} ", cat.title()), style)
        })
        .collect();

    let tabs_line = Line::from(tabs);
    let tabs_widget = Paragraph::new(tabs_line)
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::BOTTOM));
    frame.render_widget(tabs_widget, area);
}

fn render_category_content(frame: &mut Frame, area: Rect, state: &SettingsState) {
    match state.category {
        SettingsCategory::General => render_general(frame, area, state),
        SettingsCategory::Layout => render_layout(frame, area, state),
        SettingsCategory::Paths => render_paths(frame, area, state),
        SettingsCategory::Templates => render_templates(frame, area, state),
        SettingsCategory::About => render_about(frame, area),
    }
}

fn render_general(frame: &mut Frame, area: Rect, state: &SettingsState) {
    let auto_refresh_display = if state.auto_refresh_ms == 0 {
        "disabled".to_string()
    } else {
        format!("{}ms", state.auto_refresh_ms)
    };

    let items = vec![
        format_setting(
            "Shell Path",
            &state.shell_path,
            state.selected_idx == 0,
            state.editing.as_ref() == Some(&SettingsField::ShellPath),
            &state.input_buffer,
        ),
        format_setting(
            "Scrollback Lines",
            &state.scrollback_lines.to_string(),
            state.selected_idx == 1,
            state.editing.as_ref() == Some(&SettingsField::ScrollbackLines),
            &state.input_buffer,
        ),
        format_bool_setting(
            "Show Hidden Files",
            state.show_hidden_files,
            state.selected_idx == 2,
        ),
        format_bool_setting("Autosave", state.autosave, state.selected_idx == 3),
        format_setting(
            "Auto Refresh (ms)",
            &auto_refresh_display,
            state.selected_idx == 4,
            state.editing.as_ref() == Some(&SettingsField::AutoRefreshMs),
            &state.input_buffer,
        ),
        format_action_setting("Check for Updates", state.selected_idx == 5),
    ];

    let list = create_settings_list(items);
    frame.render_widget(list, area);
}

fn render_layout(frame: &mut Frame, area: Rect, state: &SettingsState) {
    let items = vec![
        format_setting(
            "File Browser Width %",
            &state.file_browser_width.to_string(),
            state.selected_idx == 0,
            state.editing.as_ref() == Some(&SettingsField::FileBrowserWidth),
            &state.input_buffer,
        ),
        format_setting(
            "Preview Width %",
            &state.preview_width.to_string(),
            state.selected_idx == 1,
            state.editing.as_ref() == Some(&SettingsField::PreviewWidth),
            &state.input_buffer,
        ),
        format_setting(
            "Right Panel Width %",
            &state.right_panel_width.to_string(),
            state.selected_idx == 2,
            state.editing.as_ref() == Some(&SettingsField::RightPanelWidth),
            &state.input_buffer,
        ),
        format_setting(
            "Claude Height %",
            &state.claude_height.to_string(),
            state.selected_idx == 3,
            state.editing.as_ref() == Some(&SettingsField::ClaudeHeight),
            &state.input_buffer,
        ),
    ];

    let list = create_settings_list(items);
    frame.render_widget(list, area);
}

fn render_paths(frame: &mut Frame, area: Rect, state: &SettingsState) {
    let items = vec![
        format_setting(
            "Claude CLI Path",
            &state.claude_path,
            state.selected_idx == 0,
            state.editing.as_ref() == Some(&SettingsField::ClaudePath),
            &state.input_buffer,
        ),
        format_setting(
            "LazyGit Path",
            &state.lazygit_path,
            state.selected_idx == 1,
            state.editing.as_ref() == Some(&SettingsField::LazygitPath),
            &state.input_buffer,
        ),
    ];

    let list = create_settings_list(items);
    frame.render_widget(list, area);
}

fn render_templates(frame: &mut Frame, area: Rect, state: &SettingsState) {
    let items: Vec<ListItem> = state
        .available_templates
        .iter()
        .enumerate()
        .map(|(i, template)| {
            let is_selected = i == state.selected_idx;
            let is_active = i == state.selected_template_idx;

            let marker = if is_active { "● " } else { "○ " };
            let style = if is_selected {
                Style::default().fg(Color::Black).bg(Color::Cyan)
            } else if is_active {
                Style::default().fg(Color::Green)
            } else {
                Style::default()
            };

            let text = format!("{}{} - {}", marker, template.name, template.description);
            ListItem::new(Line::from(text)).style(style)
        })
        .collect();

    let list = List::new(items).block(Block::default().borders(Borders::NONE));
    frame.render_widget(list, area);
}

fn render_about(frame: &mut Frame, area: Rect) {
    let text = vec![
        Line::from(""),
        Line::from(Span::styled(
            "Claude Workbench",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from("Version: 0.6.1"),
        Line::from(""),
        Line::from("A TUI multiplexer for Claude Code development"),
        Line::from(""),
        Line::from(Span::styled(
            "Components:",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from("  • Ratatui - Terminal UI framework"),
        Line::from("  • portable-pty - PTY management"),
        Line::from("  • vt100 - Terminal emulation"),
        Line::from("  • syntect - Syntax highlighting"),
        Line::from(""),
        Line::from(Span::styled(
            "Keyboard Shortcuts:",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from("  F1-F6: Switch panes"),
        Line::from("  F8: Open settings"),
        Line::from("  Ctrl+Q/C: Quit"),
    ];

    let paragraph = Paragraph::new(text).alignment(Alignment::Center);
    frame.render_widget(paragraph, area);
}

fn render_footer(frame: &mut Frame, area: Rect, state: &SettingsState) {
    let text = if state.editing.is_some() {
        "Enter: Save │ Esc: Cancel"
    } else if state.category == SettingsCategory::Templates {
        "Tab: Category │ j/k: Navigate │ Enter: Select │ s: Save & Close │ Esc: Close"
    } else {
        "Tab: Category │ j/k: Navigate │ Enter: Edit │ Space: Toggle │ s: Save & Close │ Esc: Close"
    };

    let footer = Paragraph::new(text)
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::DarkGray))
        .block(Block::default().borders(Borders::TOP));
    frame.render_widget(footer, area);
}

fn format_setting(
    label: &str,
    value: &str,
    selected: bool,
    editing: bool,
    input_buffer: &str,
) -> ListItem<'static> {
    let display_value = if editing {
        format!("{}█", input_buffer)
    } else {
        value.to_string()
    };

    let style = if selected {
        Style::default().fg(Color::Black).bg(Color::Cyan)
    } else {
        Style::default()
    };

    let text = format!("{:<25} {}", format!("{}:", label), display_value);
    ListItem::new(Line::from(text)).style(style)
}

fn format_bool_setting(label: &str, value: bool, selected: bool) -> ListItem<'static> {
    let style = if selected {
        Style::default().fg(Color::Black).bg(Color::Cyan)
    } else {
        Style::default()
    };

    let marker = if value { "[✓]" } else { "[ ]" };
    let text = format!("{:<25} {}", format!("{}:", label), marker);
    ListItem::new(Line::from(text)).style(style)
}

fn format_action_setting(label: &str, selected: bool) -> ListItem<'static> {
    let style = if selected {
        Style::default().fg(Color::Black).bg(Color::Cyan)
    } else {
        Style::default().fg(Color::Yellow)
    };

    let text = format!("▶ {}", label);
    ListItem::new(Line::from(text)).style(style)
}

fn create_settings_list(items: Vec<ListItem<'static>>) -> List<'static> {
    List::new(items).block(Block::default().borders(Borders::NONE))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_category_navigation() {
        let mut state = SettingsState::default();
        assert_eq!(state.category, SettingsCategory::General);

        state.next_category();
        assert_eq!(state.category, SettingsCategory::Layout);

        state.prev_category();
        assert_eq!(state.category, SettingsCategory::General);
    }

    #[test]
    fn test_item_navigation() {
        let mut state = SettingsState::default();
        assert_eq!(state.selected_idx, 0);

        state.move_down();
        assert_eq!(state.selected_idx, 1);

        state.move_up();
        assert_eq!(state.selected_idx, 0);
    }
}
