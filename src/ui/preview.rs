use ratatui::{
    prelude::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, Wrap},
    Frame,
};
use std::fs;
use std::path::{Path, PathBuf};
use tui_textarea::TextArea;

use crate::types::EditorMode;
use crate::ui::syntax::SyntaxManager;

/// Check if a file is a Markdown file based on extension
fn is_markdown_file(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|e| e.to_str()),
        Some("md") | Some("markdown") | Some("mdown") | Some("mkd")
    )
}

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

    // Edit-mode highlighting cache (updated on each change)
    pub edit_highlighted_lines: Vec<Line<'static>>,

    // Markdown rendering flag
    pub is_markdown: bool,
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
            edit_highlighted_lines: Vec::new(),
            is_markdown: false,
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
        self.is_markdown = is_markdown_file(&path);

        // Set syntax name (Markdown or detected syntax)
        if self.is_markdown {
            self.syntax_name = Some("Markdown".to_string());
        } else {
            self.syntax_name = syntax_manager.detect_syntax_name(&path);
        }

        if let Ok(content) = fs::read_to_string(&path) {
            self.content = content.clone();
            self.original_content = content.clone();

            // Use tui-markdown for markdown files, syntect for others
            if self.is_markdown {
                let md_text = tui_markdown::from_str(&content);
                // Convert to owned Lines to avoid lifetime issues
                self.highlighted_lines = md_text
                    .lines
                    .into_iter()
                    .map(|line| {
                        Line::from(
                            line.spans
                                .into_iter()
                                .map(|span| Span::styled(span.content.to_string(), span.style))
                                .collect::<Vec<_>>(),
                        )
                    })
                    .collect();
            } else {
                self.highlighted_lines = syntax_manager.highlight(&content, &path);
            }
        } else if path.is_dir() {
            self.content = "[Directory]".to_string();
            self.highlighted_lines = vec![Line::from("[Directory]")];
            self.is_markdown = false;
        } else {
            self.content = "[Binary or unreadable file]".to_string();
            self.highlighted_lines = vec![Line::from("[Binary or unreadable file]")];
            self.is_markdown = false;
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

        // Configure textarea appearance (minimal - we render our own highlighting)
        textarea.set_line_number_style(Style::default().fg(Color::DarkGray));
        textarea.set_cursor_line_style(Style::default());
        textarea.set_cursor_style(Style::default().add_modifier(Modifier::REVERSED));

        self.editor = Some(textarea);
        self.original_content = self.content.clone();
        self.mode = EditorMode::Edit;
        self.modified = false;

        // Copy highlighted lines for edit mode
        self.edit_highlighted_lines = self.highlighted_lines.clone();
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

    /// Update edit-mode highlighting based on current editor content
    pub fn update_edit_highlighting(&mut self, syntax_manager: &SyntaxManager) {
        if let (Some(editor), Some(path)) = (&self.editor, &self.current_file) {
            let content = editor.lines().join("\n");
            self.edit_highlighted_lines = syntax_manager.highlight(&content, path);
        }
    }
}

