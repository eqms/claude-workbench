//! Overlay key handlers — fuzzy finder, update dialog, export chooser,
//! confirm/input dialogs, file menu, about/help, permission-mode dialog,
//! Claude startup dialog. Each handler owns all input while its overlay
//! is visible; the dispatcher in `keyboard::mod` routes here based on
//! visibility flags on `App`.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::types::PaneId;
use crate::ui;

use super::super::App;

impl App {
    pub(super) fn handle_fuzzy_finder_key(&mut self, key: KeyEvent) {
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

    pub(super) fn handle_update_dialog_key(&mut self, key: KeyEvent) {
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

    pub(super) fn handle_export_chooser_key(&mut self, key: KeyEvent) {
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

    pub(super) fn handle_active_dialog_key(&mut self, key: KeyEvent) {
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

    pub(super) fn handle_menu_key(&mut self, key: KeyEvent) {
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

    pub(super) fn handle_about_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc | KeyCode::F(10) | KeyCode::Char('q') => self.about.close(),
            _ => {}
        }
    }

    pub(super) fn handle_help_key(&mut self, key: KeyEvent) {
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

    pub(super) fn handle_permission_mode_dialog_key(&mut self, key: KeyEvent) {
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

    pub(super) fn handle_claude_startup_key(&mut self, key: KeyEvent) {
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
}
