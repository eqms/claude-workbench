use ratatui::{
    layout::Rect,
    style::{Color, Style, Modifier},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MenuAction {
    None,
    NewFile,
    NewDirectory,
    RenameFile,
    DuplicateFile,
    CopyFileTo,
    MoveFileTo,
    DeleteFile,
    CopyAbsolutePath,
    CopyRelativePath,
}

pub struct MenuBar {
    pub visible: bool,
    pub selected: usize,
}

impl Default for MenuBar {
    fn default() -> Self {
        Self {
            visible: false,
            selected: 0,
        }
    }
}

impl MenuBar {
    pub fn toggle(&mut self) {
        self.visible = !self.visible;
        self.selected = 0;
    }

    pub fn next(&mut self) {
        self.selected = (self.selected + 1) % 9;
    }

    pub fn prev(&mut self) {
        if self.selected == 0 {
            self.selected = 8;
        } else {
            self.selected -= 1;
        }
    }

    pub fn action(&self) -> MenuAction {
        match self.selected {
            0 => MenuAction::NewFile,
            1 => MenuAction::NewDirectory,
            2 => MenuAction::RenameFile,
            3 => MenuAction::DuplicateFile,
            4 => MenuAction::CopyFileTo,
            5 => MenuAction::MoveFileTo,
            6 => MenuAction::DeleteFile,
            7 => MenuAction::CopyAbsolutePath,
            8 => MenuAction::CopyRelativePath,
            _ => MenuAction::None,
        }
    }
}

pub fn render(f: &mut Frame, area: Rect, menu: &MenuBar) {
    if !menu.visible {
        return;
    }

    let items = vec![
        ("n", "New File"),
        ("N", "New Directory"),
        ("r", "Rename"),
        ("u", "Duplicate"),
        ("c", "Copy to..."),
        ("m", "Move to..."),
        ("d", "Delete"),
        ("y", "Copy Abs Path"),
        ("Y", "Copy Rel Path"),
    ];

    // Menu popup in center-top
    let width = 40u16;
    let height = (items.len() + 2) as u16;
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + 2;
    let popup_area = Rect::new(x, y, width, height);

    // Clear background
    f.render_widget(Clear, popup_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" File Menu (Esc to close) ")
        .style(Style::default().bg(Color::DarkGray));

    let inner = block.inner(popup_area);
    f.render_widget(block, popup_area);

    for (i, (key, label)) in items.iter().enumerate() {
        let style = if i == menu.selected {
            Style::default().bg(Color::Blue).fg(Color::White).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };

        let line = Line::from(vec![
            Span::styled(format!(" [{}] ", key), Style::default().fg(Color::Yellow)),
            Span::styled(*label, style),
        ]);

        let item_area = Rect::new(inner.x, inner.y + i as u16, inner.width, 1);
        f.render_widget(Paragraph::new(line).style(Style::default().bg(Color::DarkGray)), item_area);
    }
}
