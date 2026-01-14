use ratatui::{
    layout::Rect,
    style::{Color, Style, Modifier},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

#[derive(Debug, Clone, PartialEq)]
pub enum DialogType {
    None,
    Input { title: String, value: String, action: DialogAction },
    Confirm { title: String, message: String, action: DialogAction },
}

#[derive(Debug, Clone, PartialEq)]
pub enum DialogAction {
    NewFile,
    RenameFile { old_path: std::path::PathBuf },
    DeleteFile { path: std::path::PathBuf },
    DiscardEditorChanges,
    SwitchFile { target_idx: usize },
    EnterDirectory { target_idx: usize },
    /// Git pull confirmation (repo_root is the path to pull from)
    GitPull { repo_root: std::path::PathBuf },
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

    pub fn push_char(&mut self, c: char) {
        if let DialogType::Input { value, .. } = &mut self.dialog_type {
            value.push(c);
        }
    }

    pub fn pop_char(&mut self) {
        if let DialogType::Input { value, .. } = &mut self.dialog_type {
            value.pop();
        }
    }

    pub fn get_action(&self) -> Option<DialogAction> {
        match &self.dialog_type {
            DialogType::Input { action, .. } => Some(action.clone()),
            DialogType::Confirm { action, .. } => Some(action.clone()),
            DialogType::None => None,
        }
    }
}

pub fn render(f: &mut Frame, area: Rect, dialog: &mut Dialog) {
    // Clear stored button areas
    dialog.yes_button_area = None;
    dialog.no_button_area = None;
    dialog.popup_area = None;

    match &dialog.dialog_type {
        DialogType::None => {}
        DialogType::Input { title, value, .. } => {
            let width = 50u16.min(area.width.saturating_sub(4));
            let height = 5u16;
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

            // Input field
            let input_line = Line::from(vec![
                Span::styled(value.as_str(), Style::default().fg(Color::Yellow)),
                Span::styled("_", Style::default().fg(Color::White).add_modifier(Modifier::SLOW_BLINK)),
            ]);
            f.render_widget(Paragraph::new(input_line), Rect::new(inner.x, inner.y + 1, inner.width, 1));

            // Help text
            let help = Paragraph::new("Enter: Confirm | Esc: Cancel")
                .style(Style::default().fg(Color::Gray));
            f.render_widget(help, Rect::new(inner.x, inner.y + 2, inner.width, 1));
        }
        DialogType::Confirm { title, message, .. } => {
            let width = 50u16.min(area.width.saturating_sub(4));
            let height = 7u16;  // Slightly taller for better spacing
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
                .title_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
                .style(Style::default().bg(Color::DarkGray));

            let inner = block.inner(popup_rect);
            f.render_widget(block, popup_rect);

            // Message with white text on dark background
            let msg = Paragraph::new(message.as_str())
                .style(Style::default().fg(Color::White));
            f.render_widget(msg, Rect::new(inner.x, inner.y + 1, inner.width, 2));

            // Button dimensions: " [Y] Yes " = 9 chars, "   " = 3 chars, " [N] No " = 8 chars
            let yes_width = 9u16;
            let no_width = 8u16;
            let gap_width = 3u16;
            let button_y = inner.y + 4;

            // Store button areas for mouse click detection
            dialog.yes_button_area = Some(Rect::new(inner.x, button_y, yes_width, 1));
            dialog.no_button_area = Some(Rect::new(inner.x + yes_width + gap_width, button_y, no_width, 1));

            // Buttons with better contrast
            let buttons = Line::from(vec![
                Span::styled(" [Y] Yes ", Style::default().bg(Color::Cyan).fg(Color::Black)),
                Span::raw("   "),
                Span::styled(" [N] No ", Style::default().bg(Color::Gray).fg(Color::White)),
            ]);
            f.render_widget(Paragraph::new(buttons), Rect::new(inner.x, button_y, inner.width, 1));
        }
    }
}
