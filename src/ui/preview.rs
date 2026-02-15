use ratatui::{
    layout::{Constraint, Layout},
    prelude::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState},
    Frame,
};
use std::fs;
use std::path::{Path, PathBuf};
use tui_textarea::TextArea;

use crate::types::{EditorMode, SearchMode, SearchState};
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
    pub horizontal_scroll: u16,

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

    // Cached horizontal scrollbar area from last render (for accurate mouse hit testing)
    pub cached_h_scrollbar_area: Option<Rect>,
}

impl Default for PreviewState {
    fn default() -> Self {
        Self {
            current_file: None,
            content: String::new(),
            scroll: 0,
            horizontal_scroll: 0,
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
            cached_h_scrollbar_area: None,
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
        self.horizontal_scroll = 0;
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
            self.last_modified = fs::metadata(&path).and_then(|m| m.modified()).ok();

            // Use tui-markdown for markdown files, syntect for others
            if self.is_markdown {
                // Catch potential panics in tui-markdown library (known bug in 0.3.7)
                // IMPORTANT: The entire conversion must be inside catch_unwind because
                // tui-markdown can panic during iteration over md_text.lines, not just in from_str()
                let content_clone = content.clone();
                let result: Result<Vec<Line<'static>>, _> =
                    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        let md_text = tui_markdown::from_str(&content_clone);
                        // Convert to owned Lines inside the catch_unwind
                        md_text
                            .lines
                            .into_iter()
                            .map(|line| {
                                Line::from(
                                    line.spans
                                        .into_iter()
                                        .map(|span| {
                                            Span::styled(span.content.to_string(), span.style)
                                        })
                                        .collect::<Vec<_>>(),
                                )
                            })
                            .collect()
                    }));

                match result {
                    Ok(lines) => {
                        self.highlighted_lines = lines;
                    }
                    Err(_) => {
                        // Fallback: show raw markdown content when tui-markdown panics
                        self.highlighted_lines = content
                            .lines()
                            .map(|line| Line::from(line.to_string()))
                            .collect();
                    }
                }
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

    pub fn scroll_left(&mut self) {
        self.horizontal_scroll = self.horizontal_scroll.saturating_sub(1);
    }

    pub fn scroll_right(&mut self, max_width: u16) {
        if self.horizontal_scroll < max_width {
            self.horizontal_scroll = self.horizontal_scroll.saturating_add(1);
        }
    }

    /// Calculate the maximum line width across all highlighted lines
    pub fn max_line_width(&self) -> u16 {
        self.highlighted_lines
            .iter()
            .map(|line| {
                line.spans
                    .iter()
                    .map(|s| s.content.chars().count())
                    .sum::<usize>()
            })
            .max()
            .unwrap_or(0) as u16
    }

