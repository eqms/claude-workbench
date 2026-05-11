//! Preview-pane key handling — search/replace overlay, edit-mode bindings
//! (Ctrl+S save, F-keys block ops, autosave toggle, MC-style edit), and
//! read-only navigation (j/k/h/l, page, search jump, terminal-selection).

use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};

use crate::types::{EditorMode, PaneId, SearchMode};
use crate::ui;

use super::super::App;

impl App {
    pub(super) fn handle_preview_pane_key(&mut self, key: KeyEvent) {
        // Search/Replace mode handling (priority over other modes)
        if self.preview.search.active {
            match key.code {
                KeyCode::Esc => {
                    self.preview.search.close();
                    return;
                }
                KeyCode::Char('h') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    if self.preview.mode == EditorMode::Edit {
                        self.preview.search.toggle_replace_mode();
                    }
                    return;
                }
                KeyCode::Tab => {
                    self.preview.search.toggle_field_focus();
                    return;
                }
                KeyCode::Char('i') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    self.preview.search.case_sensitive = !self.preview.search.case_sensitive;
                    self.preview.perform_search();
                    return;
                }
                KeyCode::Char('\x09') => {
                    self.preview.search.case_sensitive = !self.preview.search.case_sensitive;
                    self.preview.perform_search();
                    return;
                }
                KeyCode::Enter => {
                    if self.preview.search.mode == SearchMode::Replace
                        && self.preview.mode == EditorMode::Edit
                    {
                        self.preview.replace_and_next(&self.syntax_manager);
                    } else {
                        self.preview.jump_to_current_match();
                        self.preview.search.active = false;
                    }
                    return;
                }
                KeyCode::Char('r')
                    if key.modifiers.contains(KeyModifiers::CONTROL)
                        && self.preview.search.mode == SearchMode::Replace =>
                {
                    if self.preview.mode == EditorMode::Edit {
                        let _count = self.preview.replace_all(&self.syntax_manager);
                    }
                    return;
                }
                KeyCode::Char('n') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    self.preview.search.next_match();
                    self.preview.jump_to_current_match();
                    return;
                }
                KeyCode::Char('p') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    self.preview.search.prev_match();
                    self.preview.jump_to_current_match();
                    return;
                }
                KeyCode::Left => {
                    self.preview.search.cursor_left();
                    return;
                }
                KeyCode::Right => {
                    self.preview.search.cursor_right();
                    return;
                }
                KeyCode::Home => {
                    self.preview.search.cursor_home();
                    return;
                }
                KeyCode::End => {
                    self.preview.search.cursor_end();
                    return;
                }
                KeyCode::Delete => {
                    self.preview.search.delete_char_at();
                    if !self.preview.search.focus_on_replace {
                        self.preview.perform_search();
                        self.preview.jump_to_current_match();
                    }
                    return;
                }
                KeyCode::Backspace => {
                    self.preview.search.delete_char_before();
                    if !self.preview.search.focus_on_replace {
                        self.preview.perform_search();
                        self.preview.jump_to_current_match();
                    }
                    return;
                }
                KeyCode::Char(c) => {
                    self.preview.search.insert_char(c);
                    if !self.preview.search.focus_on_replace {
                        self.preview.perform_search();
                        self.preview.jump_to_current_match();
                    }
                    return;
                }
                _ => {
                    return;
                }
            }
        }

        // Check for search trigger (/ in read-only, Ctrl+F in any mode)
        let is_ctrl_f = (key.code == KeyCode::Char('f')
            && key.modifiers.contains(KeyModifiers::CONTROL))
            || key.code == KeyCode::Char('\x06');
        let is_slash = key.code == KeyCode::Char('/') && self.preview.mode == EditorMode::ReadOnly;
        let is_ctrl_h = (key.code == KeyCode::Char('h')
            && key.modifiers.contains(KeyModifiers::CONTROL))
            || key.code == KeyCode::Char('\x08');

        if is_ctrl_f || is_slash {
            self.preview.search.open();
            return;
        }

        if is_ctrl_h && self.preview.mode == EditorMode::Edit {
            self.preview.search.open();
            self.preview.search.mode = SearchMode::Replace;
            return;
        }

        if self.preview.mode == EditorMode::Edit {
            self.handle_preview_edit_key(key);
        } else {
            self.handle_preview_readonly_key(key);
        }
    }

    fn handle_preview_edit_key(&mut self, key: KeyEvent) {
        let is_ctrl_s = (key.code == KeyCode::Char('s')
            && key.modifiers.contains(KeyModifiers::CONTROL))
            || key.code == KeyCode::Char('\x13');
        let is_ctrl_y = (key.code == KeyCode::Char('y')
            && key.modifiers.contains(KeyModifiers::CONTROL))
            || key.code == KeyCode::Char('\x19');

        if key.code == KeyCode::Esc {
            if self.preview.block_marking {
                self.preview.cancel_selection();
            } else if self.preview.is_modified() {
                if self.config.ui.autosave {
                    let _ = self.preview.save();
                    self.last_autosave_time = Some(std::time::Instant::now());
                    self.preview.exit_edit_mode(false);
                    self.preview.refresh_highlighting(&self.syntax_manager);
                } else {
                    self.dialog.dialog_type = ui::dialog::DialogType::Confirm {
                        title: "Unsaved Changes".to_string(),
                        message: "Discard changes?".to_string(),
                        action: ui::dialog::DialogAction::DiscardEditorChanges,
                    };
                }
            } else {
                self.preview.exit_edit_mode(true);
            }
        } else if is_ctrl_s {
            if self.preview.save().is_ok() {
                self.preview.refresh_highlighting(&self.syntax_manager);
            }
        } else if key.code == KeyCode::Char('\x01')
            || (key.code == KeyCode::Char('a') && key.modifiers.contains(KeyModifiers::CONTROL))
        {
            // Ctrl+A: toggle autosave
            self.config.ui.autosave = !self.config.ui.autosave;
            let _ = crate::config::save_config(&self.config);
        } else if is_ctrl_y {
            self.preview.delete_line();
            self.preview.update_modified();
            self.preview.update_edit_highlighting(&self.syntax_manager);
        } else if key.code == KeyCode::Char('c')
            && (key.modifiers.contains(KeyModifiers::SUPER)
                || key.modifiers.contains(KeyModifiers::CONTROL))
        {
            self.preview.copy_block();
        } else if key.code == KeyCode::Char('x')
            && (key.modifiers.contains(KeyModifiers::SUPER)
                || key.modifiers.contains(KeyModifiers::CONTROL))
        {
            self.preview.move_block();
            self.preview.update_modified();
            self.preview.update_edit_highlighting(&self.syntax_manager);
        } else if key.code == KeyCode::Char('v')
            && (key.modifiers.contains(KeyModifiers::SUPER)
                || key.modifiers.contains(KeyModifiers::CONTROL))
        {
            self.preview.paste_from_clipboard();
            self.preview.update_modified();
            self.preview.update_edit_highlighting(&self.syntax_manager);
        } else if key.code == KeyCode::F(3) && key.modifiers.contains(KeyModifiers::CONTROL) {
            self.preview.toggle_block_marking();
        } else if key.code == KeyCode::F(5) && key.modifiers.contains(KeyModifiers::CONTROL) {
            self.preview.copy_block();
            self.preview.update_modified();
            self.preview.update_edit_highlighting(&self.syntax_manager);
        } else if key.code == KeyCode::F(6) && key.modifiers.contains(KeyModifiers::CONTROL) {
            self.preview.move_block();
            self.preview.update_modified();
            self.preview.update_edit_highlighting(&self.syntax_manager);
        } else if key.code == KeyCode::F(8) && key.modifiers.contains(KeyModifiers::CONTROL) {
            self.preview.delete_block();
            self.preview.update_modified();
            self.preview.update_edit_highlighting(&self.syntax_manager);
        } else if key.modifiers.contains(KeyModifiers::SHIFT) {
            use tui_textarea::CursorMove;
            match key.code {
                KeyCode::Up => {
                    self.preview.extend_selection(CursorMove::Up);
                }
                KeyCode::Down => {
                    self.preview.extend_selection(CursorMove::Down);
                }
                KeyCode::Left => {
                    self.preview.extend_selection(CursorMove::Back);
                }
                KeyCode::Right => {
                    self.preview.extend_selection(CursorMove::Forward);
                }
                _ => {
                    if let Some(editor) = &mut self.preview.editor {
                        editor.input(Event::Key(key));
                        self.preview.update_modified();
                        self.preview.update_edit_highlighting(&self.syntax_manager);
                    }
                }
            }
        } else {
            match key.code {
                KeyCode::PageUp => {
                    if let Some(editor) = &mut self.preview.editor {
                        for _ in 0..20 {
                            editor.move_cursor(tui_textarea::CursorMove::Up);
                        }
                    }
                }
                KeyCode::PageDown => {
                    if let Some(editor) = &mut self.preview.editor {
                        for _ in 0..20 {
                            editor.move_cursor(tui_textarea::CursorMove::Down);
                        }
                    }
                }
                _ => {
                    if let Some(editor) = &mut self.preview.editor {
                        editor.input(Event::Key(key));
                        self.preview.update_modified();
                        self.preview.update_edit_highlighting(&self.syntax_manager);
                    }
                    if let Some(editor) = &self.preview.editor {
                        let (_, cursor_col) = editor.cursor();
                        let visible_width = self.preview_width as usize;
                        let h_scroll = self.preview.horizontal_scroll as usize;
                        if visible_width > 0
                            && cursor_col >= h_scroll + visible_width.saturating_sub(5)
                        {
                            self.preview.horizontal_scroll =
                                (cursor_col.saturating_sub(visible_width / 2)) as u16;
                        } else if cursor_col < h_scroll {
                            self.preview.horizontal_scroll = cursor_col.saturating_sub(5) as u16;
                        }
                    }
                }
            }
        }
    }

    fn handle_preview_readonly_key(&mut self, key: KeyEvent) {
        if self.terminal_selection.active
            && self.terminal_selection.source_pane == Some(PaneId::Preview)
        {
            match key.code {
                KeyCode::Up | KeyCode::Char('k') => {
                    if let Some(end) = self.terminal_selection.end_line {
                        self.terminal_selection.extend(end.saturating_sub(1));
                    }
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if let Some(end) = self.terminal_selection.end_line {
                        self.terminal_selection.extend(end + 1);
                    }
                }
                KeyCode::Enter | KeyCode::Char('y') => {
                    self.copy_selection_to_claude();
                    self.terminal_selection.clear();
                }
                KeyCode::Char('c')
                    if key.modifiers.contains(KeyModifiers::CONTROL)
                        || key.modifiers.contains(KeyModifiers::SUPER) =>
                {
                    self.copy_selection_to_clipboard();
                    self.terminal_selection.clear();
                }
                KeyCode::Esc => {
                    self.terminal_selection.clear();
                }
                _ => {}
            }
            return;
        }

        let is_ctrl_s = (key.code == KeyCode::Char('s')
            && key.modifiers.contains(KeyModifiers::CONTROL))
            || key.code == KeyCode::Char('\x13');
        if is_ctrl_s {
            self.terminal_selection
                .start(self.preview.scroll as usize, PaneId::Preview);
            return;
        }

        let is_ctrl_a = key.code == KeyCode::Char('\x01')
            || (key.code == KeyCode::Char('a') && key.modifiers.contains(KeyModifiers::CONTROL));
        if is_ctrl_a {
            self.config.ui.autosave = !self.config.ui.autosave;
            let _ = crate::config::save_config(&self.config);
            return;
        }

        match key.code {
            KeyCode::Down | KeyCode::Char('j') => self.preview.scroll_down(),
            KeyCode::Up | KeyCode::Char('k') => self.preview.scroll_up(),
            KeyCode::Left | KeyCode::Char('h') => {
                self.preview.scroll_left();
            }
            KeyCode::Right | KeyCode::Char('l') => {
                let max = self.preview.max_line_width();
                self.preview.scroll_right(max);
            }
            KeyCode::PageDown => {
                for _ in 0..10 {
                    self.preview.scroll_down();
                }
            }
            KeyCode::PageUp => {
                for _ in 0..10 {
                    self.preview.scroll_up();
                }
            }
            KeyCode::Home => {
                self.preview.scroll = 0;
            }
            KeyCode::End => {
                let max = self.preview.highlighted_lines.len().saturating_sub(1) as u16;
                self.preview.scroll = max;
            }
            KeyCode::Char('e') | KeyCode::Char('E') => {
                self.preview.enter_edit_mode();
            }
            KeyCode::Char('n') if !self.preview.search.matches.is_empty() => {
                self.preview.search.next_match();
                self.preview.jump_to_current_match();
            }
            KeyCode::Char('N') if !self.preview.search.matches.is_empty() => {
                self.preview.search.prev_match();
                self.preview.jump_to_current_match();
            }
            _ => {}
        }
    }
}
