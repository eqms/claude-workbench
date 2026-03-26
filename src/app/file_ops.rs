use crossterm::event::{KeyCode, KeyModifiers};

use crate::ui;

use super::App;

impl App {
    pub(super) fn handle_menu_action(&mut self, action: ui::menu::MenuAction) {
        use ui::dialog::{DialogAction, DialogType};
        use ui::menu::MenuAction;

        match action {
            MenuAction::NewFile => {
                self.dialog.dialog_type = DialogType::Input {
                    title: "New File".to_string(),
                    value: String::new(),
                    cursor: 0,
                    action: DialogAction::NewFile,
                };
            }
            MenuAction::NewDirectory => {
                self.dialog.dialog_type = DialogType::Input {
                    title: "New Directory".to_string(),
                    value: String::new(),
                    cursor: 0,
                    action: DialogAction::NewDirectory,
                };
            }
            MenuAction::RenameFile => {
                if let Some(selected) = self.file_browser.selected_file() {
                    let name = selected
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_default();
                    let cursor_pos = name.chars().count();
                    self.dialog.dialog_type = DialogType::Input {
                        title: "Rename".to_string(),
                        value: name,
                        cursor: cursor_pos,
                        action: DialogAction::RenameFile { old_path: selected },
                    };
                }
            }
            MenuAction::DuplicateFile => {
                if let Some(selected) = self.file_browser.selected_file() {
                    if selected.is_file() {
                        // Generate duplicate name with counter
                        let parent = selected.parent().unwrap_or(&self.file_browser.current_dir);
                        let stem = selected
                            .file_stem()
                            .map(|s| s.to_string_lossy().to_string())
                            .unwrap_or_default();
                        let ext = selected
                            .extension()
                            .map(|e| format!(".{}", e.to_string_lossy()))
                            .unwrap_or_default();

                        let mut counter = 1;
                        let mut new_path;
                        loop {
                            let new_name = format!("{} - Duplikat {}{}", stem, counter, ext);
                            new_path = parent.join(&new_name);
                            if !new_path.exists() {
                                break;
                            }
                            counter += 1;
                        }

                        if let Err(e) = std::fs::copy(&selected, &new_path) {
                            eprintln!("Failed to duplicate file: {}", e);
                        } else {
                            self.file_browser.refresh();
                            self.update_preview();
                        }
                    }
                }
            }
            MenuAction::CopyFileTo => {
                if let Some(selected) = self.file_browser.selected_file() {
                    if selected.is_file() {
                        let dir_str = self.file_browser.current_dir.to_string_lossy().to_string();
                        let cursor_pos = dir_str.chars().count();
                        self.dialog.dialog_type = DialogType::Input {
                            title: "Copy to".to_string(),
                            value: dir_str,
                            cursor: cursor_pos,
                            action: DialogAction::CopyFileTo { source: selected },
                        };
                    }
                }
            }
            MenuAction::MoveFileTo => {
                if let Some(selected) = self.file_browser.selected_file() {
                    if selected.is_file() {
                        let dir_str = self.file_browser.current_dir.to_string_lossy().to_string();
                        let cursor_pos = dir_str.chars().count();
                        self.dialog.dialog_type = DialogType::Input {
                            title: "Move to".to_string(),
                            value: dir_str,
                            cursor: cursor_pos,
                            action: DialogAction::MoveFileTo { source: selected },
                        };
                    }
                }
            }
            MenuAction::DeleteFile => {
                if let Some(selected) = self.file_browser.selected_file() {
                    if selected.file_name().map(|n| n.to_string_lossy()) != Some("..".into()) {
                        let name = selected
                            .file_name()
                            .map(|n| n.to_string_lossy().to_string())
                            .unwrap_or_default();
                        self.dialog.dialog_type = DialogType::Confirm {
                            title: "Delete".to_string(),
                            message: format!("Delete '{}'?", name),
                            action: DialogAction::DeleteFile { path: selected },
                        };
                    }
                }
            }
            MenuAction::CopyAbsolutePath => {
                if let Some(selected) = self.file_browser.selected_file() {
                    let path = selected.to_string_lossy().to_string();
                    crate::clipboard::copy_to_clipboard(&path);
                }
            }
            MenuAction::CopyRelativePath => {
                if let Some(selected) = self.file_browser.selected_file() {
                    if let Ok(rel) = selected.strip_prefix(&self.file_browser.current_dir) {
                        let path = rel.to_string_lossy().to_string();
                        crate::clipboard::copy_to_clipboard(&path);
                    }
                }
            }
            MenuAction::GoToPath => {
                let dir_str = self.file_browser.current_dir.to_string_lossy().to_string();
                let cursor_pos = dir_str.chars().count();
                self.dialog.dialog_type = ui::dialog::DialogType::Input {
                    title: "Go to Path".to_string(),
                    value: dir_str,
                    cursor: cursor_pos,
                    action: ui::dialog::DialogAction::GoToPath,
                };
            }
            MenuAction::AddToGitignore => {
                if let Some(path) = self.file_browser.selected_file() {
                    self.add_to_gitignore(&path);
                }
            }
            MenuAction::ExportFile => {
                if let Some(path) = &self.preview.current_file {
                    if self.preview.is_markdown {
                        self.export_chooser = crate::types::ExportChooserState {
                            visible: true,
                            source_path: path.clone(),
                            selected: 0,
                        };
                    }
                }
            }
            MenuAction::None => {}
        }
    }

