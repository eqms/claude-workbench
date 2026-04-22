use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use crate::types::{EditorMode, PaneId, SearchMode};
use crate::ui;

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

    // =====================================================================
    // Overlay key handlers (extracted from handle_key_event for clarity).
    // Each handler owns all input while its overlay is visible.
    // =====================================================================

    fn handle_fuzzy_finder_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => self.fuzzy_finder.close(),
            KeyCode::Enter => {
                if let Some(selected) = self.fuzzy_finder.selected() {
                    let full_path = self.fuzzy_finder.base_dir.join(&selected);
                    if let Some(parent) = full_path.parent() {
                        self.file_browser.current_dir = parent.to_path_buf();
                        self.file_browser.load_directory();
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
    }

    fn handle_update_dialog_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                self.update_state.close_dialog();
            }
            KeyCode::Enter => {
                if self.update_state.update_success {
                    if self.update_dialog_button
                        == crate::ui::update_dialog::UpdateDialogButton::Restart
                    {
                        self.should_restart = true;
                        self.should_quit = true;
                    } else {
                        self.update_state.close_dialog();
                    }
                } else if self.update_state.available_version.is_some()
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
                if self.update_state.update_success {
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
                    self.update_dialog_button = self.update_dialog_button.toggle();
                }
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.update_state.scroll_release_notes_up();
            }
            KeyCode::Down | KeyCode::Char('j') => {
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
    }

    fn handle_export_chooser_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => self.export_chooser.visible = false,
            KeyCode::Up | KeyCode::Char('k') => {
                if self.export_chooser.selected > 0 {
                    self.export_chooser.selected -= 1;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.export_chooser.selected < 1 {
                    self.export_chooser.selected += 1;
                }
            }
            KeyCode::Enter => {
                let format = if self.export_chooser.selected == 0 {
                    crate::browser::pdf_export::ExportFormat::Markdown
                } else {
                    crate::browser::pdf_export::ExportFormat::Pdf
                };
                let source = self.export_chooser.source_path.clone();
                self.export_chooser.visible = false;

                let export_dir =
                    crate::browser::pdf_export::resolve_export_dir(&self.config.ui.export_dir);
                let project_name = self
                    .file_browser
                    .root_dir
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("");
                let filename = crate::browser::pdf_export::default_export_filename(
                    &source,
                    format,
                    project_name,
                );
                let target = export_dir.join(&filename);
                let target_str = target.to_string_lossy().to_string();
                let cursor = target_str.len();
                self.dialog.dialog_type = ui::dialog::DialogType::Input {
                    title: format!(
                        "Export as {}",
                        if format == crate::browser::pdf_export::ExportFormat::Pdf {
                            "PDF"
                        } else {
                            "Markdown"
                        }
                    ),
                    value: target_str,
                    cursor,
                    action: ui::dialog::DialogAction::ExportMarkdown { source, format },
                };
            }
            _ => {}
        }
    }

    fn handle_active_dialog_key(&mut self, key: KeyEvent) {
        match &self.dialog.dialog_type {
            ui::dialog::DialogType::Input { value, action, .. } => match key.code {
                KeyCode::Esc => self.dialog.close(),
                KeyCode::Enter => {
                    let val = value.clone();
                    let act = action.clone();
                    self.dialog.close();
                    self.execute_dialog_action(act, Some(val));
                }
                KeyCode::Tab => {
                    if matches!(
                        action,
                        ui::dialog::DialogAction::GoToPath
                            | ui::dialog::DialogAction::OpenMarkdownPreview
                            | ui::dialog::DialogAction::ExportMarkdown { .. }
                    ) {
                        self.dialog.try_complete_path();
                    }
                }
                KeyCode::Backspace => self.dialog.delete_char_before(),
                KeyCode::Delete => self.dialog.delete_char_at(),
                KeyCode::Left => self.dialog.cursor_left(),
                KeyCode::Right => self.dialog.cursor_right(),
                KeyCode::Home => self.dialog.cursor_home(),
                KeyCode::End => self.dialog.cursor_end(),
                KeyCode::Char(c) => {
                    if key.modifiers.contains(KeyModifiers::CONTROL) {
                        match c {
                            'v' => {
                                if let Some(text) = crate::clipboard::paste_from_clipboard() {
                                    for ch in text.chars() {
                                        self.dialog.insert_char(ch);
                                    }
                                }
                            }
                            'c' => {
                                self.dialog.close();
                            }
                            _ => {}
                        }
                    } else {
                        self.dialog.insert_char(c);
                    }
                }
                _ => {}
            },
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
    }

    fn handle_menu_key(&mut self, key: KeyEvent) {
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
            KeyCode::Char('x') => {
                self.menu.visible = false;
                self.handle_menu_action(ui::menu::MenuAction::ExportFile);
            }
            _ => {}
        }
    }

    fn handle_about_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc | KeyCode::F(10) | KeyCode::Char('q') => self.about.close(),
            _ => {}
        }
    }

    fn handle_help_key(&mut self, key: KeyEvent) {
        if self.help.search_active {
            match key.code {
                KeyCode::Esc => {
                    self.help.stop_search();
                }
                KeyCode::Enter => {
                    self.help.stop_search();
                    self.help.scroll = 0;
                }
                KeyCode::Backspace => {
                    self.help.search_backspace();
                    self.help.scroll = 0;
                }
                KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    self.help.clear_search();
                }
                KeyCode::Char(c) => {
                    self.help.search_add_char(c);
                    self.help.scroll = 0;
                }
                _ => {}
            }
        } else {
            match key.code {
                KeyCode::Esc | KeyCode::F(12) | KeyCode::Char('q') => self.help.close(),
                KeyCode::Char('/') | KeyCode::Char('f')
                    if key.code == KeyCode::Char('/')
                        || key.modifiers.contains(KeyModifiers::CONTROL) =>
                {
                    self.help.start_search();
                }
                KeyCode::Up | KeyCode::Char('k') => self.help.scroll_up(1),
                KeyCode::Down | KeyCode::Char('j') => self.help.scroll_down(1),
                KeyCode::PageUp => self.help.page_up(),
                KeyCode::PageDown => self.help.page_down(),
                KeyCode::Home | KeyCode::Char('g') => self.help.scroll_to_top(),
                KeyCode::End | KeyCode::Char('G') => self.help.scroll_to_bottom(),
                KeyCode::Char('u') => {
                    self.help.close();
                    self.update_state.manual_check = true;
                    self.start_update_check();
                }
                _ => {}
            }
        }
    }

    fn handle_permission_mode_dialog_key(&mut self, key: KeyEvent) {
        use crate::ui::permission_mode::DialogSection;

        match key.code {
            KeyCode::Esc => {
                // Close without saving — start with persisted defaults.
                self.permission_mode_dialog.close();
                if self.claude_pty_pending {
                    let opts = crate::app::pty::StartupOptions {
                        permission_mode: self
                            .config
                            .claude
                            .default_permission_mode
                            .unwrap_or(crate::types::ClaudePermissionMode::Default),
                        model: self.config.claude.default_model,
                        effort: self.config.claude.default_effort,
                        session_name: self.config.claude.default_session_name.clone(),
                        worktree: self.config.claude.default_worktree.clone(),
                        remote_control: self.config.claude.remote_control,
                    };
                    self.init_claude_pty(opts);
                }
                self.active_pane = PaneId::Claude;
            }
            KeyCode::Enter => {
                // Collect all values from dialog state
                let mode = self.permission_mode_dialog.selected_permission_mode();
                let model = self.permission_mode_dialog.selected_model();
                let effort = self.permission_mode_dialog.selected_effort();
                let session_name = self.permission_mode_dialog.session_name.clone();
                let worktree = self.permission_mode_dialog.worktree.clone();
                let remote = self.permission_mode_dialog.remote_control;

                self.permission_mode_dialog.confirm();

                // Persist everything
                self.config.claude.default_permission_mode = Some(mode);
                self.config.claude.default_model = model;
                self.config.claude.default_effort = effort;
                self.config.claude.default_session_name = session_name.clone();
                self.config.claude.default_worktree = worktree.clone();
                self.config.claude.remote_control = remote;
                let _ = crate::config::save_config(&self.config);

                if self.claude_pty_pending {
                    let opts = crate::app::pty::StartupOptions {
                        permission_mode: mode,
                        model,
                        effort,
                        session_name,
                        worktree,
                        remote_control: remote,
                    };
                    self.init_claude_pty(opts);
                }
                self.active_pane = PaneId::Claude;
            }
            KeyCode::Tab => self.permission_mode_dialog.next_section(),
            KeyCode::BackTab => self.permission_mode_dialog.prev_section(),
            _ => {
                // Delegate remaining keys based on current section
                match self.permission_mode_dialog.section {
                    DialogSection::Permission => match key.code {
                        KeyCode::Up | KeyCode::Char('k') => self.permission_mode_dialog.prev_item(),
                        KeyCode::Down | KeyCode::Char('j') => {
                            self.permission_mode_dialog.next_item()
                        }
                        _ => {}
                    },
                    DialogSection::Model | DialogSection::Effort => match key.code {
                        KeyCode::Left | KeyCode::Up | KeyCode::Char('h') | KeyCode::Char('k') => {
                            self.permission_mode_dialog.prev_item()
                        }
                        KeyCode::Right
                        | KeyCode::Down
                        | KeyCode::Char('l')
                        | KeyCode::Char('j') => self.permission_mode_dialog.next_item(),
                        _ => {}
                    },
                    DialogSection::Session | DialogSection::Worktree => match key.code {
                        KeyCode::Left => self.permission_mode_dialog.cursor_left(),
                        KeyCode::Right => self.permission_mode_dialog.cursor_right(),
                        KeyCode::Home => self.permission_mode_dialog.cursor_home(),
                        KeyCode::End => self.permission_mode_dialog.cursor_end(),
                        KeyCode::Backspace => self.permission_mode_dialog.delete_char_before(),
                        KeyCode::Delete => self.permission_mode_dialog.delete_char_at(),
                        KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                            self.permission_mode_dialog.insert_char(c);
                        }
                        _ => {}
                    },
                    DialogSection::RemoteControl => {
                        if let KeyCode::Char(' ') = key.code {
                            self.permission_mode_dialog.toggle_remote_control();
                        }
                    }
                }
            }
        }
    }

    fn handle_claude_startup_key(&mut self, key: KeyEvent) {
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
    }

    /// Returns true if the key was a global shortcut and has been consumed.
    fn handle_global_shortcut(&mut self, key: KeyEvent) -> bool {
        // F10/F12 work everywhere
        if key.code == KeyCode::F(12) {
            self.help.open();
            return true;
        }
        if key.code == KeyCode::F(10) {
            self.about.open();
            return true;
        }

        // F7: Toggle between ~/.claude and previous directory
        if key.code == KeyCode::F(7) {
            if let Some(home) = std::env::var_os("HOME") {
                let claude_dir = std::path::PathBuf::from(home).join(".claude");
                if self.file_browser.current_dir.starts_with(&claude_dir)
                    || self.file_browser.root_dir.starts_with(&claude_dir)
                {
                    if let Some(prev) = self.file_browser.previous_dir.take() {
                        self.file_browser.current_dir = prev;
                        self.file_browser.load_directory();
                        self.active_pane = PaneId::FileBrowser;
                    }
                } else if claude_dir.exists() && claude_dir.is_dir() {
                    self.file_browser.previous_dir = Some(self.file_browser.root_dir.clone());
                    self.file_browser.current_dir = claude_dir;
                    self.file_browser.load_directory();
                    self.active_pane = PaneId::FileBrowser;
                }
            }
            return true;
        }

        // '?' for help - only in FileBrowser or Preview (read-only)
        if key.code == KeyCode::Char('?')
            && matches!(self.active_pane, PaneId::FileBrowser | PaneId::Preview)
            && self.preview.mode != EditorMode::Edit
        {
            self.help.open();
            return true;
        }

        // Shift+F9 or Ctrl+F9: Copy last N lines with interactive count input
        if key.code == KeyCode::F(9)
            && (key.modifiers.contains(KeyModifiers::SHIFT)
                || key.modifiers.contains(KeyModifiers::CONTROL))
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
            return true;
        }

        // F9: Copy last N lines (terminal panes) or File Menu (file browser/preview)
        if key.code == KeyCode::F(9)
            && !key.modifiers.contains(KeyModifiers::SHIFT)
            && !key.modifiers.contains(KeyModifiers::CONTROL)
        {
            if matches!(
                self.active_pane,
                PaneId::Claude | PaneId::LazyGit | PaneId::Terminal
            ) {
                self.copy_last_lines_to_clipboard();
            } else {
                self.menu.toggle();
            }
            return true;
        }

        // Ctrl+P: Open fuzzy finder
        if key.code == KeyCode::Char('p') && key.modifiers.contains(KeyModifiers::CONTROL) {
            self.fuzzy_finder.open(&self.file_browser.current_dir);
            return true;
        }

        // Ctrl+O: Open Markdown file by path (dialog with tab-completion)
        if key.code == KeyCode::Char('o') && key.modifiers.contains(KeyModifiers::CONTROL) {
            let default_path = dirs::home_dir()
                .map(|h| format!("{}/.claude/plans/", h.display()))
                .unwrap_or_default();
            let cursor = default_path.len();
            self.dialog.dialog_type = crate::ui::dialog::DialogType::Input {
                title: "Open Markdown Preview".to_string(),
                value: default_path,
                cursor,
                action: crate::ui::dialog::DialogAction::OpenMarkdownPreview,
            };
            return true;
        }

        // Ctrl+X: Export current Markdown file (show format chooser)
        if key.code == KeyCode::Char('x') && key.modifiers.contains(KeyModifiers::CONTROL) {
            if let Some(path) = &self.preview.current_file {
                if self.preview.is_markdown {
                    self.export_chooser = crate::types::ExportChooserState {
                        visible: true,
                        source_path: path.clone(),
                        selected: 0,
                    };
                    return true;
                }
            }
        }

        // Ctrl+E: Open selected file in external GUI editor
        if key.code == KeyCode::Char('e') && key.modifiers.contains(KeyModifiers::CONTROL) {
            let path_opt = if self.active_pane == PaneId::Preview {
                self.preview.current_file.clone()
            } else {
                self.file_browser.selected_file()
            };
            if let Some(path) = path_opt {
                let editor = &self.config.ui.external_editor;
                if !editor.is_empty() {
                    let _ = crate::browser::open_file_with_editor(&path, editor);
                }
            }
            return true;
        }

        // F8: Open settings
        if key.code == KeyCode::F(8) {
            self.settings.open(&self.config);
            return true;
        }

        // Ctrl+Shift+W: Re-run setup wizard
        if key.code == KeyCode::Char('W')
            && key
                .modifiers
                .contains(KeyModifiers::CONTROL | KeyModifiers::SHIFT)
        {
            self.wizard.open();
            return true;
        }

        false
    }

    fn handle_preview_pane_key(&mut self, key: KeyEvent) {
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

    fn handle_terminal_pane_key(&mut self, key: KeyEvent) {
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
        if key.code == KeyCode::Char('s') && key.modifiers.contains(KeyModifiers::CONTROL) {
            if let Some(pty) = self.terminals.get(&self.active_pane) {
                let cursor_row = pty.cursor_row() as usize;
                self.terminal_selection.start(cursor_row, self.active_pane);
            }
            return;
        }

        if let Some(pty) = self.terminals.get(&self.active_pane) {
            if pty.has_exited() && !self.config.pty.auto_restart {
                if key.code == KeyCode::Enter {
                    self.restart_single_pty(self.active_pane);
                }
                return;
            }
        }

        if let Some(pty) = self.terminals.get_mut(&self.active_pane) {
            if key.modifiers.contains(KeyModifiers::SHIFT) {
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

    fn handle_file_browser_pane_key(&mut self, key: KeyEvent) {
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
}
