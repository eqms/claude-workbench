//! Keyboard event dispatch.
//!
//! Splits per-context into submodules:
//! - [`dialogs`] — overlay key handlers (fuzzy finder, update, dialogs, menu, about, help, permission mode, claude startup)
//! - [`global`] — keys that fire regardless of active pane (F12/F10/F7/F9/F11, Ctrl+P/O/X/E, F8, Ctrl+Shift+W)
//! - [`preview`]    — preview-pane handler (search, edit mode, read-only)
//! - [`terminal`]    — terminal-pane handler (Claude/LazyGit/User)
//! - [`file_browser`] — file-browser pane handler
//!
//! `handle_key_event` is the single entry point invoked by `App::run`.

mod dialogs;
mod file_browser;
mod global;
mod preview;
mod terminal;

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use crate::types::{EditorMode, PaneId};

use super::App;

impl App {
    pub(super) fn handle_key_event(&mut self, key: KeyEvent) {
        if key.kind != KeyEventKind::Press {
            return;
        }

        // Fuzzy finder handling (highest priority)
        if self.fuzzy_finder.visible {
            self.handle_fuzzy_finder_key(key);
            return;
        }

        // Update dialog handling (high priority)
        if self.update_state.show_dialog {
            self.handle_update_dialog_key(key);
            return;
        }

        // Wizard handling (high priority)
        if self.wizard.visible {
            self.handle_wizard_input(key.code, key.modifiers);
            return;
        }

        // Settings handling (high priority)
        if self.settings.visible {
            self.handle_settings_input(key.code, key.modifiers);
            return;
        }

        // Export format chooser handling
        if self.export_chooser.visible {
            self.handle_export_chooser_key(key);
            return;
        }

        // Dialog handling (highest priority)
        if self.dialog.is_active() {
            self.handle_active_dialog_key(key);
            return;
        }

        // Menu handling
        if self.menu.visible {
            self.handle_menu_key(key);
            return;
        }

        // About dialog handling
        if self.about.visible {
            self.handle_about_key(key);
            return;
        }

        if self.help.visible {
            self.handle_help_key(key);
            return;
        }

        // XRDP-defensive: Esc cancels a stuck mouse selection.
        // Under XRDP/Kitty, ButtonRelease events for the left mouse button
        // can be dropped by the RDP transport, leaving `selecting` true and
        // the highlight visually frozen. Esc provides a manual escape hatch.
        if key.code == KeyCode::Esc && self.mouse_selection.selecting {
            self.mouse_selection.clear();
            return;
        }

        // Terminal-pane passthrough (tmux-style prefix). When the User Terminal
        // is focused and a prefix is configured, keys go straight to the PTY so
        // TUI apps (nano, mc, vim) work; Workbench commands are reached via the
        // Ctrl+B prefix, and Ctrl+Q stays reserved as a guaranteed quit. Skipped
        // while a Claude permission/startup overlay is up so those keep control.
        if self.active_pane == PaneId::Terminal
            && !self.permission_mode_dialog.visible
            && !self.claude_startup.visible
        {
            if let Some(prefix) = self.config.pty.prefix_key() {
                if self.handle_terminal_passthrough(key, prefix) {
                    return;
                }
            }
        }

        // Global shortcuts (F12/F10/F7/?/F9 variants, Ctrl+P/O/X/E, F8, Ctrl+Shift+W)
        if self.handle_global_shortcut(key) {
            return;
        }

        // Permission mode dialog handling (high priority - before Claude startup)
        // Skip if update dialog is visible - update takes priority
        if self.permission_mode_dialog.visible && !self.update_state.show_dialog {
            self.handle_permission_mode_dialog_key(key);
            return;
        }

        // Claude startup dialog handling (high priority)
        if self.claude_startup.visible {
            self.handle_claude_startup_key(key);
            return;
        }

        // Interactive pane resizing: Alt+Shift+Arrow
        if key
            .modifiers
            .contains(KeyModifiers::ALT | KeyModifiers::SHIFT)
            && self.handle_pane_resize_key(key)
        {
            return;
        }

        // Global Focus Switching
        match key.code {
            KeyCode::F(1) => self.toggle_file_browser(),
            KeyCode::F(2) => self.toggle_preview(),
            KeyCode::F(3) => self.toggle_preview_maximize(),
            KeyCode::F(4) => self.focus_claude(),
            KeyCode::F(5) => self.toggle_lazygit(),
            KeyCode::F(6) => self.toggle_terminal(),
            // QUIT: Ctrl+Q only (Ctrl+C goes to PTY for SIGINT)
            KeyCode::Char('q')
                if key
                    .modifiers
                    .contains(crossterm::event::KeyModifiers::CONTROL) =>
            {
                self.should_quit = true;
            }
            _ => {
                // Pane specific handling
                match self.active_pane {
                    PaneId::FileBrowser => self.handle_file_browser_pane_key(key),

                    PaneId::Preview => self.handle_preview_pane_key(key),
                    PaneId::Terminal | PaneId::Claude | PaneId::LazyGit => {
                        self.handle_terminal_pane_key(key)
                    }
                }
            }
        }
    }