    /// Check if a filename is safe (no path traversal).
    /// Uses char-level checks for proper Unicode handling.
    pub(super) fn is_safe_filename(name: &str) -> bool {
        !name.contains("..")
            && !name.starts_with('/')
            && !name.starts_with('\\')
            && !name.contains('\0')
            && !name.chars().any(|c| c == '/' || c == '\\')
    }

    /// Validate that a destination path is safe for file operations.
    /// Ensures the resolved path is a real directory and prevents traversal tricks.
    pub(super) fn is_safe_destination(dest: &str) -> bool {
        let path = std::path::Path::new(dest);
        // Path must exist and be a directory
        if !path.is_dir() {
            return false;
        }
        // Canonicalize to resolve symlinks and .. components
        // If canonicalize fails, the path is suspicious
        path.canonicalize().is_ok()
    }

    pub(super) fn execute_dialog_action(
        &mut self,
        action: ui::dialog::DialogAction,
        value: Option<String>,
    ) {
        use ui::dialog::DialogAction;

        match action {
            DialogAction::NewFile => {
                if let Some(name) = value {
                    if !name.is_empty() && Self::is_safe_filename(&name) {
                        let new_path = self.file_browser.current_dir.join(&name);
                        let _ = std::fs::write(&new_path, "");
                        self.file_browser.refresh();
                    }
                }
            }
            DialogAction::NewDirectory => {
                if let Some(name) = value {
                    if !name.is_empty() && Self::is_safe_filename(&name) {
                        let new_path = self.file_browser.current_dir.join(&name);
                        if let Err(e) = std::fs::create_dir(&new_path) {
                            eprintln!("Failed to create directory: {}", e);
                        } else {
                            self.file_browser.refresh();
                        }
                    }
                }
            }
            DialogAction::RenameFile { old_path } => {
                if let Some(new_name) = value {
                    if !new_name.is_empty() && Self::is_safe_filename(&new_name) {
                        let new_path = old_path
                            .parent()
                            .map(|p| p.join(&new_name))
                            .unwrap_or_else(|| std::path::PathBuf::from(&new_name));
                        let _ = std::fs::rename(&old_path, &new_path);
                        self.file_browser.refresh();
                    }
                }
            }
            DialogAction::DeleteFile { path } => {
                if path.is_file() {
                    let _ = std::fs::remove_file(&path);
                } else if path.is_dir() {
                    let _ = std::fs::remove_dir_all(&path);
                }
                self.file_browser.refresh();
            }
            DialogAction::CopyFileTo { source } => {
                if let Some(dest_dir) = value {
                    if !dest_dir.is_empty() && Self::is_safe_destination(&dest_dir) {
                        let dest_path = std::path::Path::new(&dest_dir)
                            .canonicalize()
                            .unwrap_or_default();
                        if let Some(filename) = source.file_name() {
                            let target = dest_path.join(filename);
                            if let Err(e) = std::fs::copy(&source, &target) {
                                eprintln!("Failed to copy file: {}", e);
                            } else {
                                self.file_browser.refresh();
                            }
                        }
                    }
                }
            }
            DialogAction::MoveFileTo { source } => {
                if let Some(dest_dir) = value {
                    if !dest_dir.is_empty() && Self::is_safe_destination(&dest_dir) {
                        let dest_path = std::path::Path::new(&dest_dir)
                            .canonicalize()
                            .unwrap_or_default();
                        if let Some(filename) = source.file_name() {
                            let target = dest_path.join(filename);
                            if let Err(e) = std::fs::rename(&source, &target) {
                                eprintln!("Failed to move file: {}", e);
                            } else {
                                self.file_browser.refresh();
                            }
                        }
                    }
                }
            }
            DialogAction::DiscardEditorChanges => {
                self.preview.exit_edit_mode(true); // true = discard changes
                self.preview.refresh_highlighting(&self.syntax_manager);
            }
            DialogAction::SwitchFile { target_idx } => {
                // Discard changes and switch to clicked file
                self.preview.exit_edit_mode(true);
                self.preview.refresh_highlighting(&self.syntax_manager);
                self.file_browser.list_state.select(Some(target_idx));
                self.update_preview();
            }
            DialogAction::EnterDirectory { target_idx } => {
                // Discard changes and enter the clicked directory
                self.preview.exit_edit_mode(true);
                self.preview.refresh_highlighting(&self.syntax_manager);
                self.file_browser.list_state.select(Some(target_idx));
                self.file_browser.enter_selected();
                self.update_preview();
                self.sync_terminals();
                self.check_repo_change();
            }
            DialogAction::GitPull { repo_root } => {
                // Execute git pull
                match crate::git::pull(&repo_root) {
                    Ok(output) => {
                        // Show success dialog with first 2 lines of output
                        let summary: String = output.lines().take(2).collect::<Vec<_>>().join("\n");
                        self.dialog.dialog_type = ui::dialog::DialogType::Confirm {
                            title: "Git Pull".to_string(),
                            message: format!("✓ Pull successful!\n{}", summary),
                            action: DialogAction::GoToPath, // Dummy action - just closes on confirm
                        };
                        // Refresh file browser to show any new/changed files
                        self.file_browser.refresh();
                    }
                    Err(err) => {
                        // Show error dialog
                        self.dialog.dialog_type = ui::dialog::DialogType::Confirm {
                            title: "Git Pull Error".to_string(),
                            message: format!("✗ Pull failed:\n{}", err),
                            action: DialogAction::GoToPath, // Dummy action - just closes on confirm
                        };
                    }
                }
            }
            DialogAction::CopyLastLines => {
                if let Some(count_str) = value {
                    if let Ok(count) = count_str.trim().parse::<usize>() {
                        if count > 0 {
                            self.copy_last_lines_to_clipboard_n(count);
                        }
                    }
                }
            }
            DialogAction::GoToPath => {
                if let Some(path_str) = value {
                    if !path_str.is_empty() {
                        let target = std::path::Path::new(&path_str);
                        if target.is_dir() {
                            self.file_browser.current_dir = target.to_path_buf();
                            self.file_browser.load_directory();
                            self.update_preview();
                            self.sync_terminals();
                            self.check_repo_change();
                        } else if target.is_file() {
                            if let Some(parent) = target.parent() {
                                self.file_browser.current_dir = parent.to_path_buf();
                                self.file_browser.load_directory();
                                // Select the file
                                if let Some(name) = target.file_name() {
                                    let name_str = name.to_string_lossy().to_string();
                                    for (idx, entry) in self.file_browser.entries.iter().enumerate()
                                    {
                                        if entry.name == name_str {
                                            self.file_browser.list_state.select(Some(idx));
                                            break;
                                        }
                                    }
                                }
                                self.update_preview();
                                self.sync_terminals();
                                self.check_repo_change();
                            }
                        }
                    }
                }
            }
            DialogAction::OpenMarkdownPreview => {
                if let Some(path_str) = value {
                    if !path_str.is_empty() {
                        // Expand tilde to home directory
                        let expanded = if path_str.starts_with('~') {
                            if let Some(home) = dirs::home_dir() {
                                path_str.replacen('~', &home.display().to_string(), 1)
                            } else {
                                path_str.to_string()
                            }
                        } else {
                            path_str.to_string()
                        };
                        let target = std::path::PathBuf::from(&expanded);
                        if target.is_file() {
                            self.open_in_browser(&target);
                        }
                    }
                }
            }
            DialogAction::ExportMarkdown { source, format } => {
                if let Some(target_str) = value {
                    if !target_str.is_empty() {
                        // Expand tilde
                        let expanded = if target_str.starts_with('~') {
                            if let Some(home) = dirs::home_dir() {
                                target_str.replacen('~', &home.display().to_string(), 1)
                            } else {
                                target_str.to_string()
                            }
                        } else {
                            target_str.to_string()
                        };
                        let target_path = std::path::PathBuf::from(&expanded);

                        // Ensure parent directory exists
                        if let Some(parent) = target_path.parent() {
                            let _ = std::fs::create_dir_all(parent);
                        }

                        let title = source
                            .file_stem()
                            .and_then(|s| s.to_str())
                            .unwrap_or("Export")
                            .to_string();

                        let options = crate::browser::pdf_export::ExportOptions {
                            title,
                            author: self.config.document.resolved_author(),
                            date: chrono_date_now(),
                            format,
                        };

                        match crate::browser::pdf_export::export_markdown(
                            &source,
                            &target_path,
                            &options,
                            &self.config.document,
                        ) {
                            Ok(path) => {
                                // Flash success indicator
                                self.copy_flash_lines = 0; // reuse flash mechanism
                                self.last_copy_time = Some(std::time::Instant::now());
                                // Open the exported file with configured browser
                                let _ = crate::browser::opener::open_file_with_browser(
                                    &path,
                                    &self.config.ui.browser,
                                );
                            }
                            Err(e) => {
                                // Show error to user via confirm dialog
                                self.dialog.dialog_type = ui::dialog::DialogType::Confirm {
                                    title: "Export Failed".to_string(),
                                    message: format!("{}", e),
                                    action: ui::dialog::DialogAction::DiscardEditorChanges,
                                };
                            }
                        }
                    }
                }
            }
        }
    }