    /// Calculate the maximum display width (chars count) across all editor lines
    pub fn edit_max_display_width(&self) -> u16 {
        self.editor
            .as_ref()
            .map(|e| {
                e.lines()
                    .iter()
                    .map(|l| l.chars().count())
                    .max()
                    .unwrap_or(0)
            })
            .unwrap_or(0) as u16
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

    /// Jump to current match - moves cursor in Edit mode, scrolls in ReadOnly mode
    pub fn jump_to_current_match(&mut self) {
        let Some((line, start_col, _end_col)) = self.search.get_current_match() else {
            return;
        };

        if self.mode == EditorMode::Edit {
            // In Edit mode: Move cursor to the match position
            if let Some(editor) = &mut self.editor {
                editor.move_cursor(tui_textarea::CursorMove::Jump(
                    line as u16,
                    start_col as u16,
                ));
            }
        } else {
            // In ReadOnly mode: Scroll to show the match line
            self.scroll = line as u16;
        }
    }

    // ============================================================
    // Search & Replace Operations (MC Edit Style)
    // ============================================================

    /// Replace the current match with replacement text
    /// Returns true if replacement was made
    pub fn replace_current(&mut self) -> bool {
        if self.mode != EditorMode::Edit {
            return false;
        }

        let Some((line_idx, start_col, end_col)) = self.search.get_current_match() else {
            return false;
        };

        let Some(editor) = &mut self.editor else {
            return false;
        };

        // Get the line content
        let lines = editor.lines();
        if line_idx >= lines.len() {
            return false;
        }

        let line = &lines[line_idx];
        let chars: Vec<char> = line.chars().collect();

        // Validate bounds
        if start_col > chars.len() || end_col > chars.len() {
            return false;
        }

        // Build new line content
        let before: String = chars[..start_col].iter().collect();
        let after: String = chars[end_col..].iter().collect();
        let new_line = format!("{}{}{}", before, self.search.replace_text, after);

        // Get all lines and rebuild with the modified line
        let mut all_lines: Vec<String> = editor.lines().to_vec();
        all_lines[line_idx] = new_line;

        // Rebuild the editor with modified content
        let cursor_pos = (line_idx, start_col + self.search.replace_text.len());
        let new_editor = tui_textarea::TextArea::new(all_lines);
        self.editor = Some(new_editor);

        // Move cursor to after the replacement
        if let Some(editor) = &mut self.editor {
            editor.move_cursor(tui_textarea::CursorMove::Jump(
                cursor_pos.0 as u16,
                cursor_pos.1 as u16,
            ));
        }

        self.modified = true;
        true
    }

    /// Replace current match and move to next
    pub fn replace_and_next(&mut self, syntax_manager: &SyntaxManager) {
        if self.replace_current() {
            // Update syntax highlighting
            self.update_edit_highlighting(syntax_manager);

            // Re-run search to update matches
            self.perform_search();

            // Jump to current match (index stays same but points to next occurrence)
            self.jump_to_current_match();
        }
    }

    /// Replace all matches
    /// Returns number of replacements made
    pub fn replace_all(&mut self, syntax_manager: &SyntaxManager) -> usize {
        if self.mode != EditorMode::Edit || self.search.matches.is_empty() {
            return 0;
        }

        let Some(editor) = &self.editor else {
            return 0;
        };

        let query = &self.search.query;
        let replacement = &self.search.replace_text;

        if query.is_empty() {
            return 0;
        }

        // Get all lines and perform replacement
        let mut lines: Vec<String> = editor.lines().to_vec();
        let mut total_replacements = 0;

        for line in lines.iter_mut() {
            if self.search.case_sensitive {
                let count = line.matches(query.as_str()).count();
                *line = line.replace(query.as_str(), replacement.as_str());
                total_replacements += count;
            } else {
                // Case-insensitive replacement
                let lower_query = query.to_lowercase();
                let mut new_line = String::new();
                let mut last_end = 0;
                let lower_line = line.to_lowercase();

                while let Some(start) = lower_line[last_end..].find(&lower_query) {
                    let abs_start = last_end + start;
                    new_line.push_str(&line[last_end..abs_start]);
                    new_line.push_str(replacement);
                    last_end = abs_start + query.len();
                    total_replacements += 1;
                }
                new_line.push_str(&line[last_end..]);
                *line = new_line;
            }
        }

        // Rebuild the editor with modified content
        let new_editor = tui_textarea::TextArea::new(lines);
        self.editor = Some(new_editor);

        // Clear matches and update state
        self.search.matches.clear();
        self.search.current_match = 0;
        self.modified = true;
        self.update_edit_highlighting(syntax_manager);

        total_replacements
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
    /// Copies to both tui-textarea internal buffer AND system clipboard
    pub fn copy_block(&mut self) {
        if let Some(editor) = &mut self.editor {
            // First, copy to internal buffer
            editor.copy();

            // Also copy to system clipboard
            // Get the selected text from the yank buffer (which was just populated by editor.copy())
            let yank_text = editor.yank_text();
            if !yank_text.is_empty() {
                if let Ok(mut clipboard) = arboard::Clipboard::new() {
                    let _ = clipboard.set_text(yank_text.to_string());
                }
            }
            // Don't cancel selection after copy - user might want to see what was copied
        }
    }

    /// Move (cut) selection (MC F6)
    /// User should position cursor and paste (Ctrl+V) to complete move
    /// Copies to both tui-textarea internal buffer AND system clipboard
    pub fn move_block(&mut self) {
        if let Some(editor) = &mut self.editor {
            // First copy to clipboard before cutting
            let yank_text = editor.yank_text();
            editor.cut();

            // Copy to system clipboard (use yank_text from before cut, or get it from cut result)
            let cut_text = editor.yank_text();
            let text_to_copy = if !cut_text.is_empty() {
                cut_text.to_string()
            } else if !yank_text.is_empty() {
                yank_text.to_string()
            } else {
                String::new()
            };

            if !text_to_copy.is_empty() {
                if let Ok(mut clipboard) = arboard::Clipboard::new() {
                    let _ = clipboard.set_text(text_to_copy);
                }
            }

            self.block_marking = false;
            self.selection_start = None;
        }
    }

    /// Paste text from system clipboard at cursor position
    pub fn paste_from_clipboard(&mut self) {
        if let Some(editor) = &mut self.editor {
            if let Ok(mut clipboard) = arboard::Clipboard::new() {
                if let Ok(text) = clipboard.get_text() {
                    if !text.is_empty() {
                        editor.insert_str(&text);
                    }
                }
            }
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

/// Calculate the width needed for line number gutter based on total line count
/// Returns total gutter width including separator: " 123 │"
pub(crate) fn calculate_gutter_width(total_lines: usize) -> u16 {
    if total_lines == 0 {
        return 4; // Minimum: " 1 │"
    }
    let digits = ((total_lines as f64).log10().floor() as u16) + 1;
    digits + 3 // " " + digits + " │"
}

/// Render line numbers gutter
fn render_gutter(
    f: &mut Frame,
    gutter_area: Rect,
    total_lines: usize,
    scroll_offset: usize,
    current_line: Option<usize>,
    visible_height: usize,
) {
    let width = gutter_area.width.saturating_sub(2) as usize; // Space for separator "│"

    let mut gutter_lines: Vec<Line<'static>> = Vec::new();

    for visible_row in 0..visible_height {
        let line_number = scroll_offset + visible_row + 1; // 1-based line numbers

        if line_number > total_lines {
            // Empty line beyond content (show tilde like vim)
            let line = Line::from(vec![
                Span::styled(
                    format!("{:>width$} ", "~", width = width),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled("│", Style::default().fg(Color::DarkGray)),
            ]);
            gutter_lines.push(line);
        } else {
            let is_current = current_line.is_some_and(|cl| cl + 1 == line_number);

            let number_style = if is_current {
                // Current line: highlighted (yellow/bold)
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                // Regular line numbers: dimmed
                Style::default().fg(Color::DarkGray)
            };

            let line = Line::from(vec![
                Span::styled(
                    format!("{:>width$} ", line_number, width = width),
                    number_style,
                ),
                Span::styled("│", Style::default().fg(Color::DarkGray)),
            ]);
            gutter_lines.push(line);
        }
    }

    let gutter_paragraph = Paragraph::new(gutter_lines);
    f.render_widget(gutter_paragraph, gutter_area);
}

pub fn render(
    f: &mut Frame,
    area: Rect,
    state: &mut PreviewState,
    is_focused: bool,
    selection_range: Option<(usize, usize)>,
    char_selection: Option<((usize, usize), (usize, usize))>,
) {
    let title = build_title(state);
    let selection_active = selection_range.is_some() || char_selection.is_some();
    let (border_style, border_type) =
        get_border_style(is_focused, state.mode, state.modified, selection_active);

    let block = Block::bordered()
        .title(format!(" {} ", title))
        .border_style(border_style)
        .border_type(border_type);

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
                let editor_lines = editor.lines();
                let total_lines = editor_lines.len();

                // Calculate gutter width and split inner area
                let gutter_width = calculate_gutter_width(total_lines);
                let chunks =
                    Layout::horizontal([Constraint::Length(gutter_width), Constraint::Min(1)])
                        .split(inner);

                let gutter_area = chunks[0];
                let mut content_area = chunks[1];

                // Check if horizontal scrollbar will be needed - reserve space
                let edit_max_width = editor_lines
                    .iter()
                    .map(|l| l.chars().count())
                    .max()
                    .unwrap_or(0);
                let needs_h_scrollbar = edit_max_width > content_area.width as usize;
                if needs_h_scrollbar {
                    content_area.height = content_area.height.saturating_sub(1);
                }

                // Calculate scroll to keep cursor visible
                let visible_height = content_area.height as usize;
                let scroll_offset = if cursor_row >= visible_height {
                    cursor_row.saturating_sub(visible_height / 2)
                } else {
                    0
                };

                // Render the block (border) first
                f.render_widget(block, editor_area);

                // Render line numbers gutter
                render_gutter(
                    f,
                    gutter_area,
                    total_lines,
                    scroll_offset,
                    Some(cursor_row),
                    visible_height,
                );

                // Get selection range for visualization
                let selection = state.get_selection_range();

                // Build lines with cursor and selection highlighting
                let mut lines_with_cursor: Vec<Line<'static>> = Vec::new();

                for (idx, line_content) in editor_lines.iter().enumerate() {
                    // Get the highlighted line if available, otherwise use plain text
                    let base_line = state
                        .edit_highlighted_lines
                        .get(idx)
                        .cloned()
                        .unwrap_or_else(|| Line::from(line_content.clone()));

                    // Apply selection highlighting if this line is in selection range
                    let line_with_selection =
                        if let Some((start_row, start_col, end_row, end_col)) = selection {
                            apply_selection_to_line(
                                &base_line,
                                line_content,
                                idx,
                                start_row,
                                start_col,
                                end_row,
                                end_col,
                            )
                        } else {
                            base_line
                        };

                    if idx == cursor_row {
                        // Insert cursor into this line
                        let line_with_cursor =
                            insert_cursor_into_line(&line_with_selection, cursor_col, line_content);
                        lines_with_cursor.push(line_with_cursor);
                    } else {
                        lines_with_cursor.push(line_with_selection);
                    }
                }

                // Render content without block (block already rendered)
                let paragraph = Paragraph::new(lines_with_cursor)
                    .scroll((scroll_offset as u16, state.horizontal_scroll));

                f.render_widget(paragraph, content_area);

                // Scrollbar styling: green when focused, gray when not
                let sb_color = if is_focused {
                    Color::Green
                } else {
                    Color::Gray
                };
                let sb_style = Style::default().fg(sb_color);

                // Scrollbar
                if total_lines > 0 {
                    let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                        .begin_symbol(Some("▲"))
                        .end_symbol(Some("▼"))
                        .style(sb_style);

                    let mut scrollbar_state =
                        ScrollbarState::new(total_lines).position(scroll_offset);

                    f.render_stateful_widget(scrollbar, editor_area, &mut scrollbar_state);
                }

                // Horizontal scrollbar (rendered in reserved space below content)
                if needs_h_scrollbar {
                    let h_scrollbar_area = Rect::new(
                        content_area.x,
                        content_area.y + content_area.height,
                        content_area.width,
                        1,
                    );
                    state.cached_h_scrollbar_area = Some(h_scrollbar_area);
                    let h_scrollbar = Scrollbar::new(ScrollbarOrientation::HorizontalBottom)
                        .begin_symbol(Some("◄"))
                        .end_symbol(Some("►"))
                        .style(sb_style);
                    let mut h_sb_state = ScrollbarState::new(edit_max_width)
                        .position(state.horizontal_scroll as usize);
                    f.render_stateful_widget(h_scrollbar, h_scrollbar_area, &mut h_sb_state);
                } else {
                    state.cached_h_scrollbar_area = None;
                }
            }

            // Render editor status bar
            render_edit_shortcuts(f, shortcut_area, state.block_marking);
        }
        EditorMode::ReadOnly => {
            let total_lines = state.highlighted_lines.len();
            let scroll_offset = state.scroll as usize;

            // Render block first
            f.render_widget(block.clone(), area);

            // Calculate inner area and split for gutter
            let inner = block.inner(area);
            let gutter_width = calculate_gutter_width(total_lines);
            let chunks = Layout::horizontal([Constraint::Length(gutter_width), Constraint::Min(1)])
                .split(inner);

            let gutter_area = chunks[0];
            let mut content_area = chunks[1];

            // Check if horizontal scrollbar will be needed - reserve space
            let max_width = state.max_line_width() as usize;
            let needs_h_scrollbar = max_width > content_area.width as usize;
            if needs_h_scrollbar {
                content_area.height = content_area.height.saturating_sub(1);
            }
            let visible_height = content_area.height as usize;

            // Render line numbers gutter (no current line in ReadOnly)
            render_gutter(
                f,
                gutter_area,
                total_lines,
                scroll_offset,
                None, // No current line highlighting in ReadOnly mode
                visible_height,
            );

            // Apply selection highlighting if active
            // NOTE: selection_range is in SCREEN coordinates (0 = first visible line)
            // We need to adjust by scroll offset to get content line indices
            let lines = if let Some(((start_row, start_col), (end_row, end_col))) = char_selection {
                // Character-level mouse selection
                let adjusted_start_row = start_row + scroll_offset;
                let adjusted_end_row = end_row + scroll_offset;
                state
                    .highlighted_lines
                    .iter()
                    .enumerate()
                    .map(|(idx, line)| {
                        if idx < adjusted_start_row || idx > adjusted_end_row {
                            return line.clone();
                        }
                        // This line is in selection range - apply char-level highlighting
                        apply_char_selection_to_line(
                            line,
                            idx,
                            adjusted_start_row,
                            start_col,
                            adjusted_end_row,
                            end_col,
                        )
                    })
                    .collect::<Vec<_>>()
            } else if let Some((start, end)) = selection_range {
                // Line-based keyboard selection
                let adjusted_start = start + scroll_offset;
                let adjusted_end = end + scroll_offset;
                state
                    .highlighted_lines
                    .iter()
                    .enumerate()
                    .map(|(idx, line)| {
                        if idx >= adjusted_start && idx <= adjusted_end {
                            // Apply DarkGray background to selected lines
                            let styled_spans: Vec<Span> = line
                                .spans
                                .iter()
                                .map(|span| {
                                    Span::styled(
                                        span.content.clone(),
                                        span.style.bg(Color::DarkGray),
                                    )
                                })
                                .collect();
                            Line::from(styled_spans)
                        } else {
                            line.clone()
                        }
                    })
                    .collect::<Vec<_>>()
            } else {
                state.highlighted_lines.clone()
            };

            // Render highlighted content in read-only mode (without block, already rendered)
            // Note: No wrapping - code should not wrap as it breaks indentation/readability
            // Horizontal scroll enabled via h/l keys and Shift+Scroll
            let paragraph = Paragraph::new(lines).scroll((state.scroll, state.horizontal_scroll));

            f.render_widget(paragraph, content_area);

            // Scrollbar styling: green when focused, gray when not
            let sb_color = if is_focused {
                Color::Green
            } else {
                Color::Gray
            };
            let sb_style = Style::default().fg(sb_color);

            // Vertical scrollbar for read-only mode
            if total_lines > 0 {
                let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                    .begin_symbol(Some("▲"))
                    .end_symbol(Some("▼"))
                    .style(sb_style);

                let mut scrollbar_state = ScrollbarState::new(total_lines).position(scroll_offset);

                f.render_stateful_widget(scrollbar, area, &mut scrollbar_state);
            }

            // Horizontal scrollbar (rendered in reserved space below content)
            if needs_h_scrollbar {
                let h_scrollbar_area = Rect::new(
                    content_area.x,
                    content_area.y + content_area.height,
                    content_area.width,
                    1,
                );
                state.cached_h_scrollbar_area = Some(h_scrollbar_area);
                let h_scrollbar = Scrollbar::new(ScrollbarOrientation::HorizontalBottom)
                    .begin_symbol(Some("◄"))
                    .end_symbol(Some("►"))
                    .style(sb_style);
                let mut h_sb_state =
                    ScrollbarState::new(max_width).position(state.horizontal_scroll as usize);
                f.render_stateful_widget(h_scrollbar, h_scrollbar_area, &mut h_sb_state);
            } else {
                state.cached_h_scrollbar_area = None;
            }
        }
    }

    // Render search bar at bottom if search is active
    if state.search.active {
        render_search_bar(f, area, state);
    }
}

/// Render the search/replace bar at the bottom of the preview area
fn render_search_bar(f: &mut Frame, area: Rect, state: &PreviewState) {
    let is_replace_mode = state.search.mode == SearchMode::Replace;
    let bar_height: u16 = if is_replace_mode { 3 } else { 1 };

    // Position bar at the bottom of the preview area (inside borders)
    let bar_area = Rect {
        x: area.x + 1,
        y: area.y + area.height.saturating_sub(bar_height + 1),
        width: area.width.saturating_sub(2),
        height: bar_height,
    };

    // Clear background for the entire bar area with solid fill
    use ratatui::widgets::Clear;
    f.render_widget(Clear, bar_area);
    let bg_fill = " ".repeat(bar_area.width as usize);
    let mut bg_lines = Vec::new();
    for _ in 0..bar_height {
        bg_lines.push(Line::from(bg_fill.clone()));
    }
    let solid_bg = Paragraph::new(bg_lines).style(Style::default().bg(Color::DarkGray));
    f.render_widget(solid_bg, bar_area);

    // Build match count info
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

    // Case sensitivity indicator
    let case_indicator = if state.search.case_sensitive {
        "[Aa]"
    } else {
        "[aa]"
    };

    // Search line
    let search_line_area = Rect {
        x: bar_area.x,
        y: bar_area.y,
        width: bar_area.width,
        height: 1,
    };

    let search_focused = !state.search.focus_on_replace;
    let search_label = if is_replace_mode { "Find: " } else { "/" };

    let search_style = if search_focused {
        Style::default().fg(Color::Yellow).bg(Color::DarkGray)
    } else {
        Style::default().fg(Color::White).bg(Color::DarkGray)
    };

    // Build search line with cursor at correct position
    let mut search_spans = vec![Span::styled(
        search_label,
        Style::default().fg(Color::Cyan).bg(Color::DarkGray),
    )];

    // Split query at cursor position (UTF-8 safe)
    let cursor_pos = state.search.query_cursor;
    let query_chars: Vec<char> = state.search.query.chars().collect();
    let before_cursor: String = query_chars.iter().take(cursor_pos).collect();
    let after_cursor: String = query_chars.iter().skip(cursor_pos).collect();

    search_spans.push(Span::styled(before_cursor, search_style));

    if search_focused {
        // Show cursor as inverted block
        let cursor_char = if cursor_pos < query_chars.len() {
            query_chars[cursor_pos].to_string()
        } else {
            " ".to_string()
        };
        search_spans.push(Span::styled(
            cursor_char,
            Style::default().fg(Color::Black).bg(Color::Yellow),
        ));
        // Text after cursor (skip the cursor char if within bounds)
        if cursor_pos < query_chars.len() {
            let rest: String = query_chars.iter().skip(cursor_pos + 1).collect();
            search_spans.push(Span::styled(rest, search_style));
        }
    } else {
        search_spans.push(Span::styled(after_cursor, search_style));
    }

    search_spans.push(Span::styled(
        match_info,
        Style::default().fg(Color::Gray).bg(Color::DarkGray),
    ));
    search_spans.push(Span::styled(
        format!(" {}", case_indicator),
        Style::default().fg(Color::DarkGray).bg(Color::DarkGray),
    ));

    let search_line = Line::from(search_spans);

    f.render_widget(Paragraph::new(search_line), search_line_area);

    // Replace line and hints (only in Replace mode)
    if is_replace_mode {
        let replace_line_area = Rect {
            x: bar_area.x,
            y: bar_area.y + 1,
            width: bar_area.width,
            height: 1,
        };

        let replace_focused = state.search.focus_on_replace;

        let replace_style = if replace_focused {
            Style::default().fg(Color::Yellow).bg(Color::DarkGray)
        } else {
            Style::default().fg(Color::White).bg(Color::DarkGray)
        };

        // Build replace line with cursor at correct position
        let mut replace_spans = vec![Span::styled(
            "Repl: ",
            Style::default().fg(Color::Cyan).bg(Color::DarkGray),
        )];

        // Split replace_text at cursor position (UTF-8 safe)
        let replace_cursor_pos = state.search.replace_cursor;
        let replace_chars: Vec<char> = state.search.replace_text.chars().collect();
        let replace_before: String = replace_chars.iter().take(replace_cursor_pos).collect();
        let replace_after: String = replace_chars.iter().skip(replace_cursor_pos).collect();

        replace_spans.push(Span::styled(replace_before, replace_style));

        if replace_focused {
            // Show cursor as inverted block
            let cursor_char = if replace_cursor_pos < replace_chars.len() {
                replace_chars[replace_cursor_pos].to_string()
            } else {
                " ".to_string()
            };
            replace_spans.push(Span::styled(
                cursor_char,
                Style::default().fg(Color::Black).bg(Color::Yellow),
            ));
            // Text after cursor (skip the cursor char if within bounds)
            if replace_cursor_pos < replace_chars.len() {
                let rest: String = replace_chars.iter().skip(replace_cursor_pos + 1).collect();
                replace_spans.push(Span::styled(rest, replace_style));
            }
        } else {
            replace_spans.push(Span::styled(replace_after, replace_style));
        }

        let replace_line = Line::from(replace_spans);

        f.render_widget(Paragraph::new(replace_line), replace_line_area);

        // Shortcut hints line
        let hints_area = Rect {
            x: bar_area.x,
            y: bar_area.y + 2,
            width: bar_area.width,
            height: 1,
        };

        let edit_mode_available = state.mode == EditorMode::Edit;
        let hint_style = if edit_mode_available {
            Style::default().fg(Color::Gray).bg(Color::DarkGray)
        } else {
            Style::default().fg(Color::Red).bg(Color::DarkGray)
        };

        let hints = if edit_mode_available {
            "Tab:Field  Enter:Replace  ^R:All  ^N:Next  ^P:Prev  ^I:Case  Esc:Close"
        } else {
            "[Read-only - press E to edit]  ^N:Next  ^P:Prev  Esc:Close"
        };

        f.render_widget(Paragraph::new(hints).style(hint_style), hints_area);
    }
}

/// Apply character-level mouse selection highlighting to a line (ReadOnly mode)
/// For mouse selection in preview pane - uses LightYellow background for visibility
fn apply_char_selection_to_line(
    line: &Line<'static>,
    line_idx: usize,
    start_row: usize,
    start_col: usize,
    end_row: usize,
    end_col: usize,
) -> Line<'static> {
    let selection_style = Style::default().bg(Color::LightYellow).fg(Color::Black);

    // Flatten the line into a single string to get accurate character positions
    let full_text: String = line.spans.iter().map(|s| s.content.as_ref()).collect();
    let line_chars: Vec<char> = full_text.chars().collect();
    let line_len = line_chars.len();

    // Determine selection range for this line
    let (sel_start, sel_end) = if start_row == end_row {
        // Single line selection
        (start_col, end_col + 1) // +1 to include end_col
    } else if line_idx == start_row {
        // First line of multi-line selection
        (start_col, line_len)
    } else if line_idx == end_row {
        // Last line of multi-line selection
        (0, end_col + 1) // +1 to include end_col
    } else {
        // Middle line - entire line selected
        (0, line_len)
    };

    // If nothing to select, return as-is
    if sel_start >= line_len {
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

        if overlap_start < overlap_end && overlap_start < span_end && overlap_end > current_col {
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
            let selected: String = span_chars[sel_start_in_span..sel_end_in_span]
                .iter()
                .collect();
            result_spans.push(Span::styled(selected, selection_style));

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

    Line::from(result_spans)
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
            let selected: String = span_chars[sel_start_in_span..sel_end_in_span]
                .iter()
                .collect();
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
    let cursor_style = Style::default().add_modifier(Modifier::REVERSED | Modifier::SLOW_BLINK);

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

/// Render editor status bar with platform-aware shortcuts
fn render_edit_shortcuts(f: &mut Frame, area: Rect, block_marking: bool) {
    let shortcuts: Vec<(&str, &str)> = if block_marking {
        vec![
            ("Sh+←→↑↓", "Mark"),
            ("^C", "Copy"),
            ("^X", "Cut"),
            ("^V", "Paste"),
            ("^F3", "EndBlk"),
            ("^F8", "Del"),
            ("^Z", "Undo"),
        ]
    } else {
        vec![
            ("Sh+←→↑↓", "Mark"),
            ("^C", "Copy"),
            ("^X", "Cut"),
            ("^V", "Paste"),
            ("^Z", "Undo"),
            ("^Y", "DelLn"),
            ("^H", "S&R"),
            ("^S", "Save"),
            ("Esc", "Exit"),
        ]
    };

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

    let paragraph = Paragraph::new(Line::from(spans)).style(Style::default().bg(Color::Blue));
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

fn get_border_style(
    is_focused: bool,
    mode: EditorMode,
    modified: bool,
    selection_active: bool,
) -> (Style, BorderType) {
    // Selection mode takes priority (yellow border like terminal panes)
    if selection_active {
        return (Style::default().fg(Color::Yellow), BorderType::Double);
    }
    match (mode, modified, is_focused) {
        // Edit + modified: Yellow + Double
        (EditorMode::Edit, true, _) => (Style::default().fg(Color::Yellow), BorderType::Double),
        // Edit + saved: Cyan + Double
        (EditorMode::Edit, false, _) => (Style::default().fg(Color::Cyan), BorderType::Double),
        // Focused (ReadOnly): Green + Double
        (_, _, true) => (Style::default().fg(Color::Green), BorderType::Double),
        // Default: no style + Rounded
        _ => (Style::default(), BorderType::Rounded),
    }
}
