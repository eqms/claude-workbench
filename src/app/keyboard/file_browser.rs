//! File-browser pane key handling — j/k navigation, h/l directory entry,
//! `o`/`O` external open, `.` toggle hidden, `i` add-to-gitignore,
//! Ctrl+A toggle autosave, `q` quit.

use crossterm::event::KeyCode;

use super::super::App;

impl App {
    pub(super) fn handle_file_browser_pane_key(&mut self, key: crossterm::event::KeyEvent) {
        match key.code {
            KeyCode::Down | KeyCode::Char('j') => {
                self.file_browser.down();
                self.update_preview();
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.file_browser.up();
                self.update_preview();
            }
            KeyCode::Enter | KeyCode::Right | KeyCode::Char('l') => {
                if let Some(_path) = self.file_browser.enter_selected() {
                    // File opened
                } else {
                    self.update_preview();
                    self.sync_terminals();
                    self.check_repo_change();
                }
            }
            KeyCode::Backspace | KeyCode::Left | KeyCode::Char('h') => {
                self.file_browser.go_parent();
                self.update_preview();
                self.sync_terminals();
                self.check_repo_change();
            }
            KeyCode::Char('o') => {
                if let Some(path) = self.file_browser.selected_file() {
                    self.open_in_browser(&path);
                }
            }
            KeyCode::Char('O') => {
                let _ = crate::browser::open_in_file_manager(&self.file_browser.current_dir);
            }
            KeyCode::Char('q') => {
                self.should_quit = true;
            }
            KeyCode::Char('.') => {
                self.file_browser.show_hidden = !self.file_browser.show_hidden;
                self.file_browser.refresh();
                self.update_preview();
            }
            KeyCode::Char('i') => {
                if let Some(path) = self.file_browser.selected_file() {
                    self.add_to_gitignore(&path);
                }
            }
            // Ctrl+A = Toggle Autosave
            KeyCode::Char('\x01') => {
                self.config.ui.autosave = !self.config.ui.autosave;
                let _ = crate::config::save_config(&self.config);
            }
            _ => {}
        }
    }
}
