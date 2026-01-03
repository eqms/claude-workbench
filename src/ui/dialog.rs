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
}

#[derive(Debug, Clone)]
pub struct Dialog {
    pub dialog_type: DialogType,
}

impl Default for Dialog {
    fn default() -> Self {
        Self {
            dialog_type: DialogType::None,
        }
    }
}

impl Dialog {
    pub fn is_active(&self) -> bool {
        !matches!(self.dialog_type, DialogType::None)
    }

    pub fn close(&mut self) {
        self.dialog_type = DialogType::None;
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

pub fn render(f: &mut Frame, area: Rect, dialog: &Dialog) {
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
            let height = 6u16;
            let x = area.x + (area.width.saturating_sub(width)) / 2;
            let y = area.y + (area.height.saturating_sub(height)) / 2;
            let popup_area = Rect::new(x, y, width, height);

            f.render_widget(Clear, popup_area);

            let block = Block::default()
                .borders(Borders::ALL)
                .title(format!(" {} ", title))
                .style(Style::default().bg(Color::Red).fg(Color::White));

            let inner = block.inner(popup_area);
            f.render_widget(block, popup_area);

            // Message
            let msg = Paragraph::new(message.as_str())
                .style(Style::default().fg(Color::White));
            f.render_widget(msg, Rect::new(inner.x, inner.y + 1, inner.width, 1));

            // Buttons
            let buttons = Line::from(vec![
                Span::styled(" [Y] Yes ", Style::default().bg(Color::Green).fg(Color::Black)),
                Span::raw("  "),
                Span::styled(" [N] No ", Style::default().bg(Color::Gray).fg(Color::Black)),
            ]);
            f.render_widget(Paragraph::new(buttons), Rect::new(inner.x, inner.y + 3, inner.width, 1));
        }
    }
}
