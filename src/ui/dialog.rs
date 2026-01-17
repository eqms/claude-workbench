use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

#[derive(Debug, Clone, PartialEq)]
pub enum DialogType {
    None,
    Input {
        title: String,
        value: String,
        cursor: usize,
        action: DialogAction,
    },
    Confirm {
        title: String,
        message: String,
        action: DialogAction,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum DialogAction {
    NewFile,
    NewDirectory,
    RenameFile {
        old_path: std::path::PathBuf,
    },
    DeleteFile {
        path: std::path::PathBuf,
    },
    /// Copy file to destination (value is destination path)
    CopyFileTo {
        source: std::path::PathBuf,
    },
    /// Move file to destination (value is destination path)
    MoveFileTo {
        source: std::path::PathBuf,
    },
    DiscardEditorChanges,
    SwitchFile {
        target_idx: usize,
    },
    EnterDirectory {
        target_idx: usize,
    },
    /// Git pull confirmation (repo_root is the path to pull from)
    GitPull {
        repo_root: std::path::PathBuf,
    },
    /// Navigate to a specific path
    GoToPath,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ConfirmResult {
    Yes,
    No,
}

#[derive(Debug, Clone)]
pub struct Dialog {
    pub dialog_type: DialogType,
    /// Stored button areas for mouse click detection (set during render)
    pub yes_button_area: Option<Rect>,
    pub no_button_area: Option<Rect>,
    pub popup_area: Option<Rect>,
}

impl Default for Dialog {
    fn default() -> Self {
        Self {
            dialog_type: DialogType::None,
            yes_button_area: None,
            no_button_area: None,
            popup_area: None,
        }
    }
}

impl Dialog {
    pub fn is_active(&self) -> bool {
        !matches!(self.dialog_type, DialogType::None)
    }

    pub fn close(&mut self) {
        self.dialog_type = DialogType::None;
        self.yes_button_area = None;
        self.no_button_area = None;
        self.popup_area = None;
    }

    /// Check if a click is inside the popup area
    pub fn contains(&self, x: u16, y: u16) -> bool {
        if let Some(area) = self.popup_area {
            x >= area.x && x < area.x + area.width && y >= area.y && y < area.y + area.height
        } else {
            false
        }
    }

    /// Check which button was clicked (if any)
    pub fn check_button_click(&self, x: u16, y: u16) -> Option<ConfirmResult> {
        if let Some(area) = self.yes_button_area {
            if x >= area.x && x < area.x + area.width && y >= area.y && y < area.y + area.height {
                return Some(ConfirmResult::Yes);
            }
        }
        if let Some(area) = self.no_button_area {
            if x >= area.x && x < area.x + area.width && y >= area.y && y < area.y + area.height {
                return Some(ConfirmResult::No);
            }
        }
        None
    }

    pub fn input_value(&self) -> Option<&str> {
        match &self.dialog_type {
            DialogType::Input { value, .. } => Some(value),
            _ => None,
        }
    }

    /// Insert character at cursor position
    pub fn insert_char(&mut self, c: char) {
        if let DialogType::Input { value, cursor, .. } = &mut self.dialog_type {
            // Get byte position from char index
            let byte_pos = value
                .char_indices()
                .nth(*cursor)
                .map(|(i, _)| i)
                .unwrap_or(value.len());
            value.insert(byte_pos, c);
            *cursor += 1;
        }
    }

    /// Delete character before cursor (Backspace)
    pub fn delete_char_before(&mut self) {
        if let DialogType::Input { value, cursor, .. } = &mut self.dialog_type {
            if *cursor > 0 {
                // Get byte position of char before cursor
                let byte_pos = value
                    .char_indices()
                    .nth(*cursor - 1)
                    .map(|(i, _)| i)
                    .unwrap_or(0);
                value.remove(byte_pos);
                *cursor -= 1;
            }
        }
    }

    /// Delete character at cursor (Delete key)
    pub fn delete_char_at(&mut self) {
        if let DialogType::Input { value, cursor, .. } = &mut self.dialog_type {
            let char_count = value.chars().count();
            if *cursor < char_count {
                let byte_pos = value
                    .char_indices()
                    .nth(*cursor)
                    .map(|(i, _)| i)
                    .unwrap_or(value.len());
                value.remove(byte_pos);
            }
        }
    }

    /// Move cursor left
    pub fn cursor_left(&mut self) {
        if let DialogType::Input { cursor, .. } = &mut self.dialog_type {
            if *cursor > 0 {
                *cursor -= 1;
            }
        }
    }

    /// Move cursor right
    pub fn cursor_right(&mut self) {
        if let DialogType::Input { value, cursor, .. } = &mut self.dialog_type {
            let char_count = value.chars().count();
            if *cursor < char_count {
                *cursor += 1;
            }
        }
    }

    /// Move cursor to start (Home)
    pub fn cursor_home(&mut self) {
        if let DialogType::Input { cursor, .. } = &mut self.dialog_type {
            *cursor = 0;
        }
    }

    /// Move cursor to end (End)
    pub fn cursor_end(&mut self) {
        if let DialogType::Input { value, cursor, .. } = &mut self.dialog_type {
            *cursor = value.chars().count();
        }
    }

    /// Get current cursor position
    pub fn cursor_pos(&self) -> usize {
        match &self.dialog_type {
            DialogType::Input { cursor, .. } => *cursor,
            _ => 0,
        }
    }

    pub fn get_action(&self) -> Option<DialogAction> {
        match &self.dialog_type {
            DialogType::Input { action, .. } => Some(action.clone()),
            DialogType::Confirm { action, .. } => Some(action.clone()),
            DialogType::None => None,
        }
    }

    /// Set the input value and move cursor to end (used by tab-completion)
    pub fn set_value(&mut self, new_value: String) {
        if let DialogType::Input { value, cursor, .. } = &mut self.dialog_type {
            let new_cursor = new_value.chars().count();
            *value = new_value;
            *cursor = new_cursor;
        }
    }

    /// Try to complete path with tab completion
    /// Returns true if completion was performed
    pub fn try_complete_path(&mut self) -> bool {
        let current_value = match &self.dialog_type {
            DialogType::Input {
                value,
                action: DialogAction::GoToPath,
                ..
            } => value.clone(),
            _ => return false,
        };

        if current_value.is_empty() {
            return false;
        }

        use std::path::Path;
        let path = Path::new(&current_value);

        // Determine the directory to scan and the prefix to match
        let (scan_dir, prefix) = if current_value.ends_with('/') || current_value.ends_with('\\') {
            // Path ends with separator: scan that directory, match all entries
            (path.to_path_buf(), String::new())
        } else if path.is_dir() {
            // Complete path is a directory: add separator and continue
            let mut completed = current_value.clone();
            completed.push('/');
            self.set_value(completed);
            return true;
        } else {
            // Partial name: scan parent directory, match entries starting with the last component
            let parent = path.parent().unwrap_or(Path::new("/"));
            let file_name = path
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default();
            (parent.to_path_buf(), file_name)
        };

        // Read directory entries
        let entries: Vec<String> = match std::fs::read_dir(&scan_dir) {
            Ok(dir) => dir
                .filter_map(|e| e.ok())
                .filter_map(|e| {
                    let name = e.file_name().to_string_lossy().to_string();
                    // Filter entries that start with prefix (case-insensitive)
                    if prefix.is_empty()
                        || name.to_lowercase().starts_with(&prefix.to_lowercase())
                    {
                        Some(name)
                    } else {
                        None
                    }
                })
                .collect(),
            Err(_) => return false,
        };

        if entries.is_empty() {
            return false;
        }

        // Sort entries: directories first, then files, alphabetically
        let mut sorted_entries: Vec<(String, bool)> = entries
            .into_iter()
            .map(|name| {
                let full_path = scan_dir.join(&name);
                let is_dir = full_path.is_dir();
                (name, is_dir)
            })
            .collect();

        sorted_entries.sort_by(|(a_name, a_dir), (b_name, b_dir)| {
            match (a_dir, b_dir) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a_name.to_lowercase().cmp(&b_name.to_lowercase()),
            }
        });

        // Find longest common prefix among all matches
        if sorted_entries.len() == 1 {
            // Single match: complete fully
            let (name, is_dir) = &sorted_entries[0];
            let mut completed = scan_dir.join(name).to_string_lossy().to_string();
            if *is_dir {
                completed.push('/');
            }
            self.set_value(completed);
            return true;
        }

        // Multiple matches: complete to longest common prefix
        let first = &sorted_entries[0].0;
        let mut common_len = first.len();

        for (name, _) in &sorted_entries[1..] {
            let matching = first
                .chars()
                .zip(name.chars())
                .take_while(|(a, b)| a.to_lowercase().eq(b.to_lowercase()))
                .count();
            common_len = common_len.min(matching);
        }

        if common_len > prefix.len() {
            // We can extend the path
            let common: String = first.chars().take(common_len).collect();
            let completed = scan_dir.join(&common).to_string_lossy().to_string();
            self.set_value(completed);
            return true;
        }

        false
    }

    /// Get completion suggestions for current path input
    pub fn get_path_completions(&self) -> Vec<(String, bool)> {
        let current_value = match &self.dialog_type {
            DialogType::Input {
                value,
                action: DialogAction::GoToPath,
                ..
            } => value.clone(),
            _ => return Vec::new(),
        };

        if current_value.is_empty() {
            return Vec::new();
        }

        use std::path::Path;
        let path = Path::new(&current_value);

        // Determine the directory to scan and the prefix to match
        let (scan_dir, prefix) = if current_value.ends_with('/') || current_value.ends_with('\\') {
            (path.to_path_buf(), String::new())
        } else {
            let parent = path.parent().unwrap_or(Path::new("/"));
            let file_name = path
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default();
            (parent.to_path_buf(), file_name)
        };

        // Read directory entries
        let entries: Vec<(String, bool)> = match std::fs::read_dir(&scan_dir) {
            Ok(dir) => dir
                .filter_map(|e| e.ok())
                .filter_map(|e| {
                    let name = e.file_name().to_string_lossy().to_string();
                    if prefix.is_empty()
                        || name.to_lowercase().starts_with(&prefix.to_lowercase())
                    {
                        let full_path = scan_dir.join(&name);
                        let is_dir = full_path.is_dir();
                        Some((name, is_dir))
                    } else {
                        None
                    }
                })
                .collect(),
            Err(_) => return Vec::new(),
        };

        // Sort: directories first, then alphabetically
        let mut sorted = entries;
        sorted.sort_by(|(a_name, a_dir), (b_name, b_dir)| {
            match (a_dir, b_dir) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a_name.to_lowercase().cmp(&b_name.to_lowercase()),
            }
        });

        // Limit to 10 suggestions
        sorted.truncate(10);
        sorted
    }
}

pub fn render(f: &mut Frame, area: Rect, dialog: &mut Dialog) {
    // Clear stored button areas
    dialog.yes_button_area = None;
    dialog.no_button_area = None;
    dialog.popup_area = None;

    match &dialog.dialog_type {
        DialogType::None => {}
        DialogType::Input {
            title,
            value,
            cursor,
            action,
        } => {
            // Get completions for GoToPath dialog
            let completions = dialog.get_path_completions();
            let has_completions =
                matches!(action, DialogAction::GoToPath) && !completions.is_empty();

            // Dynamic height: base 5 + completion list rows (max 8)
            let completion_rows = if has_completions {
                completions.len().min(8) as u16
            } else {
                0
            };
            let width = 60u16.min(area.width.saturating_sub(4));
            let height = 5u16 + completion_rows;
            let x = area.x + (area.width.saturating_sub(width)) / 2;
            let y = area.y + (area.height.saturating_sub(height)) / 2;
            let popup_area = Rect::new(x, y, width, height);

            f.render_widget(Clear, popup_area);

            let block = Block::default()
                .borders(Borders::ALL)
                .title(format!(" {} ", title))
                .style(Style::default().bg(Color::DarkGray).fg(Color::White));

            let inner = block.inner(popup_area);
            f.render_widget(block, popup_area);

            // Input field with cursor at correct position
            let chars: Vec<char> = value.chars().collect();
            let before_cursor: String = chars[..*cursor].iter().collect();
            let at_cursor: String = if *cursor < chars.len() {
                chars[*cursor].to_string()
            } else {
                " ".to_string()
            };
            let after_cursor: String = if *cursor < chars.len() {
                chars[*cursor + 1..].iter().collect()
            } else {
                String::new()
            };

            let input_line = Line::from(vec![
                Span::styled(before_cursor, Style::default().fg(Color::Yellow)),
                Span::styled(
                    at_cursor,
                    Style::default()
                        .fg(Color::Black)
                        .bg(Color::Yellow)
                        .add_modifier(Modifier::SLOW_BLINK),
                ),
                Span::styled(after_cursor, Style::default().fg(Color::Yellow)),
            ]);
            f.render_widget(
                Paragraph::new(input_line),
                Rect::new(inner.x, inner.y + 1, inner.width, 1),
            );

            // Help text (with Tab hint for GoToPath)
            let help_text = if matches!(action, DialogAction::GoToPath) {
                "Tab: Complete | Enter: Confirm | Esc: Cancel"
            } else {
                "Enter: Confirm | Esc: Cancel"
            };
            let help = Paragraph::new(help_text).style(Style::default().fg(Color::Gray));
            f.render_widget(help, Rect::new(inner.x, inner.y + 2, inner.width, 1));

            // Render completion list for GoToPath
            if has_completions {
                for (i, (name, is_dir)) in completions.iter().take(8).enumerate() {
                    let display_name = if *is_dir {
                        format!("📁 {}/", name)
                    } else {
                        format!("📄 {}", name)
                    };

                    let style = if *is_dir {
                        Style::default().fg(Color::Cyan)
                    } else {
                        Style::default().fg(Color::White)
                    };

                    let completion_line = Paragraph::new(display_name).style(style);
                    f.render_widget(
                        completion_line,
                        Rect::new(inner.x, inner.y + 3 + i as u16, inner.width, 1),
                    );
                }
            }
        }
        DialogType::Confirm { title, message, .. } => {
            let width = 50u16.min(area.width.saturating_sub(4));
            let height = 7u16; // Slightly taller for better spacing
            let x = area.x + (area.width.saturating_sub(width)) / 2;
            let y = area.y + (area.height.saturating_sub(height)) / 2;
            let popup_rect = Rect::new(x, y, width, height);

            // Store popup area for click detection
            dialog.popup_area = Some(popup_rect);

            f.render_widget(Clear, popup_rect);

            // Neutral dark background with yellow warning border
            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Yellow))
                .title(format!(" ⚠ {} ", title))
                .title_style(
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                )
                .style(Style::default().bg(Color::DarkGray));

            let inner = block.inner(popup_rect);
            f.render_widget(block, popup_rect);

            // Message with white text on dark background
            let msg = Paragraph::new(message.as_str()).style(Style::default().fg(Color::White));
            f.render_widget(msg, Rect::new(inner.x, inner.y + 1, inner.width, 2));

            // Button dimensions: " [Y] Yes " = 9 chars, "   " = 3 chars, " [N] No " = 8 chars
            let yes_width = 9u16;
            let no_width = 8u16;
            let gap_width = 3u16;
            let button_y = inner.y + 4;

            // Store button areas for mouse click detection
            dialog.yes_button_area = Some(Rect::new(inner.x, button_y, yes_width, 1));
            dialog.no_button_area = Some(Rect::new(
                inner.x + yes_width + gap_width,
                button_y,
                no_width,
                1,
            ));

            // Buttons with better contrast
            let buttons = Line::from(vec![
                Span::styled(
                    " [Y] Yes ",
                    Style::default().bg(Color::Cyan).fg(Color::Black),
                ),
                Span::raw("   "),
                Span::styled(" [N] No ", Style::default().bg(Color::Red).fg(Color::White)),
            ]);
            f.render_widget(
                Paragraph::new(buttons),
                Rect::new(inner.x, button_y, inner.width, 1),
            );
        }
    }
}
