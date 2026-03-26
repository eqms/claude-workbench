//! Settings menu UI and state

use crate::app_detector::{self, DetectedApp};
use crate::config::Config;
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
    Document,
    About,
}

impl SettingsCategory {
    pub fn all() -> &'static [SettingsCategory] {
        &[
            SettingsCategory::General,
            SettingsCategory::Layout,
            SettingsCategory::Paths,
            SettingsCategory::Document,
            SettingsCategory::About,
        ]
    }

    pub fn title(&self) -> &'static str {
        match self {
            SettingsCategory::General => "General",
            SettingsCategory::Layout => "Layout",
            SettingsCategory::Paths => "Paths",
            SettingsCategory::Document => "Document",
            SettingsCategory::About => "About",
        }
    }

    pub fn next(&self) -> Self {
        match self {
            SettingsCategory::General => SettingsCategory::Layout,
            SettingsCategory::Layout => SettingsCategory::Paths,
            SettingsCategory::Paths => SettingsCategory::Document,
            SettingsCategory::Document => SettingsCategory::About,
            SettingsCategory::About => SettingsCategory::General,
        }
    }

    pub fn prev(&self) -> Self {
        match self {
            SettingsCategory::General => SettingsCategory::About,
            SettingsCategory::Layout => SettingsCategory::General,
            SettingsCategory::Paths => SettingsCategory::Layout,
            SettingsCategory::Document => SettingsCategory::Paths,
            SettingsCategory::About => SettingsCategory::Document,
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
    Browser,
    ExternalEditor,
    ExportDir,
    // Document
    CompanyName,
    CompanyFooterText,
    CompanyAuthor,
    CompanyWebsite,
    DocBodyFont,
    DocCodeFont,
    DocTableHeaderBg,
    DocTableBorder,
    DocPageSize,
    DocPageMargin,
}

// ── Dropdown types ─────────────────────────────────────────────────────

/// A single item in the app selection dropdown
#[derive(Debug, Clone)]
pub struct DropdownItem {
    /// Display text shown in the list
    pub display: String,
    /// Value to store in config
    pub value: String,
    /// Whether this is the "Custom path..." fallback entry
    pub is_custom: bool,
}

/// State for the browser/editor selection dropdown
#[derive(Debug, Clone)]
pub struct AppDropdownState {
    pub field: SettingsField,
    pub items: Vec<DropdownItem>,
    pub selected_idx: usize,
    pub scroll_offset: usize,
}

impl AppDropdownState {
    /// Maximum visible items before scrolling
    const MAX_VISIBLE: usize = 12;

    pub fn move_up(&mut self) {
        if self.selected_idx > 0 {
            self.selected_idx -= 1;
            if self.selected_idx < self.scroll_offset {
                self.scroll_offset = self.selected_idx;
            }
        }
    }

    pub fn move_down(&mut self) {
        if self.selected_idx + 1 < self.items.len() {
            self.selected_idx += 1;
            if self.selected_idx >= self.scroll_offset + Self::MAX_VISIBLE {
                self.scroll_offset = self.selected_idx + 1 - Self::MAX_VISIBLE;
            }
        }
    }
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
    pub browser: String,
    pub external_editor: String,
    pub export_dir: String,
    // Document settings
    pub company_name: String,
    pub company_footer_text: String,
    pub company_author: String,
    pub company_website: String,
    pub doc_body_font: String,
    pub doc_code_font: String,
    pub doc_table_header_bg: String,
    pub doc_table_border: String,
    pub doc_page_size: String,
    pub doc_page_margin: String,

    // App detection (cached)
    pub dropdown: Option<AppDropdownState>,
    pub detected_browsers: Vec<DetectedApp>,
    pub detected_editors: Vec<DetectedApp>,
    pub apps_detected: bool,

    // Track if changes were made
    pub has_changes: bool,
}

impl Default for SettingsState {
    fn default() -> Self {
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
            browser: String::new(),
            external_editor: String::new(),
            export_dir: String::new(),
            company_name: "Musterfirma".to_string(),
            company_footer_text: "Generated by {company_name}".to_string(),
            company_author: "{company_name}".to_string(),
            company_website: String::new(),
            doc_body_font: "Calibri, -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif"
                .to_string(),
            doc_code_font: "'SF Mono', Monaco, 'Cascadia Code', Consolas, monospace".to_string(),
            doc_table_header_bg: "#D5E8F0".to_string(),
            doc_table_border: "#999999".to_string(),
            doc_page_size: "A4".to_string(),
            doc_page_margin: "2.5cm".to_string(),
            dropdown: None,
            detected_browsers: Vec::new(),
            detected_editors: Vec::new(),
            apps_detected: false,
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
        self.browser = config.ui.browser.clone();
        self.external_editor = config.ui.external_editor.clone();
        self.export_dir = config.ui.export_dir.clone();
        // Document settings
        self.company_name = config.document.company.name.clone();
        self.company_footer_text = config.document.company.footer_text.clone();
        self.company_author = config.document.company.author.clone();
        self.company_website = config.document.company.website.clone();
        self.doc_body_font = config.document.fonts.body.clone();
        self.doc_code_font = config.document.fonts.code.clone();
        self.doc_table_header_bg = config.document.colors.table_header_bg.clone();
        self.doc_table_border = config.document.colors.table_border.clone();
        self.doc_page_size = config.document.pdf.page_size.clone();
        self.doc_page_margin = config.document.pdf.margin.clone();
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
        config.ui.browser = self.browser.clone();
        config.ui.external_editor = self.external_editor.clone();
        config.ui.export_dir = self.export_dir.clone();
        // Document settings
        config.document.company.name = self.company_name.clone();
        config.document.company.footer_text = self.company_footer_text.clone();
        config.document.company.author = self.company_author.clone();
        config.document.company.website = self.company_website.clone();
        config.document.fonts.body = self.doc_body_font.clone();
        config.document.fonts.code = self.doc_code_font.clone();
        config.document.colors.table_header_bg = self.doc_table_header_bg.clone();
        config.document.colors.table_border = self.doc_table_border.clone();
        config.document.pdf.page_size = self.doc_page_size.clone();
        config.document.pdf.margin = self.doc_page_margin.clone();
    }

    pub fn open(&mut self, config: &Config) {
        self.load_from_config(config);
        self.visible = true;
        self.category = SettingsCategory::General;
        self.selected_idx = 0;
        self.editing = None;
        self.dropdown = None;

        // Detect installed apps once
        if !self.apps_detected {
            self.detected_browsers = app_detector::detect_browsers();
            self.detected_editors = app_detector::detect_editors();
            self.apps_detected = true;
        }
    }

    pub fn close(&mut self) {
        self.visible = false;
        self.editing = None;
        self.dropdown = None;
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
            SettingsCategory::Paths => 5,   // claude, lazygit, browser, external_editor, export_dir
            SettingsCategory::Document => 10,
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
                2 => Some(SettingsField::Browser),
                3 => Some(SettingsField::ExternalEditor),
                4 => Some(SettingsField::ExportDir),
                _ => None,
            },
            SettingsCategory::Document => match self.selected_idx {
                0 => Some(SettingsField::CompanyName),
                1 => Some(SettingsField::CompanyFooterText),
                2 => Some(SettingsField::CompanyAuthor),
                3 => Some(SettingsField::CompanyWebsite),
                4 => Some(SettingsField::DocBodyFont),
                5 => Some(SettingsField::DocCodeFont),
                6 => Some(SettingsField::DocTableHeaderBg),
                7 => Some(SettingsField::DocTableBorder),
                8 => Some(SettingsField::DocPageSize),
                9 => Some(SettingsField::DocPageMargin),
                _ => None,
            },
            _ => None,
        };

        if let Some(f) = field {
            // Browser and ExternalEditor open a dropdown instead of text input
            match &f {
                SettingsField::Browser => {
                    self.open_browser_dropdown();
                    return;
                }
                SettingsField::ExternalEditor => {
                    self.open_editor_dropdown();
                    return;
                }
                _ => {}
            }

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
                SettingsField::ExportDir => self.export_dir.clone(),
                SettingsField::CompanyName => self.company_name.clone(),
                SettingsField::CompanyFooterText => self.company_footer_text.clone(),
                SettingsField::CompanyAuthor => self.company_author.clone(),
                SettingsField::CompanyWebsite => self.company_website.clone(),
                SettingsField::DocBodyFont => self.doc_body_font.clone(),
                SettingsField::DocCodeFont => self.doc_code_font.clone(),
                SettingsField::DocTableHeaderBg => self.doc_table_header_bg.clone(),
                SettingsField::DocTableBorder => self.doc_table_border.clone(),
                SettingsField::DocPageSize => self.doc_page_size.clone(),
                SettingsField::DocPageMargin => self.doc_page_margin.clone(),
                SettingsField::Browser | SettingsField::ExternalEditor => unreachable!(),
                SettingsField::CheckForUpdates => {
                    unreachable!("CheckForUpdates is an action, not a field")
                }
            };
            self.editing = Some(f);
        }
    }

    /// Start text-based editing for a field (used by "Custom path..." dropdown option)
    pub fn start_text_editing(&mut self, field: SettingsField) {
        self.input_buffer = match &field {
            SettingsField::Browser => self.browser.clone(),
            SettingsField::ExternalEditor => self.external_editor.clone(),
            _ => String::new(),
        };
        self.editing = Some(field);
    }

    /// Open dropdown for browser selection
    fn open_browser_dropdown(&mut self) {
        let mut items = vec![DropdownItem {
            display: "(system default)".to_string(),
            value: String::new(),
            is_custom: false,
        }];

        for app in &self.detected_browsers {
            items.push(DropdownItem {
                display: app.display_name.clone(),
                value: app.command.clone(),
                is_custom: false,
            });
        }

        items.push(DropdownItem {
            display: "Custom path...".to_string(),
            value: String::new(),
            is_custom: true,
        });

        // Pre-select the current value
        let selected_idx = items
            .iter()
            .position(|item| !item.is_custom && item.value == self.browser)
            .unwrap_or(0);

        self.dropdown = Some(AppDropdownState {
            field: SettingsField::Browser,
            items,
            selected_idx,
            scroll_offset: 0,
        });
    }

    /// Open dropdown for editor selection
    fn open_editor_dropdown(&mut self) {
        let mut items = vec![DropdownItem {
            display: "(not configured)".to_string(),
            value: String::new(),
            is_custom: false,
        }];

        for app in &self.detected_editors {
            items.push(DropdownItem {
                display: app.display_name.clone(),
                value: app.command.clone(),
                is_custom: false,
            });
        }

        items.push(DropdownItem {
            display: "Custom path...".to_string(),
            value: String::new(),
            is_custom: true,
        });

        // Pre-select the current value
        let selected_idx = items
            .iter()
            .position(|item| !item.is_custom && item.value == self.external_editor)
            .unwrap_or(0);

        self.dropdown = Some(AppDropdownState {
            field: SettingsField::ExternalEditor,
            items,
            selected_idx,
            scroll_offset: 0,
        });
    }

    /// Move dropdown selection up
    pub fn dropdown_move_up(&mut self) {
        if let Some(dd) = &mut self.dropdown {
            dd.move_up();
        }
    }

    /// Move dropdown selection down
    pub fn dropdown_move_down(&mut self) {
        if let Some(dd) = &mut self.dropdown {
            dd.move_down();
        }
    }

    /// Confirm dropdown selection. Returns true if "Custom path..." was selected
    /// (caller should then handle text input mode).
    pub fn dropdown_confirm(&mut self) -> bool {
        if let Some(dd) = self.dropdown.take() {
            if let Some(item) = dd.items.get(dd.selected_idx) {
                if item.is_custom {
                    // Open text input for custom path
                    self.start_text_editing(dd.field);
                    return true;
                }
                match dd.field {
                    SettingsField::Browser => {
                        self.browser = item.value.clone();
                    }
                    SettingsField::ExternalEditor => {
                        self.external_editor = item.value.clone();
                    }
                    _ => {}
                }
                self.has_changes = true;
            }
        }
        false
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
                SettingsField::Browser => self.browser = value,
                SettingsField::ExternalEditor => self.external_editor = value,
                SettingsField::ExportDir => self.export_dir = value,
                SettingsField::CompanyName => self.company_name = value,
                SettingsField::CompanyFooterText => self.company_footer_text = value,
                SettingsField::CompanyAuthor => self.company_author = value,
                SettingsField::CompanyWebsite => self.company_website = value,
                SettingsField::DocBodyFont => self.doc_body_font = value,
                SettingsField::DocCodeFont => self.doc_code_font = value,
                SettingsField::DocTableHeaderBg => self.doc_table_header_bg = value,
                SettingsField::DocTableBorder => self.doc_table_border = value,
                SettingsField::DocPageSize => self.doc_page_size = value,
                SettingsField::DocPageMargin => self.doc_page_margin = value,
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
    /// Check if dropdown is currently active
    pub fn has_dropdown(&self) -> bool {
        self.dropdown.is_some()
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

    // Render dropdown overlay on top if active
    if state.dropdown.is_some() {
        render_app_dropdown(frame, popup_area, state);
    }
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
        SettingsCategory::Document => render_document(frame, area, state),
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
    let browser_display = if state.browser.is_empty() {
        "(system default)".to_string()
    } else {
        // Show friendly name if we can match the command to a detected app
        state
            .detected_browsers
            .iter()
            .find(|app| app.command == state.browser)
            .map(|app| format!("{} ({})", app.display_name, app.command))
            .unwrap_or_else(|| state.browser.clone())
    };
    let editor_display = if state.external_editor.is_empty() {
        "(not configured)".to_string()
    } else {
        state
            .detected_editors
            .iter()
            .find(|app| app.command == state.external_editor)
            .map(|app| format!("{} ({})", app.display_name, app.command))
            .unwrap_or_else(|| state.external_editor.clone())
    };
    let export_dir_display = if state.export_dir.is_empty() {
        "~/Downloads (default)".to_string()
    } else {
        state.export_dir.clone()
    };

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
        format_dropdown_setting(
            "Browser",
            &browser_display,
            state.selected_idx == 2,
            state.editing.as_ref() == Some(&SettingsField::Browser),
            &state.input_buffer,
        ),
        format_dropdown_setting(
            "External Editor",
            &editor_display,
            state.selected_idx == 3,
            state.editing.as_ref() == Some(&SettingsField::ExternalEditor),
            &state.input_buffer,
        ),
        format_setting(
            "Export Directory",
            &export_dir_display,
            state.selected_idx == 4,
            state.editing.as_ref() == Some(&SettingsField::ExportDir),
            &state.input_buffer,
        ),
    ];

    let list = create_settings_list(items);
    frame.render_widget(list, area);
}

