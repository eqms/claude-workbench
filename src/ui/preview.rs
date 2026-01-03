use ratatui::{
    prelude::Rect,
    style::{Color, Modifier, Style},
    text::Line,
    widgets::{Block, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, Wrap},
    Frame,
};
use std::fs;
use std::path::PathBuf;
use tui_textarea::TextArea;

use crate::types::EditorMode;
use crate::ui::syntax::SyntaxManager;

#[derive(Debug)]
pub struct PreviewState {
    // Core state
    pub current_file: Option<PathBuf>,
    pub content: String,
    pub scroll: u16,

    // Editor state
    pub mode: EditorMode,
    pub editor: Option<TextArea<'static>>,
    pub modified: bool,
    pub original_content: String,

    // Syntax highlighting cache
    pub highlighted_lines: Vec<Line<'static>>,
    pub syntax_name: Option<String>,
}

impl Default for PreviewState {
    fn default() -> Self {
        Self {
            current_file: None,
            content: String::new(),
            scroll: 0,
            mode: EditorMode::ReadOnly,
            editor: None,
            modified: false,
            original_content: String::new(),
            highlighted_lines: Vec::new(),
            syntax_name: None,
        }
    }
}

impl PreviewState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn load_file(&mut self, path: PathBuf, syntax_manager: &SyntaxManager) {
        // Exit edit mode if active
        if self.mode == EditorMode::Edit {
            self.exit_edit_mode(true);
        }

        self.current_file = Some(path.clone());
        self.scroll = 0;
        self.syntax_name = syntax_manager.detect_syntax_name(&path);

        if let Ok(content) = fs::read_to_string(&path) {
            self.content = content.clone();
            self.original_content = content.clone();
            self.highlighted_lines = syntax_manager.highlight(&content, &path);
        } else if path.is_dir() {
            self.content = "[Directory]".to_string();
            self.highlighted_lines = vec![Line::from("[Directory]")];
        } else {
            self.content = "[Binary or unreadable file]".to_string();
            self.highlighted_lines = vec![Line::from("[Binary or unreadable file]")];
        }
    }

    pub fn scroll_down(&mut self) {
        self.scroll = self.scroll.saturating_add(1);
    }

    pub fn scroll_up(&mut self) {
        self.scroll = self.scroll.saturating_sub(1);
    }

    /// Enter edit mode - only for readable files
    pub fn enter_edit_mode(&mut self) {
        if self.current_file.is_none() || self.mode == EditorMode::Edit {
            return;
        }

        // Check if file is editable (not directory, not binary)
        if let Some(path) = &self.current_file {
            if !path.is_file() {
                return;
            }
        }

        let lines: Vec<String> = self.content.lines().map(String::from).collect();
        let mut textarea = TextArea::new(lines);

        // Configure textarea appearance
        textarea.set_line_number_style(Style::default().fg(Color::DarkGray));
        textarea.set_cursor_line_style(Style::default().add_modifier(Modifier::UNDERLINED));
        textarea.set_cursor_style(Style::default().add_modifier(Modifier::REVERSED));

        self.editor = Some(textarea);
        self.original_content = self.content.clone();
        self.mode = EditorMode::Edit;
        self.modified = false;
    }

    /// Exit edit mode
    pub fn exit_edit_mode(&mut self, discard: bool) {
        if let Some(editor) = self.editor.take() {
            if !discard {
                self.content = editor.lines().join("\n");
            }
        }
        self.mode = EditorMode::ReadOnly;
        self.modified = false;
    }

    /// Save file to disk
    pub fn save(&mut self) -> anyhow::Result<()> {
        if let (Some(path), Some(editor)) = (&self.current_file, &self.editor) {
            let content = editor.lines().join("\n");
            fs::write(path, &content)?;
            self.original_content = content.clone();
            self.content = content;
            self.modified = false;
        }
        Ok(())
    }

    /// Check if content has been modified
    pub fn is_modified(&self) -> bool {
        if let Some(editor) = &self.editor {
            editor.lines().join("\n") != self.original_content
        } else {
            false
        }
    }

    /// Update modified flag based on current editor state
    pub fn update_modified(&mut self) {
        self.modified = self.is_modified();
    }

    /// Refresh highlighting after edit
    pub fn refresh_highlighting(&mut self, syntax_manager: &SyntaxManager) {
        if let Some(path) = &self.current_file {
            self.highlighted_lines = syntax_manager.highlight(&self.content, path);
        }
    }
}

pub fn render(f: &mut Frame, area: Rect, state: &PreviewState, is_focused: bool) {
    let title = build_title(state);
    let border_style = get_border_style(is_focused, state.mode, state.modified);

    let block = Block::bordered()
        .title(format!(" {} ", title))
        .border_style(border_style);

    match state.mode {
        EditorMode::Edit => {
            // Render TextArea in edit mode
            if let Some(editor) = &state.editor {
                let inner = block.inner(area);
                f.render_widget(block, area);
                f.render_widget(editor, inner);
            }
        }
        EditorMode::ReadOnly => {
            // Render highlighted content in read-only mode
            let paragraph = Paragraph::new(state.highlighted_lines.clone())
                .block(block)
                .wrap(Wrap { trim: false })
                .scroll((state.scroll, 0));

            f.render_widget(paragraph, area);

            // Scrollbar for read-only mode
            let content_length = state.highlighted_lines.len();
            if content_length > 0 {
                let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                    .begin_symbol(Some("▲"))
                    .end_symbol(Some("▼"));

                let mut scrollbar_state = ScrollbarState::new(content_length)
                    .position(state.scroll as usize);

                f.render_stateful_widget(scrollbar, area, &mut scrollbar_state);
            }
        }
    }
}

fn build_title(state: &PreviewState) -> String {
    let mut title = state
        .current_file
        .as_ref()
        .and_then(|p| p.file_name())
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "Preview".to_string());

    // Add syntax name if available
    if let Some(syntax) = &state.syntax_name {
        title.push_str(&format!(" [{}]", syntax));
    }

    // Add modified indicator
    if state.modified {
        title.push_str(" [+]");
    }

    // Add mode indicator
    if state.mode == EditorMode::Edit {
        title.push_str(" EDIT");
    }

    title
}

fn get_border_style(is_focused: bool, mode: EditorMode, modified: bool) -> Style {
    match (mode, modified, is_focused) {
        (EditorMode::Edit, true, _) => Style::default().fg(Color::Yellow), // Edit + modified
        (EditorMode::Edit, false, _) => Style::default().fg(Color::Cyan),  // Edit + saved
        (_, _, true) => Style::default().fg(Color::Green),                 // Focused
        _ => Style::default(),                                             // Default
    }
}