pub fn render(
    f: &mut Frame,
    area: Rect,
    state: &PreviewState,
    is_focused: bool,
    selection_range: Option<(usize, usize)>,
) {
    let title = build_title(state);
    let selection_active = selection_range.is_some();
    let border_style = get_border_style(is_focused, state.mode, state.modified, selection_active);

    let block = Block::bordered()
        .title(format!(" {} ", title))
        .border_style(border_style);

    match state.mode {
        EditorMode::Edit => {
            // Render highlighted content with cursor overlay in edit mode
            if let Some(editor) = &state.editor {
                let inner = block.inner(area);
                let (cursor_row, cursor_col) = editor.cursor();

                // Calculate scroll to keep cursor visible
                let visible_height = inner.height.saturating_sub(1) as usize;
                let scroll_offset = if cursor_row >= visible_height {
                    cursor_row.saturating_sub(visible_height / 2)
                } else {
                    0
                };

                // Build lines with cursor
                let mut lines_with_cursor: Vec<Line<'static>> = Vec::new();
                let editor_lines = editor.lines();

                for (idx, line_content) in editor_lines.iter().enumerate() {
                    // Get the highlighted line if available, otherwise use plain text
                    let base_line = state.edit_highlighted_lines
                        .get(idx)
                        .cloned()
                        .unwrap_or_else(|| Line::from(line_content.clone()));

                    if idx == cursor_row {
                        // Insert cursor into this line
                        let line_with_cursor = insert_cursor_into_line(&base_line, cursor_col, line_content);
                        lines_with_cursor.push(line_with_cursor);
                    } else {
                        lines_with_cursor.push(base_line);
                    }
                }

                let paragraph = Paragraph::new(lines_with_cursor)
                    .block(block)
                    .scroll((scroll_offset as u16, 0));

                f.render_widget(paragraph, area);

                // Scrollbar
                let content_length = editor_lines.len();
                if content_length > 0 {
                    let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                        .begin_symbol(Some("▲"))
                        .end_symbol(Some("▼"));

                    let mut scrollbar_state = ScrollbarState::new(content_length)
                        .position(scroll_offset);

                    f.render_stateful_widget(scrollbar, area, &mut scrollbar_state);
                }
            }
        }
        EditorMode::ReadOnly => {
            // Apply selection highlighting if active
            let lines = if let Some((start, end)) = selection_range {
                state.highlighted_lines.iter().enumerate().map(|(idx, line)| {
                    if idx >= start && idx <= end {
                        // Apply DarkGray background to selected lines
                        let styled_spans: Vec<Span> = line.spans.iter().map(|span| {
                            Span::styled(
                                span.content.clone(),
                                span.style.bg(Color::DarkGray),
                            )
                        }).collect();
                        Line::from(styled_spans)
                    } else {
                        line.clone()
                    }
                }).collect::<Vec<_>>()
            } else {
                state.highlighted_lines.clone()
            };

            // Render highlighted content in read-only mode
            let paragraph = Paragraph::new(lines)
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

/// Insert a cursor (reversed style) into a line at the given column
fn insert_cursor_into_line(line: &Line<'static>, col: usize, raw_text: &str) -> Line<'static> {
    // Use REVERSED + SLOW_BLINK for maximum visibility
    let cursor_style = Style::default()
        .add_modifier(Modifier::REVERSED | Modifier::SLOW_BLINK);

    // Get the character at cursor position, or use block char for visibility at end of line
    let cursor_char: char = raw_text.chars().nth(col).unwrap_or('█');

    // Build spans: before cursor, cursor char, after cursor
    let mut result_spans: Vec<Span<'static>> = Vec::new();
    let mut current_col = 0;
    let mut cursor_inserted = false;

    for span in line.spans.iter() {
        let span_text = span.content.as_ref();
        let span_len = span_text.chars().count();

        if !cursor_inserted && current_col + span_len > col {
            // Cursor is within this span
            let offset_in_span = col - current_col;
            let chars: Vec<char> = span_text.chars().collect();

            // Part before cursor
            if offset_in_span > 0 {
                let before: String = chars[..offset_in_span].iter().collect();
                result_spans.push(Span::styled(before, span.style));
            }

            // Cursor character
            result_spans.push(Span::styled(cursor_char.to_string(), cursor_style));

            // Part after cursor
            if offset_in_span + 1 < chars.len() {
                let after: String = chars[offset_in_span + 1..].iter().collect();
                result_spans.push(Span::styled(after, span.style));
            }

            cursor_inserted = true;
        } else if !cursor_inserted && current_col + span_len == col {
            // Cursor is right after this span
            result_spans.push(span.clone());
        } else {
            result_spans.push(span.clone());
        }

        current_col += span_len;
    }

    // If cursor wasn't inserted (at end of line), add it
    if !cursor_inserted {
        result_spans.push(Span::styled(cursor_char.to_string(), cursor_style));
    }

    Line::from(result_spans)
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

fn get_border_style(is_focused: bool, mode: EditorMode, modified: bool, selection_active: bool) -> Style {
    // Selection mode takes priority (yellow border like terminal panes)
    if selection_active {
        return Style::default().fg(Color::Yellow);
    }
    match (mode, modified, is_focused) {
        (EditorMode::Edit, true, _) => Style::default().fg(Color::Yellow), // Edit + modified
        (EditorMode::Edit, false, _) => Style::default().fg(Color::Cyan),  // Edit + saved
        (_, _, true) => Style::default().fg(Color::Green),                 // Focused
        _ => Style::default(),                                             // Default
    }
}