    /// Open a file in the configured browser (with Markdown→HTML conversion)
    pub(crate) fn open_in_browser(&mut self, path: &std::path::Path) {
        use crate::browser;
        if browser::can_preview_in_browser(path) {
            let preview_path = if browser::is_markdown(path) {
                match browser::markdown_to_html(path, &self.config.document) {
                    Ok(p) => {
                        self.temp_preview_files.push(p.clone());
                        p
                    }
                    Err(_) => path.to_path_buf(),
                }
            } else if browser::can_syntax_highlight(path) {
                match browser::text_to_html(path, &self.config.document) {
                    Ok(p) => {
                        self.temp_preview_files.push(p.clone());
                        p
                    }
                    Err(_) => path.to_path_buf(),
                }
            } else {
                path.to_path_buf()
            };
            let _ = browser::open_file_with_browser(&preview_path, &self.config.ui.browser);
        }
    }

    /// Handle wizard input
    pub(super) fn handle_wizard_input(&mut self, code: KeyCode, _modifiers: KeyModifiers) {
        use crate::setup::wizard::WizardStep;

        // If editing a field
        if self.wizard.editing_field.is_some() {
            match code {
                KeyCode::Esc => self.wizard.cancel_editing(),
                KeyCode::Enter => self.wizard.finish_editing(),
                KeyCode::Backspace => {
                    self.wizard.input_buffer.pop();
                }
                KeyCode::Char(c) => self.wizard.input_buffer.push(c),
                _ => {}
            }
            return;
        }

        match code {
            KeyCode::Esc => {
                // On Welcome, Esc closes wizard; on other steps, go back
                if self.wizard.step == WizardStep::Welcome {
                    self.wizard.close();
                } else {
                    self.wizard.prev_step();
                }
            }
            KeyCode::Enter => {
                if self.wizard.step == WizardStep::Confirmation {
                    // Save config immediately when confirming
                    let new_config = self.wizard.generate_config();
                    self.config = new_config;
                    if let Err(e) = crate::config::save_config(&self.config) {
                        eprintln!("Failed to save config: {}", e);
                    }
                    self.wizard.next_step(); // Go to Complete
                } else if self.wizard.step == WizardStep::Complete {
                    // Close wizard and initialize Claude PTY with new config
                    self.wizard.close();
                    self.init_claude_after_wizard();
                } else if self.wizard.can_proceed() {
                    self.wizard.next_step();
                }
            }
            KeyCode::Tab | KeyCode::Right => {
                if self.wizard.can_proceed() {
                    self.wizard.next_step();
                }
            }
            KeyCode::BackTab | KeyCode::Left => {
                self.wizard.prev_step();
            }
            KeyCode::Up | KeyCode::Char('k') => match self.wizard.step {
                WizardStep::ShellSelection => {
                    if self.wizard.selected_shell_idx > 0 {
                        self.wizard.selected_shell_idx -= 1;
                    }
                }
                WizardStep::ClaudeConfig => {
                    if self.wizard.focused_field > 0 {
                        self.wizard.focused_field -= 1;
                    }
                }
                _ => {}
            },
            KeyCode::Down | KeyCode::Char('j') => match self.wizard.step {
                WizardStep::ShellSelection => {
                    if self.wizard.selected_shell_idx
                        < self.wizard.available_shells.len().saturating_sub(1)
                    {
                        self.wizard.selected_shell_idx += 1;
                    }
                }
                WizardStep::ClaudeConfig => {
                    if self.wizard.focused_field < 1 {
                        self.wizard.focused_field += 1;
                    }
                }
                _ => {}
            },
            KeyCode::Char('e') | KeyCode::Char('E') => {
                // Edit in ClaudeConfig step
                if self.wizard.step == WizardStep::ClaudeConfig {
                    use crate::setup::wizard::WizardField;
                    let field = if self.wizard.focused_field == 0 {
                        WizardField::ClaudePath
                    } else {
                        WizardField::LazygitPath
                    };
                    self.wizard.start_editing(field);
                }
            }
            _ => {}
        }
    }

