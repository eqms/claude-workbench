use crossterm::event::{Event, KeyCode, KeyEventKind, KeyModifiers};

use crate::types::{EditorMode, PaneId, SearchMode};
use crate::ui;

use super::App;

impl App {
    pub(super) fn handle_key_event(&mut self, key: crossterm::event::KeyEvent) {
        if key.kind != KeyEventKind::Press {
            return;
        }

        // Fuzzy finder handling (highest priority)
        if self.fuzzy_finder.visible {
            match key.code {
                KeyCode::Esc => self.fuzzy_finder.close(),
                KeyCode::Enter => {
                    if let Some(selected) = self.fuzzy_finder.selected() {
                        let full_path = self.fuzzy_finder.base_dir.join(&selected);
                        // Navigate to file's directory and select it
                        if let Some(parent) = full_path.parent() {
                            self.file_browser.current_dir = parent.to_path_buf();
                            self.file_browser.load_directory();
                            // Try to select the file
                            let file_name = full_path
                                .file_name()
                                .map(|n| n.to_string_lossy().to_string());
                            if let Some(name) = file_name {
                                for (i, entry) in self.file_browser.entries.iter().enumerate() {
                                    if entry.name == name {
                                        self.file_browser.list_state.select(Some(i));
                                        break;
                                    }
                                }
                            }
                            self.update_preview();
                            self.sync_terminals();
                        }
                        self.fuzzy_finder.close();
                    }
                }
                KeyCode::Up => self.fuzzy_finder.prev(),
                KeyCode::Down => self.fuzzy_finder.next(),
                KeyCode::Backspace => self.fuzzy_finder.pop_char(),
                KeyCode::Char(c) => self.fuzzy_finder.push_char(c),
                _ => {}
            }
            return;
        }

        // Update dialog handling (high priority)
        if self.update_state.show_dialog {
            match key.code {
                KeyCode::Esc => {
                    self.update_state.close_dialog();
                }
                KeyCode::Enter => {
                    // Success screen with Restart/Close buttons
                    if self.update_state.update_success {
                        if self.update_dialog_button
                            == crate::ui::update_dialog::UpdateDialogButton::Restart
                        {
                            // Signal restart and exit cleanly
                            self.should_restart = true;
                            self.should_quit = true;
                        } else {
                            self.update_state.close_dialog();
                        }
                    }
                    // Update available with Update/Later buttons
                    else if self.update_state.available_version.is_some()
                        && !self.update_state.updating
                    {
                        if self.update_dialog_button
                            == crate::ui::update_dialog::UpdateDialogButton::Update
                        {
                            self.start_update();
                        } else {
                            self.update_state.close_dialog();
                        }
                    } else {
                        self.update_state.close_dialog();
                    }
                }
                KeyCode::Tab | KeyCode::Left | KeyCode::Right => {
                    // Toggle buttons for update available or success screens
                    if self.update_state.update_success {
                        // Toggle Restart/Close
                        self.update_dialog_button = if self.update_dialog_button
                            == crate::ui::update_dialog::UpdateDialogButton::Restart
                        {
                            crate::ui::update_dialog::UpdateDialogButton::Close
                        } else {
                            crate::ui::update_dialog::UpdateDialogButton::Restart
                        };
                    } else if self.update_state.available_version.is_some()
                        && !self.update_state.updating
                    {
                        // Toggle Update/Later
                        self.update_dialog_button = self.update_dialog_button.toggle();
                    }
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    // Scroll release notes up
                    self.update_state.scroll_release_notes_up();
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    // Scroll release notes down
                    // Max scroll = number of lines - visible area (estimate ~10)
                    let max = self
                        .update_state
                        .release_notes
                        .as_ref()
                        .map(|n| n.lines().count().saturating_sub(10) as u16)
                        .unwrap_or(0);
                    self.update_state.scroll_release_notes_down(max);
                }
                _ => {}
            }
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

        // Dialog handling (highest priority)
        if self.dialog.is_active() {
            match &self.dialog.dialog_type {
                ui::dialog::DialogType::Input { value, action, .. } => {
                    match key.code {
                        KeyCode::Esc => self.dialog.close(),
                        KeyCode::Enter => {
                            let val = value.clone();
                            let act = action.clone();
                            self.dialog.close();
                            self.execute_dialog_action(act, Some(val));
                        }
                        // Tab: Path completion for GoToPath dialog
                        KeyCode::Tab => {
                            if matches!(action, ui::dialog::DialogAction::GoToPath) {
                                self.dialog.try_complete_path();
                            }
                        }
                        KeyCode::Backspace => self.dialog.delete_char_before(),
                        KeyCode::Delete => self.dialog.delete_char_at(),
                        KeyCode::Left => self.dialog.cursor_left(),
                        KeyCode::Right => self.dialog.cursor_right(),
                        KeyCode::Home => self.dialog.cursor_home(),
                        KeyCode::End => self.dialog.cursor_end(),
                        KeyCode::Char(c) => self.dialog.insert_char(c),
                        _ => {}
                    }
                }
                ui::dialog::DialogType::Confirm { action, .. } => match key.code {
                    KeyCode::Esc | KeyCode::Char('n') | KeyCode::Char('N') => self.dialog.close(),
                    KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => {
                        let act = action.clone();
                        self.dialog.close();
                        self.execute_dialog_action(act, None);
                    }
                    _ => {}
                },
                ui::dialog::DialogType::None => {}
            }
            return;
        }

        // Menu handling
        if self.menu.visible {
            match key.code {
                KeyCode::Esc => self.menu.visible = false,
                KeyCode::Up | KeyCode::Char('k') => self.menu.prev(),
                KeyCode::Down | KeyCode::Char('j') => self.menu.next(),
                KeyCode::Enter => {
                    let action = self.menu.action();
                    self.menu.visible = false;
                    self.handle_menu_action(action);
                }
                KeyCode::Char('n') => {
                    self.menu.visible = false;
                    self.handle_menu_action(ui::menu::MenuAction::NewFile);
                }
                KeyCode::Char('N') => {
                    self.menu.visible = false;
                    self.handle_menu_action(ui::menu::MenuAction::NewDirectory);
                }
                KeyCode::Char('r') => {
                    self.menu.visible = false;
                    self.handle_menu_action(ui::menu::MenuAction::RenameFile);
                }
                KeyCode::Char('u') => {
                    self.menu.visible = false;
                    self.handle_menu_action(ui::menu::MenuAction::DuplicateFile);
                }
                KeyCode::Char('c') => {
                    self.menu.visible = false;
                    self.handle_menu_action(ui::menu::MenuAction::CopyFileTo);
                }
                KeyCode::Char('m') => {
                    self.menu.visible = false;
                    self.handle_menu_action(ui::menu::MenuAction::MoveFileTo);
                }
                KeyCode::Char('d') => {
                    self.menu.visible = false;
                    self.handle_menu_action(ui::menu::MenuAction::DeleteFile);
                }
                KeyCode::Char('y') => {
                    self.menu.visible = false;
                    self.handle_menu_action(ui::menu::MenuAction::CopyAbsolutePath);
                }
                KeyCode::Char('Y') => {
                    self.menu.visible = false;
                    self.handle_menu_action(ui::menu::MenuAction::CopyRelativePath);
                }
                KeyCode::Char('g') => {
                    self.menu.visible = false;
                    self.handle_menu_action(ui::menu::MenuAction::GoToPath);
                }
                KeyCode::Char('i') => {
                    self.menu.visible = false;
                    self.handle_menu_action(ui::menu::MenuAction::AddToGitignore);
                }
                _ => {}
            }
            return;
        }

        // About dialog handling
        if self.about.visible {
            match key.code {
                KeyCode::Esc | KeyCode::F(10) | KeyCode::Char('q') => self.about.close(),
                _ => {}
            }
            return;
        }

        if self.help.visible {
            // Search mode active: handle text input
            if self.help.search_active {
                match key.code {
                    KeyCode::Esc => {
                        // Cancel search, keep query visible
                        self.help.stop_search();
                    }
                    KeyCode::Enter => {
                        // Confirm search, navigate results
                        self.help.stop_search();
                        self.help.scroll = 0; // Jump to first match
                    }
                    KeyCode::Backspace => {
                        self.help.search_backspace();
                        self.help.scroll = 0; // Reset scroll on query change
                    }
                    KeyCode::Char('u')
                        if key
                            .modifiers
                            .contains(crossterm::event::KeyModifiers::CONTROL) =>
                    {
                        // Ctrl+U: Clear search
                        self.help.clear_search();
                    }
                    KeyCode::Char(c) => {
                        self.help.search_add_char(c);
                        self.help.scroll = 0; // Reset scroll on query change
                    }
                    _ => {}
                }
            } else {
                // Normal mode: navigation and search activation
                match key.code {
                    KeyCode::Esc | KeyCode::F(12) | KeyCode::Char('q') => self.help.close(),
                    KeyCode::Char('/') | KeyCode::Char('f')
                        if key.code == KeyCode::Char('/')
                            || key
                                .modifiers
                                .contains(crossterm::event::KeyModifiers::CONTROL) =>
                    {
                        // '/' or Ctrl+F: Start search
                        self.help.start_search();
                    }
                    KeyCode::Up | KeyCode::Char('k') => self.help.scroll_up(1),
                    KeyCode::Down | KeyCode::Char('j') => self.help.scroll_down(1),
                    KeyCode::PageUp => self.help.page_up(),
                    KeyCode::PageDown => self.help.page_down(),
                    KeyCode::Home | KeyCode::Char('g') => self.help.scroll_to_top(),
                    KeyCode::End | KeyCode::Char('G') => self.help.scroll_to_bottom(),
                    KeyCode::Char('u') => {
                        // Trigger manual update check from Help screen
                        self.help.close();
                        self.update_state.manual_check = true;
                        self.start_update_check();
                    }
                    _ => {}
                }
            }
            // Consume all keys while help is open
            return;
        }

        // Global Keys - F10/F12 work everywhere
        if key.code == KeyCode::F(12) {
            self.help.open();
            return;
        }

        if key.code == KeyCode::F(10) {
            self.about.open();
            return;
        }

        // F7: Toggle between ~/.claude and previous directory
        if key.code == KeyCode::F(7) {
            if let Some(home) = std::env::var_os("HOME") {
                let claude_dir = std::path::PathBuf::from(home).join(".claude");
                // Already in ~/.claude? → toggle back
                if self.file_browser.current_dir.starts_with(&claude_dir)
                    || self.file_browser.root_dir.starts_with(&claude_dir)
                {
                    if let Some(prev) = self.file_browser.previous_dir.take() {
                        self.file_browser.current_dir = prev;
                        self.file_browser.load_directory();
                        self.active_pane = PaneId::FileBrowser;
                    }
                } else if claude_dir.exists() && claude_dir.is_dir() {
                    // Save current dir, navigate to ~/.claude
                    self.file_browser.previous_dir = Some(self.file_browser.root_dir.clone());
                    self.file_browser.current_dir = claude_dir;
                    self.file_browser.load_directory();
                    self.active_pane = PaneId::FileBrowser;
                }
            }
            return;
        }

        // Context-specific shortcuts (only in non-terminal panes)
        // '?' for help - only in FileBrowser or Preview (read-only)
        if key.code == KeyCode::Char('?')
            && matches!(self.active_pane, PaneId::FileBrowser | PaneId::Preview)
            && self.preview.mode != EditorMode::Edit
        {
            self.help.open();
            return;
        }

        // Shift+F9 or Ctrl+F9: Copy last N lines with interactive count input
        if key.code == KeyCode::F(9)
            && (key
                .modifiers
                .contains(crossterm::event::KeyModifiers::SHIFT)
                || key
                    .modifiers
                    .contains(crossterm::event::KeyModifiers::CONTROL))
        {
            if matches!(
                self.active_pane,
                PaneId::Claude | PaneId::LazyGit | PaneId::Terminal
            ) {
                let default_count = self.config.pty.copy_lines_count.to_string();
                let cursor_pos = default_count.chars().count();
                self.dialog.dialog_type = ui::dialog::DialogType::Input {
                    title: "Copy last N lines".to_string(),
                    value: default_count,
                    cursor: cursor_pos,
                    action: ui::dialog::DialogAction::CopyLastLines,
                };
            }
            return;
        }

        // F9: Copy last N lines (terminal panes) or File Menu (file browser/preview)
        if key.code == KeyCode::F(9)
            && !key
                .modifiers
                .contains(crossterm::event::KeyModifiers::SHIFT)
            && !key
                .modifiers
                .contains(crossterm::event::KeyModifiers::CONTROL)
        {
            if matches!(
                self.active_pane,
                PaneId::Claude | PaneId::LazyGit | PaneId::Terminal
            ) {
                self.copy_last_lines_to_clipboard();
            } else {
                self.menu.toggle();
            }
            return;
        }

        // Ctrl+P: Open fuzzy finder
        if key.code == KeyCode::Char('p')
            && key
                .modifiers
                .contains(crossterm::event::KeyModifiers::CONTROL)
        {
            self.fuzzy_finder.open(&self.file_browser.current_dir);
            return;
        }

        // F8: Open settings
        if key.code == KeyCode::F(8) {
            self.settings.open(&self.config);
            return;
        }

        // Ctrl+Shift+W: Re-run setup wizard
        if key.code == KeyCode::Char('W')
            && key.modifiers.contains(
                crossterm::event::KeyModifiers::CONTROL | crossterm::event::KeyModifiers::SHIFT,
            )
        {
            self.wizard.open();
            return;
        }

        // Permission mode dialog handling (high priority - before Claude startup)
        // Skip if update dialog is visible - update takes priority
        if self.permission_mode_dialog.visible && !self.update_state.show_dialog {
            match key.code {
                KeyCode::Esc => {
                    // Cancel: use saved default or fall back to Default
                    let mode = self
                        .config
                        .claude
                        .default_permission_mode
                        .unwrap_or(crate::types::ClaudePermissionMode::Default);
                    self.permission_mode_dialog.close();
                    if self.claude_pty_pending {
                        self.init_claude_pty(mode);
                    }
                    self.active_pane = PaneId::Claude;
                }
                KeyCode::Enter => {
                    // Confirm selected mode and save to config
                    let mode = self.permission_mode_dialog.selected_mode();
                    let remote = self.permission_mode_dialog.remote_control;
                    self.permission_mode_dialog.confirm();
                    self.config.claude.default_permission_mode = Some(mode);
                    self.config.claude.remote_control = remote;
                    let _ = crate::config::save_config(&self.config);
                    if self.claude_pty_pending {
                        self.init_claude_pty(mode);
                    }
                    self.active_pane = PaneId::Claude;
                }
                KeyCode::Char(' ') => {
                    // Toggle remote control checkbox
                    self.permission_mode_dialog.toggle_remote_control();
                }
                KeyCode::Up | KeyCode::Char('k') => self.permission_mode_dialog.prev(),
                KeyCode::Down | KeyCode::Char('j') => self.permission_mode_dialog.next(),
                _ => {}
            }
            return;
        }

        // Claude startup dialog handling (high priority)
        if self.claude_startup.visible {
            match key.code {
                KeyCode::Esc => {
                    self.claude_startup.close();
                    self.active_pane = PaneId::Claude;
                }
                KeyCode::Enter => {
                    if let Some(prefix) = self.claude_startup.selected_prefix() {
                        if !prefix.is_empty() {
                            if let Some(pty) = self.terminals.get_mut(&PaneId::Claude) {
                                let cmd = format!("{}\n", prefix);
                                let _ = pty.write_input(cmd.as_bytes());
                            }
                        }
                    }
                    self.claude_startup.close();
                    self.active_pane = PaneId::Claude;
                }
                KeyCode::Up | KeyCode::Char('k') => self.claude_startup.prev(),
                KeyCode::Down | KeyCode::Char('j') => self.claude_startup.next(),
                _ => {}
            }
            return;
        }

        // Interactive pane resizing: Alt+Shift+Arrow
        if key
            .modifiers
            .contains(KeyModifiers::ALT | KeyModifiers::SHIFT)
        {
            match key.code {
                KeyCode::Left => {
                    self.config.layout.file_browser_width_percent = self
                        .config
                        .layout
                        .file_browser_width_percent
                        .saturating_sub(2)
                        .max(10);
                    let _ = crate::config::save_config(&self.config);
                    return;
                }
                KeyCode::Right => {
                    self.config.layout.file_browser_width_percent =
                        (self.config.layout.file_browser_width_percent + 2).min(50);
                    let _ = crate::config::save_config(&self.config);
                    return;
                }
                KeyCode::Up => {
                    self.config.layout.claude_height_percent = self
                        .config
                        .layout
                        .claude_height_percent
                        .saturating_sub(2)
                        .max(20);
                    let _ = crate::config::save_config(&self.config);
                    return;
                }
                KeyCode::Down => {
                    self.config.layout.claude_height_percent =
                        (self.config.layout.claude_height_percent + 2).min(80);
                    let _ = crate::config::save_config(&self.config);
                    return;
                }
                _ => {}
            }
        }

        // Global Focus Switching
        match key.code {
            KeyCode::F(1) => {
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
            KeyCode::F(2) => {
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
            KeyCode::F(3) => {
                self.toggle_preview_maximize();
            }
            KeyCode::F(4) => {
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
            KeyCode::F(5) => {
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
            KeyCode::F(6) => {
                if self.preview_maximized {
                    self.preview_maximized = false;
                }
                let was_hidden = !self.show_terminal;
                self.show_terminal = !self.show_terminal;
                self.config.ui.show_terminal = self.show_terminal;
                let _ = crate::config::save_config(&self.config);
                if self.show_terminal {
                    self.active_pane = PaneId::Terminal;
                    // Sync directory when showing terminal
                    if was_hidden {
                        self.sync_terminal_to_current_dir(PaneId::Terminal);
                    }
                } else if self.active_pane == PaneId::Terminal {
                    self.active_pane = PaneId::Preview;
                }
            }
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
                    PaneId::FileBrowser => {
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
                            // Open file in browser/external viewer
                            KeyCode::Char('o') => {
                                if let Some(path) = self.file_browser.selected_file() {
                                    if crate::browser::can_preview_in_browser(&path) {
                                        let preview_path = if crate::browser::is_markdown(&path) {
                                            match crate::browser::markdown_to_html(&path) {
                                                Ok(p) => {
                                                    self.temp_preview_files.push(p.clone());
                                                    p
                                                }
                                                Err(_) => path,
                                            }
                                        } else if crate::browser::can_syntax_highlight(&path) {
                                            match crate::browser::text_to_html(&path) {
                                                Ok(p) => {
                                                    self.temp_preview_files.push(p.clone());
                                                    p
                                                }
                                                Err(_) => path,
                                            }
                                        } else {
                                            path
                                        };
                                        let _ = crate::browser::open_file(&preview_path);
                                    }
                                }
                            }
                            // Open current directory in file manager
                            KeyCode::Char('O') => {
                                let _ = crate::browser::open_in_file_manager(
                                    &self.file_browser.current_dir,
                                );
                            }
                            // Allow single q to quit if in browser
                            KeyCode::Char('q') => {
                                self.should_quit = true;
                            }
                            // Toggle hidden files visibility
                            KeyCode::Char('.') => {
                                self.file_browser.show_hidden = !self.file_browser.show_hidden;
                                self.file_browser.refresh();
                                self.update_preview();
                            }
                            // Add selected file/folder to .gitignore
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

                    PaneId::Preview => {
                        // Search/Replace mode handling (priority over other modes)
                        if self.preview.search.active {
                            match key.code {
                                KeyCode::Esc => {
                                    self.preview.search.close();
                                    return;
                                }
                                // Ctrl+H: Toggle between Search and Replace mode (when search is open)
                                KeyCode::Char('h')
                                    if key.modifiers.contains(KeyModifiers::CONTROL) =>
                                {
                                    if self.preview.mode == EditorMode::Edit {
                                        self.preview.search.toggle_replace_mode();
                                    }
                                    return;
                                }
                                // Tab: Switch between search/replace fields (only in Replace mode)
                                KeyCode::Tab => {
                                    self.preview.search.toggle_field_focus();
                                    return;
                                }
                                // Ctrl+I: Toggle case sensitivity
                                KeyCode::Char('i')
                                    if key.modifiers.contains(KeyModifiers::CONTROL) =>
                                {
                                    self.preview.search.case_sensitive =
                                        !self.preview.search.case_sensitive;
                                    self.preview.perform_search();
                                    return;
                                }
                                KeyCode::Char('\x09') => {
                                    // Ctrl+I as control char
                                    self.preview.search.case_sensitive =
                                        !self.preview.search.case_sensitive;
                                    self.preview.perform_search();
                                    return;
                                }
                                // Enter: In Search mode = confirm and close
                                //        In Replace mode = replace current and move to next
                                KeyCode::Enter => {
                                    if self.preview.search.mode == SearchMode::Replace
                                        && self.preview.mode == EditorMode::Edit
                                    {
                                        self.preview.replace_and_next(&self.syntax_manager);
                                    } else {
                                        self.preview.jump_to_current_match();
                                        self.preview.search.active = false;
                                        // Keep query for n/N navigation
                                    }
                                    return;
                                }
                                // Ctrl+R: Replace all (only in Replace mode)
                                KeyCode::Char('r')
                                    if key.modifiers.contains(KeyModifiers::CONTROL)
                                        && self.preview.search.mode == SearchMode::Replace =>
                                {
                                    if self.preview.mode == EditorMode::Edit {
                                        let _count = self.preview.replace_all(&self.syntax_manager);
                                        // Could show count in status
                                    }
                                    return;
                                }
                                // Ctrl+N: Next match
                                KeyCode::Char('n')
                                    if key.modifiers.contains(KeyModifiers::CONTROL) =>
                                {
                                    self.preview.search.next_match();
                                    self.preview.jump_to_current_match();
                                    return;
                                }
                                // Ctrl+P: Previous match
                                KeyCode::Char('p')
                                    if key.modifiers.contains(KeyModifiers::CONTROL) =>
                                {
                                    self.preview.search.prev_match();
                                    self.preview.jump_to_current_match();
                                    return;
                                }
                                // ─────────────────────────────────────────
                                // Cursor Navigation in Search/Replace fields
                                // ─────────────────────────────────────────
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
                                // Delete: Delete character at cursor
                                KeyCode::Delete => {
                                    self.preview.search.delete_char_at();
                                    if !self.preview.search.focus_on_replace {
                                        self.preview.perform_search();
                                        self.preview.jump_to_current_match();
                                    }
                                    return;
                                }
                                // Backspace: Delete character before cursor
                                KeyCode::Backspace => {
                                    self.preview.search.delete_char_before();
                                    if !self.preview.search.focus_on_replace {
                                        self.preview.perform_search();
                                        self.preview.jump_to_current_match();
                                    }
                                    return;
                                }
                                // Character input at cursor position
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
                            || key.code == KeyCode::Char('\x06'); // Ctrl+F as control char
                        let is_slash = key.code == KeyCode::Char('/')
                            && self.preview.mode == EditorMode::ReadOnly;
                        // Ctrl+H opens Search & Replace directly
                        let is_ctrl_h = (key.code == KeyCode::Char('h')
                            && key.modifiers.contains(KeyModifiers::CONTROL))
                            || key.code == KeyCode::Char('\x08'); // Ctrl+H as control char (backspace, but with CONTROL modifier)

                        if is_ctrl_f || is_slash {
                            self.preview.search.open();
                            return;
                        }

                        // Ctrl+H: Open search in Replace mode directly
                        if is_ctrl_h && self.preview.mode == EditorMode::Edit {
                            self.preview.search.open();
                            self.preview.search.mode = SearchMode::Replace;
                            return;
                        }

                        // Edit mode handling
                        if self.preview.mode == EditorMode::Edit {
                            // Check for Ctrl+S (save) - handle both modifier and control char
                            let is_ctrl_s = (key.code == KeyCode::Char('s')
                                && key.modifiers.contains(KeyModifiers::CONTROL))
                                || key.code == KeyCode::Char('\x13'); // Ctrl+S as control char
                                                                      // Check for Ctrl+Y (delete line - MC Edit style)
                            let is_ctrl_y = (key.code == KeyCode::Char('y')
                                && key.modifiers.contains(KeyModifiers::CONTROL))
                                || key.code == KeyCode::Char('\x19'); // Ctrl+Y as control char

                            if key.code == KeyCode::Esc {
                                // Cancel selection first if active, then exit
                                if self.preview.block_marking {
                                    self.preview.cancel_selection();
                                } else if self.preview.is_modified() {
                                    if self.config.ui.autosave {
                                        // Autosave: save and exit without dialog
                                        let _ = self.preview.save();
                                        self.last_autosave_time = Some(std::time::Instant::now());
                                        self.preview.exit_edit_mode(false);
                                        self.preview.refresh_highlighting(&self.syntax_manager);
                                    } else {
                                        // Show discard dialog
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
                                if let Err(_e) = self.preview.save() {
                                    // Could show error dialog here
                                } else {
                                    // Refresh highlighting after save
                                    self.preview.refresh_highlighting(&self.syntax_manager);
                                }
                            }
                            // Ctrl+A = Toggle Autosave
                            else if key.code == KeyCode::Char('\x01')
                                || (key.code == KeyCode::Char('a')
                                    && key.modifiers.contains(KeyModifiers::CONTROL))
                            {
                                self.config.ui.autosave = !self.config.ui.autosave;
                                let _ = crate::config::save_config(&self.config);
                            }
                            // MC Edit style: Ctrl+Y = delete line
                            else if is_ctrl_y {
                                self.preview.delete_line();
                                self.preview.update_modified();
                                self.preview.update_edit_highlighting(&self.syntax_manager);
                            }
                            // Platform Copy: Cmd+C / Ctrl+C
                            else if key.code == KeyCode::Char('c')
                                && (key.modifiers.contains(KeyModifiers::SUPER)
                                    || key.modifiers.contains(KeyModifiers::CONTROL))
                            {
                                self.preview.copy_block();
                            }
                            // Platform Cut: Cmd+X / Ctrl+X
                            else if key.code == KeyCode::Char('x')
                                && (key.modifiers.contains(KeyModifiers::SUPER)
                                    || key.modifiers.contains(KeyModifiers::CONTROL))
                            {
                                self.preview.move_block();
                                self.preview.update_modified();
                                self.preview.update_edit_highlighting(&self.syntax_manager);
                            }
                            // Platform Paste: Cmd+V / Ctrl+V
                            else if key.code == KeyCode::Char('v')
                                && (key.modifiers.contains(KeyModifiers::SUPER)
                                    || key.modifiers.contains(KeyModifiers::CONTROL))
                            {
                                self.preview.paste_from_clipboard();
                                self.preview.update_modified();
                                self.preview.update_edit_highlighting(&self.syntax_manager);
                            }
                            // MC Edit style: Ctrl+F3 = toggle block marking
                            else if key.code == KeyCode::F(3)
                                && key.modifiers.contains(KeyModifiers::CONTROL)
                            {
                                self.preview.toggle_block_marking();
                            }
                            // MC Edit style: Ctrl+F5 = copy block
                            else if key.code == KeyCode::F(5)
                                && key.modifiers.contains(KeyModifiers::CONTROL)
                            {
                                self.preview.copy_block();
                                self.preview.update_modified();
                                self.preview.update_edit_highlighting(&self.syntax_manager);
                            }
                            // MC Edit style: Ctrl+F6 = move (cut) block
                            else if key.code == KeyCode::F(6)
                                && key.modifiers.contains(KeyModifiers::CONTROL)
                            {
                                self.preview.move_block();
                                self.preview.update_modified();
                                self.preview.update_edit_highlighting(&self.syntax_manager);
                            }
                            // MC Edit style: Ctrl+F8 = delete block
                            else if key.code == KeyCode::F(8)
                                && key.modifiers.contains(KeyModifiers::CONTROL)
                            {
                                self.preview.delete_block();
                                self.preview.update_modified();
                                self.preview.update_edit_highlighting(&self.syntax_manager);
                            }
                            // MC Edit style: Shift+Arrow = extend selection
                            else if key.modifiers.contains(KeyModifiers::SHIFT) {
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
                                        // Forward other Shift+key combos to TextArea
                                        if let Some(editor) = &mut self.preview.editor {
                                            editor.input(Event::Key(key));
                                            self.preview.update_modified();
                                            self.preview
                                                .update_edit_highlighting(&self.syntax_manager);
                                        }
                                    }
                                }
                            } else {
                                // Handle scrolling keys specially (TextArea moves cursor, not view)
                                match key.code {
                                    KeyCode::PageUp => {
                                        if let Some(editor) = &mut self.preview.editor {
                                            // Move cursor up by ~20 lines to simulate page scroll
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
                                        // Forward other keys to TextArea (handles Ctrl+Z undo, etc.)
                                        if let Some(editor) = &mut self.preview.editor {
                                            editor.input(Event::Key(key));
                                            self.preview.update_modified();
                                            // Update syntax highlighting for edit mode
                                            self.preview
                                                .update_edit_highlighting(&self.syntax_manager);
                                        }
                                        // Auto-adjust horizontal scroll to follow cursor
                                        if let Some(editor) = &self.preview.editor {
                                            let (_, cursor_col) = editor.cursor();
                                            let visible_width = self.preview_width as usize;
                                            let h_scroll = self.preview.horizontal_scroll as usize;
                                            if visible_width > 0
                                                && cursor_col
                                                    >= h_scroll + visible_width.saturating_sub(5)
                                            {
                                                self.preview.horizontal_scroll = (cursor_col
                                                    .saturating_sub(visible_width / 2))
                                                    as u16;
                                            } else if cursor_col < h_scroll {
                                                self.preview.horizontal_scroll =
                                                    cursor_col.saturating_sub(5) as u16;
                                            }
                                        }
                                    }
                                }
                            }
                        } else {
                            // Read-only mode - check for selection mode first
                            if self.terminal_selection.active
                                && self.terminal_selection.source_pane == Some(PaneId::Preview)
                            {
                                // Selection mode active in Preview
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
                                    // Ctrl+C / Cmd+C: Copy selection to system clipboard
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
                            } else {
                                // Normal read-only mode (no selection)
                                // Check for Ctrl+S to start selection mode
                                let is_ctrl_s = (key.code == KeyCode::Char('s')
                                    && key.modifiers.contains(KeyModifiers::CONTROL))
                                    || key.code == KeyCode::Char('\x13');
                                if is_ctrl_s {
                                    // Start keyboard selection at current scroll position
                                    self.terminal_selection
                                        .start(self.preview.scroll as usize, PaneId::Preview);
                                    return;
                                }

                                // Ctrl+A = Toggle Autosave (also available in ReadOnly)
                                let is_ctrl_a = key.code == KeyCode::Char('\x01')
                                    || (key.code == KeyCode::Char('a')
                                        && key.modifiers.contains(KeyModifiers::CONTROL));
                                if is_ctrl_a {
                                    self.config.ui.autosave = !self.config.ui.autosave;
                                    let _ = crate::config::save_config(&self.config);
                                    return;
                                }

                                match key.code {
                                    KeyCode::Down | KeyCode::Char('j') => {
                                        self.preview.scroll_down()
                                    }
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
                                        let max =
                                            self.preview.highlighted_lines.len().saturating_sub(1)
                                                as u16;
                                        self.preview.scroll = max;
                                    }
                                    KeyCode::Char('e') | KeyCode::Char('E') => {
                                        self.preview.enter_edit_mode();
                                    }
                                    // Search navigation: n = next match, N = previous match
                                    KeyCode::Char('n')
                                        if !self.preview.search.matches.is_empty() =>
                                    {
                                        self.preview.search.next_match();
                                        self.preview.jump_to_current_match();
                                    }
                                    KeyCode::Char('N')
                                        if !self.preview.search.matches.is_empty() =>
                                    {
                                        self.preview.search.prev_match();
                                        self.preview.jump_to_current_match();
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                    PaneId::Terminal | PaneId::Claude | PaneId::LazyGit => {
                        // Terminal selection mode handling
                        if self.terminal_selection.active
                            && self.terminal_selection.source_pane == Some(self.active_pane)
                        {
                            match key.code {
                                KeyCode::Up | KeyCode::Char('k') => {
                                    if let Some(end) = self.terminal_selection.end_line {
                                        self.terminal_selection.extend(end.saturating_sub(1));
                                    }
                                    return;
                                }
                                KeyCode::Down | KeyCode::Char('j') => {
                                    if let Some(end) = self.terminal_selection.end_line {
                                        self.terminal_selection.extend(end + 1);
                                    }
                                    return;
                                }
                                KeyCode::Enter | KeyCode::Char('y') => {
                                    self.copy_selection_to_claude();
                                    self.terminal_selection.clear();
                                    return;
                                }
                                // Ctrl+C / Cmd+C: Copy selection to system clipboard
                                KeyCode::Char('c')
                                    if key.modifiers.contains(KeyModifiers::CONTROL)
                                        || key.modifiers.contains(KeyModifiers::SUPER) =>
                                {
                                    self.copy_selection_to_clipboard();
                                    self.terminal_selection.clear();
                                    return;
                                }
                                KeyCode::Esc => {
                                    self.terminal_selection.clear();
                                    return;
                                }
                                _ => {
                                    // Let other keys pass through to PTY
                                }
                            }
                        }

                        // Ctrl+S: Start terminal selection mode
                        if key.code == KeyCode::Char('s')
                            && key.modifiers.contains(KeyModifiers::CONTROL)
                        {
                            if let Some(pty) = self.terminals.get(&self.active_pane) {
                                let cursor_row = pty.cursor_row() as usize;
                                self.terminal_selection.start(cursor_row, self.active_pane);
                            }
                            return;
                        }

                        if let Some(pty) = self.terminals.get(&self.active_pane) {
                            // Check if PTY has exited and auto_restart is disabled
                            if pty.has_exited() && !self.config.pty.auto_restart {
                                // Manual restart on Enter
                                if key.code == KeyCode::Enter {
                                    self.restart_single_pty(self.active_pane);
                                }
                                return;
                            }
                        }

                        if let Some(pty) = self.terminals.get_mut(&self.active_pane) {
                            // Scroll Handling
                            if key
                                .modifiers
                                .contains(crossterm::event::KeyModifiers::SHIFT)
                            {
                                match key.code {
                                    KeyCode::PageUp => {
                                        pty.scroll_up(10);
                                        return;
                                    }
                                    KeyCode::PageDown => {
                                        pty.scroll_down(10);
                                        return;
                                    }
                                    KeyCode::Up => {
                                        pty.scroll_up(1);
                                        return;
                                    }
                                    KeyCode::Down => {
                                        pty.scroll_down(1);
                                        return;
                                    }
                                    _ => {}
                                }
                            }

                            if let Some(bytes) = crate::input::map_key_to_pty(key) {
                                let _ = pty.write_input(&bytes);
                            }
                        }
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
                        editor.insert_str(&text);
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
}