fn render_document(frame: &mut Frame, area: Rect, state: &SettingsState) {
    let items = vec![
        format_setting(
            "Company Name",
            &state.company_name,
            state.selected_idx == 0,
            state.editing.as_ref() == Some(&SettingsField::CompanyName),
            &state.input_buffer,
        ),
        format_setting(
            "Footer Text",
            &state.company_footer_text,
            state.selected_idx == 1,
            state.editing.as_ref() == Some(&SettingsField::CompanyFooterText),
            &state.input_buffer,
        ),
        format_setting(
            "Author",
            &state.company_author,
            state.selected_idx == 2,
            state.editing.as_ref() == Some(&SettingsField::CompanyAuthor),
            &state.input_buffer,
        ),
        format_setting(
            "Website",
            &state.company_website,
            state.selected_idx == 3,
            state.editing.as_ref() == Some(&SettingsField::CompanyWebsite),
            &state.input_buffer,
        ),
        format_setting(
            "Body Font",
            &state.doc_body_font,
            state.selected_idx == 4,
            state.editing.as_ref() == Some(&SettingsField::DocBodyFont),
            &state.input_buffer,
        ),
        format_setting(
            "Code Font",
            &state.doc_code_font,
            state.selected_idx == 5,
            state.editing.as_ref() == Some(&SettingsField::DocCodeFont),
            &state.input_buffer,
        ),
        format_setting(
            "Table Header BG",
            &state.doc_table_header_bg,
            state.selected_idx == 6,
            state.editing.as_ref() == Some(&SettingsField::DocTableHeaderBg),
            &state.input_buffer,
        ),
        format_setting(
            "Table Border",
            &state.doc_table_border,
            state.selected_idx == 7,
            state.editing.as_ref() == Some(&SettingsField::DocTableBorder),
            &state.input_buffer,
        ),
        format_setting(
            "Page Size",
            &state.doc_page_size,
            state.selected_idx == 8,
            state.editing.as_ref() == Some(&SettingsField::DocPageSize),
            &state.input_buffer,
        ),
        format_setting(
            "Page Margin",
            &state.doc_page_margin,
            state.selected_idx == 9,
            state.editing.as_ref() == Some(&SettingsField::DocPageMargin),
            &state.input_buffer,
        ),
    ];

    let list = create_settings_list(items);
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
        Line::from(format!("Version: {}", env!("CARGO_PKG_VERSION"))),
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
    let text = if state.dropdown.is_some() {
        "j/k: Navigate │ Enter: Select │ Esc: Cancel"
    } else if state.editing.is_some() {
        "Enter: Save │ Esc: Cancel │ Ctrl+V: Paste"
    } else {
        "Tab: Category │ j/k: Navigate │ Enter: Edit │ Space: Toggle │ s: Save & Close │ Esc: Close"
    };

    let footer = Paragraph::new(text)
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::DarkGray))
        .block(Block::default().borders(Borders::TOP));
    frame.render_widget(footer, area);
}