    /// Handle settings input
    pub(super) fn handle_settings_input(&mut self, code: KeyCode, modifiers: KeyModifiers) {
        // Dropdown handling (highest priority)
        if self.settings.has_dropdown() {
            match code {
                KeyCode::Esc => self.settings.dropdown = None,
                KeyCode::Up | KeyCode::Char('k') => self.settings.dropdown_move_up(),
                KeyCode::Down | KeyCode::Char('j') => self.settings.dropdown_move_down(),
                KeyCode::Enter => {
                    self.settings.dropdown_confirm();
                }
                _ => {}
            }
            return;
        }

        // If editing a field
        if self.settings.editing.is_some() {
            match code {
                KeyCode::Esc => self.settings.cancel_editing(),
                KeyCode::Enter => self.settings.finish_editing(),
                KeyCode::Backspace => {
                    self.settings.input_buffer.pop();
                }
                KeyCode::Char(c) => {
                    if modifiers.contains(KeyModifiers::CONTROL) {
                        match c {
                            'v' => {
                                if let Some(text) = crate::clipboard::paste_from_clipboard() {
                                    self.settings.input_buffer.push_str(&text);
                                }
                            }
                            'c' => self.settings.cancel_editing(),
                            _ => {}
                        }
                    } else {
                        self.settings.input_buffer.push(c);
                    }
                }
                _ => {}
            }
            return;
        }

        match code {
            KeyCode::Esc => {
                // Auto-save changes before closing
                if self.settings.has_changes {
                    self.settings.apply_to_config(&mut self.config);
                    if let Err(e) = crate::config::save_config(&self.config) {
                        eprintln!("Failed to save config: {}", e);
                    }
                }
                self.settings.close();
            }
            KeyCode::Tab => {
                self.settings.next_category();
            }
            KeyCode::BackTab => {
                self.settings.prev_category();
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.settings.move_up();
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.settings.move_down();
            }
            KeyCode::Enter | KeyCode::Char(' ') => {
                // Check if "Check for Updates" is selected
                if self.settings.is_check_updates_selected() {
                    self.settings.close();
                    self.trigger_update_check();
                } else {
                    self.settings.toggle_or_select();
                }
            }
            KeyCode::Char('s') | KeyCode::Char('S') => {
                // Save and close
                if self.settings.has_changes {
                    self.settings.apply_to_config(&mut self.config);
                    if let Err(e) = crate::config::save_config(&self.config) {
                        eprintln!("Failed to save config: {}", e);
                    }
                }
                self.settings.close();
            }
            _ => {}
        }
    }
}

/// Get current date as DD.MM.YYYY string
fn chrono_date_now() -> String {
    use std::time::SystemTime;
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let time_t = now as libc::time_t;
    let mut tm: libc::tm = unsafe { std::mem::zeroed() };
    #[cfg(not(target_os = "windows"))]
    {
        unsafe {
            libc::localtime_r(&time_t, &mut tm);
        }
    }
    #[cfg(target_os = "windows")]
    {
        unsafe {
            libc::localtime_s(&mut tm, &time_t);
        }
    }
    format!(
        "{:02}.{:02}.{}",
        tm.tm_mday,
        tm.tm_mon + 1,
        tm.tm_year + 1900
    )
}