    pub(super) fn handle_paste_event(&mut self, text: String) {
        match self.active_pane {
            PaneId::Claude => {
                // Claude CLI doesn't understand bracketed paste sequences
                // Send text directly - for multiline, user must use \ continuation
                if let Some(pty) = self.terminals.get_mut(&PaneId::Claude) {
                    let _ = pty.write_input(text.as_bytes());
                }
            }
            PaneId::LazyGit | PaneId::Terminal => {
                if let Some(pty) = self.terminals.get_mut(&self.active_pane) {
                    // Wrap in bracketed paste escape sequences
                    // \x1b[200~ = start paste, \x1b[201~ = end paste
                    let bracketed = format!("\x1b[200~{}\x1b[201~", text);
                    let _ = pty.write_input(bracketed.as_bytes());
                }
            }
            PaneId::Preview => {
                // Forward paste to editor in edit mode
                if self.preview.mode == EditorMode::Edit {
                    if let Some(editor) = &mut self.preview.editor {
                        editor.insert_str(crate::clipboard::sanitize_pasted_text(&text));
                        self.preview.update_modified();
                        self.preview.update_edit_highlighting(&self.syntax_manager);
                    }
                }
            }
            PaneId::FileBrowser => {
                // Ignore paste in file browser
            }
        }
    }