/// Render the app selection dropdown overlay
fn render_app_dropdown(frame: &mut Frame, popup_area: Rect, state: &SettingsState) {
    let dd = match &state.dropdown {
        Some(dd) => dd,
        None => return,
    };

    let title = match dd.field {
        SettingsField::Browser => " Select Browser ",
        SettingsField::ExternalEditor => " Select Editor ",
        _ => " Select ",
    };

    // Get current value to mark active item
    let current_value = match dd.field {
        SettingsField::Browser => &state.browser,
        SettingsField::ExternalEditor => &state.external_editor,
        _ => &state.browser,
    };

    // Calculate dropdown dimensions
    let dd_width = popup_area.width.saturating_sub(8).min(60);
    let visible_items = dd.items.len().min(AppDropdownState::MAX_VISIBLE);
    let dd_height = (visible_items as u16) + 2; // +2 for borders

    // Center the dropdown over the settings popup
    let dd_x = popup_area.x + (popup_area.width.saturating_sub(dd_width)) / 2;
    let dd_y = popup_area.y + (popup_area.height.saturating_sub(dd_height)) / 2;

    let dd_area = Rect::new(dd_x, dd_y, dd_width, dd_height);

    // Clear and draw border
    frame.render_widget(Clear, dd_area);
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow));
    let inner = block.inner(dd_area);
    frame.render_widget(block, dd_area);

    // Build list items with scrolling
    let items: Vec<ListItem> = dd
        .items
        .iter()
        .enumerate()
        .skip(dd.scroll_offset)
        .take(visible_items)
        .map(|(i, item)| {
            let is_selected = i == dd.selected_idx;
            let is_active = !item.is_custom && item.value == *current_value;

            let marker = if is_active { "● " } else { "  " };

            let style = if is_selected {
                Style::default().fg(Color::Black).bg(Color::Cyan)
            } else if item.is_custom {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::ITALIC)
            } else if is_active {
                Style::default().fg(Color::Green)
            } else {
                Style::default()
            };

            let text = format!("{}{}", marker, item.display);
            ListItem::new(Line::from(text)).style(style)
        })
        .collect();

    let list = List::new(items);
    frame.render_widget(list, inner);
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

