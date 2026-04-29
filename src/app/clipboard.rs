use crate::clipboard::ClipboardOutcome;
use crate::types::{EditorMode, PaneId};
use crate::ui;

use super::App;

impl App {
    pub(super) fn copy_selection_to_claude(&mut self) {
        use crate::filter::{filter_lines, FilterOptions};

        let Some((start, end)) = self.terminal_selection.line_range() else {
            return;
        };

        let Some(source_pane) = self.terminal_selection.source_pane else {
            return;
        };

        // Extract lines from source (terminal or preview)
        let lines = if source_pane == PaneId::Preview {
            // Extract lines from preview content
            let content_lines: Vec<String> =
                self.preview.content.lines().map(String::from).collect();
            if start > content_lines.len() || end > content_lines.len() {
                return;
            }
            content_lines[start..=end.min(content_lines.len().saturating_sub(1))].to_vec()
        } else if let Some(pty) = self.terminals.get(&source_pane) {
            pty.extract_lines(start, end)
        } else {
            return;
        };

        if lines.is_empty() {
            return;
        }

        // For preview, use syntax from file; for terminal, detect syntax
        let (formatted_lines, syntax_hint) = if source_pane == PaneId::Preview {
            // Use file extension for syntax hint
            let syntax = self.preview.syntax_name.as_deref().unwrap_or("");
            (lines, Some(syntax.to_lowercase()))
        } else {
            // Apply intelligent filtering for terminal output
            let filtered = filter_lines(lines, &FilterOptions::default());
            if filtered.lines.is_empty() {
                return;
            }
            (filtered.lines, filtered.syntax_hint)
        };

        // Format for Claude - wrap in markdown code block with syntax hint
        let syntax = syntax_hint.as_deref().unwrap_or("");
        let formatted = format!("```{}\n{}\n```\n", syntax, formatted_lines.join("\n"));

        // Send to Claude PTY
        if let Some(claude_pty) = self.terminals.get_mut(&PaneId::Claude) {
            let _ = claude_pty.write_input(formatted.as_bytes());
        }
    }

    /// Copy selected lines to system clipboard (from terminal or preview)
    pub(super) fn copy_selection_to_clipboard(&mut self) {
        let Some((start, end)) = self.terminal_selection.line_range() else {
            return;
        };

        let Some(source_pane) = self.terminal_selection.source_pane else {
            return;
        };

        // Extract lines from source (terminal or preview)
        let lines = if source_pane == PaneId::Preview {
            // Extract lines from preview content
            let content_lines: Vec<String> =
                self.preview.content.lines().map(String::from).collect();
            if start > content_lines.len() || end > content_lines.len() {
                return;
            }
            content_lines[start..=end.min(content_lines.len().saturating_sub(1))].to_vec()
        } else if let Some(pty) = self.terminals.get(&source_pane) {
            pty.extract_lines(start, end)
        } else {
            return;
        };

        if lines.is_empty() {
            return;
        }

        // Join lines and copy to system clipboard
        let text = lines.join("\n");

        let outcome = crate::clipboard::copy_to_clipboard(&text);
        self.handle_copy_outcome(outcome);
    }

    /// Copy the last N lines from the active terminal pane to the system clipboard.
    /// N is configured via config.pty.copy_lines_count (default: 50).
    /// Sets copy flash state for footer indicator.
    pub(super) fn copy_last_lines_to_clipboard(&mut self) {
        let count = self.config.pty.copy_lines_count;
        self.copy_last_lines_to_clipboard_n(count);
    }

    pub(super) fn copy_last_lines_to_clipboard_n(&mut self, count: usize) {
        let pane = self.active_pane;
        if let Some(pty) = self.terminals.get(&pane) {
            let lines: Vec<String> = pty
                .extract_last_n_lines(count)
                .into_iter()
                .filter(|l| !l.is_empty())
                .collect();
            if !lines.is_empty() {
                let text = lines.join("\n");
                let outcome = crate::clipboard::copy_to_clipboard(&text);
                if outcome.is_success() {
                    self.last_copy_time = Some(std::time::Instant::now());
                    self.copy_flash_lines = lines.len();
                }
                self.handle_copy_outcome(outcome);
            }
        }
    }