    /// Returns true if the resize key was consumed.
    fn handle_pane_resize_key(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Left => {
                self.config.layout.file_browser_width_percent = self
                    .config
                    .layout
                    .file_browser_width_percent
                    .saturating_sub(2)
                    .max(10);
                let _ = crate::config::save_config(&self.config);
                true
            }
            KeyCode::Right => {
                self.config.layout.file_browser_width_percent =
                    (self.config.layout.file_browser_width_percent + 2).min(50);
                let _ = crate::config::save_config(&self.config);
                true
            }
            KeyCode::Up => {
                self.config.layout.claude_height_percent = self
                    .config
                    .layout
                    .claude_height_percent
                    .saturating_sub(2)
                    .max(20);
                let _ = crate::config::save_config(&self.config);
                true
            }
            KeyCode::Down => {
                self.config.layout.claude_height_percent =
                    (self.config.layout.claude_height_percent + 2).min(80);
                let _ = crate::config::save_config(&self.config);
                true
            }
            _ => false,
        }
    }

    // --- Pane focus / visibility (shared by F-keys and the terminal prefix) ---

    /// F1 / `Ctrl+B 1` — toggle the file browser.
    pub(super) fn toggle_file_browser(&mut self) {
        if self.preview_maximized {
            self.preview_maximized = false;
        }
        self.show_file_browser = !self.show_file_browser;
        self.config.ui.show_file_browser = self.show_file_browser;
        let _ = crate::config::save_config(&self.config);
        if self.show_file_browser {
            self.active_pane = PaneId::FileBrowser;
        } else if self.active_pane == PaneId::FileBrowser {
            self.active_pane = PaneId::Claude;
        }
    }

    /// F2 / `Ctrl+B 2` — toggle the preview pane.
    pub(super) fn toggle_preview(&mut self) {
        if self.preview_maximized {
            // Exit maximize mode first, restore layout
            self.show_file_browser = self.preview_saved_layout.show_file_browser;
            self.show_lazygit = self.preview_saved_layout.show_lazygit;
            self.show_terminal = self.preview_saved_layout.show_terminal;
            self.preview_maximized = false;
        }
        self.show_preview = !self.show_preview;
        self.config.ui.show_preview = self.show_preview;
        let _ = crate::config::save_config(&self.config);
        if self.show_preview {
            self.active_pane = PaneId::Preview;
        } else if self.active_pane == PaneId::Preview {
            self.active_pane = PaneId::FileBrowser;
        }
    }

    /// F4 / `Ctrl+B 4` — focus the Claude pane (or its startup dialog).
    pub(super) fn focus_claude(&mut self) {
        // Show startup dialog if prefixes configured and not yet shown
        if !self.claude_startup.shown_this_session
            && !self.config.claude.startup_prefixes.is_empty()
        {
            self.claude_startup
                .open(self.config.claude.startup_prefixes.clone());
        } else {
            self.active_pane = PaneId::Claude;
        }
    }

    /// F5 / `Ctrl+B 5` — toggle the LazyGit pane.
    pub(super) fn toggle_lazygit(&mut self) {
        if self.preview_maximized {
            self.preview_maximized = false;
        }
        let was_hidden = !self.show_lazygit;
        self.show_lazygit = !self.show_lazygit;
        self.config.ui.show_lazygit = self.show_lazygit;
        let _ = crate::config::save_config(&self.config);
        if self.show_lazygit {
            self.active_pane = PaneId::LazyGit;
            // Restart LazyGit in current directory when showing
            if was_hidden {
                self.restart_lazygit_in_current_dir();
            }
        } else if self.active_pane == PaneId::LazyGit {
            self.active_pane = PaneId::Preview;
        }
    }

    /// F6 / `Ctrl+B 6` — toggle the User Terminal pane.
    pub(super) fn toggle_terminal(&mut self) {
        if self.preview_maximized {
            self.preview_maximized = false;
        }
        let was_hidden = !self.show_terminal;
        self.show_terminal = !self.show_terminal;
        self.config.ui.show_terminal = self.show_terminal;
        let _ = crate::config::save_config(&self.config);
        if self.show_terminal {
            self.active_pane = PaneId::Terminal;
            // Lazy-init: spawn Terminal PTY on first show (no-op if already up)
            self.ensure_pty_for_pane(PaneId::Terminal);
            // Sync directory when showing terminal
            if was_hidden {
                self.sync_terminal_to_current_dir(PaneId::Terminal);
            }
        } else if self.active_pane == PaneId::Terminal {
            self.active_pane = PaneId::Preview;
        }
    }

    // --- Terminal-pane prefix passthrough ---

    /// Handle a key while the User Terminal is focused and passthrough is on.
    /// Always returns true (the key is fully handled here): keys are forwarded
    /// raw to the PTY, except the Ctrl+B prefix (Workbench commands) and the
    /// reserved Ctrl+Q quit.
    fn handle_terminal_passthrough(&mut self, key: KeyEvent, prefix: char) -> bool {
        // Reserved global quit — always works, even mid-passthrough.
        if key.code == KeyCode::Char('q') && key.modifiers.contains(KeyModifiers::CONTROL) {
            self.should_quit = true;
            self.terminal_prefix_armed = false;
            return true;
        }

        // PTY exited & no auto-restart: Enter restarts; swallow everything else.
        if let Some(pty) = self.terminals.get(&self.active_pane) {
            if pty.has_exited() && !self.config.pty.auto_restart {
                if key.code == KeyCode::Enter {
                    self.restart_single_pty(self.active_pane);
                }
                self.terminal_prefix_armed = false;
                return true;
            }
        }

        // Active selection mode: defer to the terminal handler's selection keys.
        if self.terminal_selection.active
            && self.terminal_selection.source_pane == Some(self.active_pane)
        {
            self.terminal_prefix_armed = false;
            self.handle_terminal_pane_key(key);
            return true;
        }

        // Prefix already armed → interpret this key as a Workbench command.
        if self.terminal_prefix_armed {
            self.terminal_prefix_armed = false;
            if is_ctrl_char(key, prefix) {
                // Ctrl+B Ctrl+B → send one literal prefix byte to the PTY.
                self.forward_key_to_active_pty(key);
            } else {
                self.handle_terminal_prefix_command(key);
            }
            return true;
        }

        // Prefix pressed → arm and wait for the command key.
        if is_ctrl_char(key, prefix) {
            self.terminal_prefix_armed = true;
            return true;
        }

        // Passthrough: local scrollback gesture, else raw bytes to the PTY.
        if self.handle_scrollback_key(key) {
            return true;
        }
        self.forward_key_to_active_pty(key);
        true
    }

    /// Map a `Ctrl+B <key>` command to its Workbench action.
    fn handle_terminal_prefix_command(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('1') => self.toggle_file_browser(),
            KeyCode::Char('2') => self.toggle_preview(),
            KeyCode::Char('3') => self.toggle_preview_maximize(),
            KeyCode::Char('4') => self.focus_claude(),
            KeyCode::Char('5') => self.toggle_lazygit(),
            KeyCode::Char('6') => self.toggle_terminal(),
            KeyCode::Char('?') | KeyCode::Char('h') => self.help.open(),
            KeyCode::Char('s') => {
                if let Some(pty) = self.terminals.get(&self.active_pane) {
                    let cursor_row = pty.cursor_row() as usize;
                    self.terminal_selection.start(cursor_row, self.active_pane);
                }
            }
            KeyCode::Char('c') => self.copy_last_command_output(),
            // Unknown command (incl. Esc) — already disarmed, ignore.
            _ => {}
        }
    }
}

/// True when `key` is Ctrl + `c` (case-insensitive ASCII letter).
fn is_ctrl_char(key: KeyEvent, c: char) -> bool {
    key.modifiers.contains(KeyModifiers::CONTROL)
        && matches!(key.code, KeyCode::Char(k) if k.eq_ignore_ascii_case(&c))
}