/// Format a dropdown-enabled setting (shows ▼ indicator when not editing)
fn format_dropdown_setting(
    label: &str,
    value: &str,
    selected: bool,
    editing: bool,
    input_buffer: &str,
) -> ListItem<'static> {
    let display_value = if editing {
        format!("{}█", input_buffer)
    } else {
        format!("{} ▼", value)
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

    #[test]
    fn test_paths_item_count() {
        let state = SettingsState::default();
        // Paths: claude, lazygit, browser, external_editor, export_dir = 5
        assert_eq!(state.item_count(), 6); // General is default
    }

    #[test]
    fn test_dropdown_confirm_sets_browser() {
        let mut state = SettingsState::default();
        state.detected_browsers = vec![DetectedApp {
            display_name: "Firefox".to_string(),
            command: "open -a Firefox".to_string(),
        }];

        state.open_browser_dropdown();
        assert!(state.dropdown.is_some());

        // Select Firefox (index 1, after "(system default)")
        if let Some(dd) = &mut state.dropdown {
            dd.selected_idx = 1;
        }
        state.dropdown_confirm();
        assert_eq!(state.browser, "open -a Firefox");
        assert!(state.has_changes);
    }

    #[test]
    fn test_dropdown_custom_returns_true() {
        let mut state = SettingsState::default();
        state.detected_browsers = vec![];

        state.open_browser_dropdown();
        // Select "Custom path..." (last item, index 1 since no browsers detected)
        if let Some(dd) = &mut state.dropdown {
            dd.selected_idx = 1; // (system default) + Custom path...
        }
        let is_custom = state.dropdown_confirm();
        assert!(is_custom);
        assert!(state.editing.is_some()); // Text editing mode activated
    }
}
