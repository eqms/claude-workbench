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

use crate::types::{EditorMode, SearchState};
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

    // Search state
    pub search: SearchState,

    // MC Edit style block marking mode
    pub block_marking: bool,
    // Selection start position (row, col) for visualization
    pub selection_start: Option<(usize, usize)>,

    // File modification tracking for auto-refresh
    pub last_modified: Option<std::time::SystemTime>,
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
            search: SearchState::default(),
            block_marking: false,
            selection_start: None,
            last_modified: None,
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

            // Store file modification time for auto-refresh
            self.last_modified = fs::metadata(&path)
                .and_then(|m| m.modified())
                .ok();

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

    /// Check if the currently displayed file has been modified externally
    /// Returns true if the file needs to be reloaded
    pub fn check_file_changed(&self) -> bool {
        // Don't check if in edit mode (user might have unsaved changes)
        if self.mode == EditorMode::Edit {
            return false;
        }

        let Some(path) = &self.current_file else {
            return false;
        };

        let Some(last_mod) = self.last_modified else {
            return false;
        };

        // Check current modification time
        if let Ok(metadata) = fs::metadata(path) {
            if let Ok(current_mod) = metadata.modified() {
                return current_mod > last_mod;
            }
        }

        false
    }

    /// Reload the file if it has been modified externally
    /// Returns true if the file was reloaded
    pub fn reload_if_changed(&mut self, syntax_manager: &SyntaxManager) -> bool {
        if !self.check_file_changed() {
            return false;
        }

        // Reload the file
        if let Some(path) = self.current_file.clone() {
            let current_scroll = self.scroll;
            self.load_file(path, syntax_manager);
            // Restore scroll position (clamped to new content length)
            let max_scroll = self.highlighted_lines.len().saturating_sub(1) as u16;
            self.scroll = current_scroll.min(max_scroll);
            return true;
        }

        false
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

    /// Perform incremental search on content
    pub fn perform_search(&mut self) {
        self.search.matches.clear();

        if self.search.query.is_empty() {
            return;
        }

        // Get content to search (from editor in edit mode, otherwise from content)
        let content = if let Some(editor) = &self.editor {
            editor.lines().join("\n")
        } else {
            self.content.clone()
        };

        let query = if self.search.case_sensitive {
            self.search.query.clone()
        } else {
            self.search.query.to_lowercase()
        };

        for (line_idx, line) in content.lines().enumerate() {
            let search_line = if self.search.case_sensitive {
                line.to_string()
            } else {
                line.to_lowercase()
            };

            let mut start = 0;
            while let Some(pos) = search_line[start..].find(&query) {
                let abs_pos = start + pos;
                self.search
                    .matches
                    .push((line_idx, abs_pos, abs_pos + query.len()));
                start = abs_pos + 1;
            }
        }

        // Reset to first match if current is out of bounds
        if self.search.current_match >= self.search.matches.len() {
            self.search.current_match = 0;
        }
    }

    /// Jump scroll position to current match
    pub fn jump_to_current_match(&mut self) {
        if let Some(line) = self.search.current_match_line() {
            self.scroll = line as u16;
        }
    }

    // ============================================================
    // MC Edit Style Block Operations
    // ============================================================

    /// Toggle block marking mode (MC F3)
    pub fn toggle_block_marking(&mut self) {
        if let Some(editor) = &mut self.editor {
            if self.block_marking {
                // End marking - keep selection visible
                self.block_marking = false;
            } else {
                // Start marking - begin selection at cursor
                let cursor = editor.cursor();
                self.selection_start = Some(cursor);
                editor.start_selection();
                self.block_marking = true;
            }
        }
    }

    /// Copy selection to clipboard (MC F5)
    pub fn copy_block(&mut self) {
        if let Some(editor) = &mut self.editor {
            editor.copy();
            // Don't cancel selection after copy - user might want to see what was copied
        }
    }

    /// Move (cut) selection (MC F6)
    /// User should position cursor and paste (Ctrl+V) to complete move
    pub fn move_block(&mut self) {
        if let Some(editor) = &mut self.editor {
            editor.cut();
            self.block_marking = false;
            self.selection_start = None;
        }
    }

    /// Delete selection (MC F8)
    pub fn delete_block(&mut self) {
        if let Some(editor) = &mut self.editor {
            // Cut deletes selected text and stores in yank buffer
            editor.cut();
            editor.cancel_selection();
            self.block_marking = false;
            self.selection_start = None;
        }
    }

    /// Delete current line (Ctrl+Y in MC)
    pub fn delete_line(&mut self) {
        if let Some(editor) = &mut self.editor {
            use tui_textarea::CursorMove;
            // Move to beginning of line
            editor.move_cursor(CursorMove::Head);
            // Delete entire line including newline
            editor.delete_line_by_end();
            // Delete the newline character to merge with next line
            editor.delete_char();
        }
    }

    /// Extend selection with Shift+Arrow keys
    pub fn extend_selection(&mut self, direction: tui_textarea::CursorMove) {
        if let Some(editor) = &mut self.editor {
            // Start selection if not already marking
            if !self.block_marking {
                let cursor = editor.cursor();
                self.selection_start = Some(cursor);
                editor.start_selection();
                self.block_marking = true;
            }
            // Move cursor to extend selection
            editor.move_cursor(direction);
        }
    }

    /// Cancel selection and exit block marking mode
    pub fn cancel_selection(&mut self) {
        if let Some(editor) = &mut self.editor {
            editor.cancel_selection();
        }
        self.block_marking = false;
        self.selection_start = None;
    }

    /// Get current selection range for visualization: (start_row, start_col, end_row, end_col)
    /// Returns None if no selection is active
    pub fn get_selection_range(&self) -> Option<(usize, usize, usize, usize)> {
        if !self.block_marking {
            return None;
        }
        let start = self.selection_start?;
        let editor = self.editor.as_ref()?;
        let end = editor.cursor();

        // Normalize: start should be before end
        if start.0 < end.0 || (start.0 == end.0 && start.1 <= end.1) {
            Some((start.0, start.1, end.0, end.1))
        } else {
            Some((end.0, end.1, start.0, start.1))
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
            // Split area: editor content + shortcut bar at bottom
            let shortcut_bar_height = 1;
            let editor_area = Rect::new(
                area.x,
                area.y,
                area.width,
                area.height.saturating_sub(shortcut_bar_height),
            );
            let shortcut_area = Rect::new(
                area.x,
                area.y + editor_area.height,
                area.width,
                shortcut_bar_height,
            );

            // Render highlighted content with cursor overlay in edit mode
            if let Some(editor) = &state.editor {
                let inner = block.inner(editor_area);
                let (cursor_row, cursor_col) = editor.cursor();

                // Calculate scroll to keep cursor visible
                let visible_height = inner.height.saturating_sub(1) as usize;
                let scroll_offset = if cursor_row >= visible_height {
                    cursor_row.saturating_sub(visible_height / 2)
                } else {
                    0
                };

                // Get selection range for visualization
                let selection = state.get_selection_range();

                // Build lines with cursor and selection highlighting
                let mut lines_with_cursor: Vec<Line<'static>> = Vec::new();
                let editor_lines = editor.lines();

                for (idx, line_content) in editor_lines.iter().enumerate() {
                    // Get the highlighted line if available, otherwise use plain text
                    let base_line = state.edit_highlighted_lines
                        .get(idx)
                        .cloned()
                        .unwrap_or_else(|| Line::from(line_content.clone()));

                    // Apply selection highlighting if this line is in selection range
                    let line_with_selection = if let Some((start_row, start_col, end_row, end_col)) = selection {
                        apply_selection_to_line(&base_line, line_content, idx, start_row, start_col, end_row, end_col)
                    } else {
                        base_line
                    };

                    if idx == cursor_row {
                        // Insert cursor into this line
                        let line_with_cursor = insert_cursor_into_line(&line_with_selection, cursor_col, line_content);
                        lines_with_cursor.push(line_with_cursor);
                    } else {
                        lines_with_cursor.push(line_with_selection);
                    }
                }

                let paragraph = Paragraph::new(lines_with_cursor)
                    .block(block)
                    .scroll((scroll_offset as u16, 0));

                f.render_widget(paragraph, editor_area);

                // Scrollbar
                let content_length = editor_lines.len();
                if content_length > 0 {
                    let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                        .begin_symbol(Some("▲"))
                        .end_symbol(Some("▼"));

                    let mut scrollbar_state = ScrollbarState::new(content_length)
                        .position(scroll_offset);

                    f.render_stateful_widget(scrollbar, editor_area, &mut scrollbar_state);
                }
            }

            // Render MC Edit style shortcut bar
            render_edit_shortcuts(f, shortcut_area, state.block_marking);
        }
        EditorMode::ReadOnly => {
            // Apply selection highlighting if active
            // NOTE: selection_range is in SCREEN coordinates (0 = first visible line)
            // We need to adjust by scroll offset to get content line indices
            let lines = if let Some((start, end)) = selection_range {
                let scroll_offset = state.scroll as usize;
                let adjusted_start = start + scroll_offset;
                let adjusted_end = end + scroll_offset;
                state.highlighted_lines.iter().enumerate().map(|(idx, line)| {
                    if idx >= adjusted_start && idx <= adjusted_end {
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

    // Render search bar at bottom if search is active
    if state.search.active {
        render_search_bar(f, area, state);
    }
}

/// Render the search bar at the bottom of the preview area
fn render_search_bar(f: &mut Frame, area: Rect, state: &PreviewState) {
    // Position search bar at the bottom of the preview area (inside borders)
    let search_area = Rect {
        x: area.x + 1,
        y: area.y + area.height.saturating_sub(2),
        width: area.width.saturating_sub(2),
        height: 1,
    };

    // Build search bar text with match count
    let match_info = if state.search.matches.is_empty() {
        if state.search.query.is_empty() {
            String::new()
        } else {
            " [No matches]".to_string()
        }
    } else {
        format!(
            " [{}/{}]",
            state.search.current_match + 1,
            state.search.matches.len()
        )
    };

    let search_text = format!("/{}{}", state.search.query, match_info);

    // Style: dark gray background, white text, with cursor indicator
    let cursor_indicator = "█";
    let display_text = format!("{}{}", search_text, cursor_indicator);

    let search_widget = Paragraph::new(display_text).style(
        Style::default()
            .fg(Color::White)
            .bg(Color::DarkGray)
            .add_modifier(Modifier::BOLD),
    );

    f.render_widget(search_widget, search_area);
}

/// Insert a cursor (reversed style) into a line at the given column
/// Apply selection highlighting to a line
/// Selection is defined by (start_row, start_col, end_row, end_col)
fn apply_selection_to_line(
    line: &Line<'static>,
    raw_text: &str,
    line_idx: usize,
    start_row: usize,
    start_col: usize,
    end_row: usize,
    end_col: usize,
) -> Line<'static> {
    // Check if this line is within selection range
    if line_idx < start_row || line_idx > end_row {
        return line.clone();
    }

    let selection_style = Style::default().bg(Color::DarkGray);
    let line_len = raw_text.chars().count();

    // Determine selection range for this line
    let (sel_start, sel_end) = if start_row == end_row {
        // Single line selection
        (start_col, end_col)
    } else if line_idx == start_row {
        // First line of multi-line selection
        (start_col, line_len)
    } else if line_idx == end_row {
        // Last line of multi-line selection
        (0, end_col)
    } else {
        // Middle line - entire line selected
        (0, line_len)
    };

    // If nothing to select on this line, return as-is
    if sel_start >= sel_end && line_idx != end_row {
        return line.clone();
    }

    // Build new spans with selection highlighting
    let mut result_spans: Vec<Span<'static>> = Vec::new();
    let mut current_col = 0;

    for span in line.spans.iter() {
        let span_text = span.content.as_ref();
        let span_chars: Vec<char> = span_text.chars().collect();
        let span_len = span_chars.len();
        let span_end = current_col + span_len;

        // Check overlap with selection
        let overlap_start = sel_start.max(current_col);
        let overlap_end = sel_end.min(span_end);

        if overlap_start < overlap_end {
            // This span has some overlap with selection

            // Part before selection (if any)
            if current_col < overlap_start {
                let before_len = overlap_start - current_col;
                let before: String = span_chars[..before_len].iter().collect();
                result_spans.push(Span::styled(before, span.style));
            }

            // Selected part
            let sel_start_in_span = overlap_start.saturating_sub(current_col);
            let sel_end_in_span = (overlap_end - current_col).min(span_len);
            let selected: String = span_chars[sel_start_in_span..sel_end_in_span].iter().collect();
            // Combine existing style with selection background
            let combined_style = span.style.bg(Color::DarkGray);
            result_spans.push(Span::styled(selected, combined_style));

            // Part after selection (if any)
            if overlap_end < span_end {
                let after_start = overlap_end - current_col;
                let after: String = span_chars[after_start..].iter().collect();
                result_spans.push(Span::styled(after, span.style));
            }
        } else {
            // No overlap - keep span as-is
            result_spans.push(span.clone());
        }

        current_col = span_end;
    }

    // If selection extends beyond line content (e.g., trailing newline area)
    if sel_end > current_col && line_idx < end_row {
        // Add visual indicator for selected newline
        result_spans.push(Span::styled(" ", selection_style));
    }

    Line::from(result_spans)
}

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

/// Render MC Edit style shortcut bar at bottom of editor
fn render_edit_shortcuts(f: &mut Frame, area: Rect, block_marking: bool) {
    let shortcuts = vec![
        ("Sh+←→↑↓", "Mark"),
        ("F3", if block_marking { "EndBlk" } else { "Block" }),
        ("F5", "Copy"),
        ("F6", "Move"),
        ("F8", "Del"),
        ("^Y", "DelLn"),
        ("^S", "Save"),
        ("Esc", "Exit"),
    ];

    let mut spans = Vec::new();
    for (key, desc) in shortcuts {
        spans.push(Span::styled(
            format!(" {} ", key),
            Style::default().bg(Color::Cyan).fg(Color::Black),
        ));
        spans.push(Span::styled(
            format!("{} ", desc),
            Style::default().bg(Color::Blue).fg(Color::White),
        ));
    }

    let paragraph = Paragraph::new(Line::from(spans))
        .style(Style::default().bg(Color::Blue));
    f.render_widget(paragraph, area);
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
        // Add block marking indicator (MC style)
        if state.block_marking {
            title.push_str(" [BLOCK]");
        }
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