    /// Copy character-level mouse selection to system clipboard
    pub(super) fn copy_mouse_selection_to_clipboard(&mut self) {
        let Some(((start_row, start_col), (end_row, end_col))) = self.mouse_selection.char_range()
        else {
            return;
        };

        let Some(source_pane) = self.mouse_selection.source_pane else {
            return;
        };

        // Extract text based on source pane type
        let text = if source_pane == PaneId::Preview {
            // Extract from preview content with character-level selection
            let content_lines: Vec<&str> = self.preview.content.lines().collect();

            // (C) Scroll offset: char_range() returns screen-relative rows
            let scroll_offset = self.preview.scroll as usize;
            let adj_start_row = start_row + scroll_offset;
            let adj_end_row = end_row + scroll_offset;

            if adj_start_row >= content_lines.len() {
                return;
            }

            // (B) Subtract gutter width from column indices
            let gutter_w = ui::preview::calculate_gutter_width(content_lines.len()) as usize;
            let base_start_col = start_col.saturating_sub(gutter_w);
            let base_end_col = end_col.saturating_sub(gutter_w);

            // (D) Add horizontal scroll offset
            let h_scroll = self.preview.horizontal_scroll as usize;
            let adj_start_col = base_start_col + h_scroll;
            let adj_end_col = base_end_col + h_scroll;

            let mut result = String::new();
            #[allow(clippy::needless_range_loop)] // row index needed for start/end column logic
            for row in adj_start_row..=adj_end_row.min(content_lines.len().saturating_sub(1)) {
                let line = content_lines[row];
                let line_chars: Vec<char> = line.chars().collect();

                let col_start = if row == adj_start_row {
                    adj_start_col.min(line_chars.len())
                } else {
                    0
                };
                let col_end = if row == adj_end_row {
                    (adj_end_col + 1).min(line_chars.len()) // +1 because end_col is inclusive
                } else {
                    line_chars.len()
                };

                let selected: String = line_chars[col_start..col_end].iter().collect();
                // (A) No trim_end() — preserve original content including spaces
                result.push_str(&selected);
                if row != adj_end_row {
                    result.push('\n');
                }
            }
            result
        } else if let Some(pty) = self.terminals.get(&source_pane) {
            // Extract from terminal with character-level selection
            pty.extract_char_range(start_row, start_col, end_row, end_col + 1) // +1 for inclusive end
        } else {
            return;
        };

        if text.is_empty() {
            return;
        }

        // Copy to system clipboard
        let outcome = crate::clipboard::copy_to_clipboard(&text);
        self.handle_copy_outcome(outcome);
    }

    /// Set an error flash on the footer if the copy failed.
    /// Successful copies are already reflected by `last_copy_time`/`copy_flash_lines`.
    pub(super) fn handle_copy_outcome(&mut self, outcome: ClipboardOutcome) {
        if let ClipboardOutcome::Failed(reason) = outcome {
            self.set_clipboard_error_flash(format!("Clipboard error: {}", reason));
        }
    }

    /// Read text from the system clipboard (with fallback chain) and inject
    /// it into the active pane. F11 binding — bypasses Kitty's bracketed-paste
    /// bridge entirely, which is the key workaround for XRDP sessions where
    /// Kitty cannot read the system clipboard.
    pub(super) fn paste_from_clipboard_to_active_pane(&mut self) {
        let Some(text) = crate::clipboard::paste_from_clipboard() else {
            self.set_clipboard_error_flash("Clipboard empty or unavailable".to_string());
            return;
        };

        match self.active_pane {
            PaneId::Claude => {
                // Claude CLI doesn't understand bracketed paste sequences.
                // Send raw bytes — for multiline, the user must use \ continuation.
                if let Some(pty) = self.terminals.get_mut(&PaneId::Claude) {
                    let _ = pty.write_input(text.as_bytes());
                }
            }
            PaneId::LazyGit | PaneId::Terminal => {
                if let Some(pty) = self.terminals.get_mut(&self.active_pane) {
                    let bracketed = format!("\x1b[200~{}\x1b[201~", text);
                    let _ = pty.write_input(bracketed.as_bytes());
                }
            }
            PaneId::Preview => {
                if self.preview.mode == EditorMode::Edit {
                    if let Some(editor) = &mut self.preview.editor {
                        editor.insert_str(&text);
                        self.preview.update_modified();
                        self.preview.update_edit_highlighting(&self.syntax_manager);
                    } else {
                        self.set_clipboard_error_flash("Preview editor unavailable".to_string());
                    }
                } else {
                    self.set_clipboard_error_flash(
                        "Preview is read-only — press i to enter edit mode".to_string(),
                    );
                }
            }
            PaneId::FileBrowser => {
                self.set_clipboard_error_flash(
                    "F11 paste only works in PTY or Edit panes".to_string(),
                );
            }
        }
    }

    /// Set the clipboard error flash (visible in the footer for ~3 s).
    pub(super) fn set_clipboard_error_flash(&mut self, message: String) {
        self.clipboard_error_flash = Some((message, std::time::Instant::now()));
    }
}
