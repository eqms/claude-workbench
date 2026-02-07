//! About/License dialog

use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

/// About dialog state
#[derive(Debug, Clone, Default)]
pub struct AboutState {
    pub visible: bool,
    /// Cached popup area for mouse hit testing
    pub popup_area: Option<Rect>,
}

impl AboutState {
    pub fn open(&mut self) {
        self.visible = true;
    }

    pub fn close(&mut self) {
        self.visible = false;
        self.popup_area = None;
    }
}

/// Render the about dialog
pub fn render(frame: &mut Frame, area: Rect, state: &mut AboutState) {
    // Fixed size compact dialog: 50x9
    let popup_width = 50u16.min(area.width.saturating_sub(4));
    let popup_height = 9u16;

    let popup_x = (area.width.saturating_sub(popup_width)) / 2;
    let popup_y = (area.height.saturating_sub(popup_height)) / 2;

    let popup_area = Rect::new(popup_x, popup_y, popup_width, popup_height);

    // Store popup area for mouse hit testing
    state.popup_area = Some(popup_area);

    // Clear background
    frame.render_widget(Clear, popup_area);

    let version = env!("CARGO_PKG_VERSION");
    let title = format!(" Claude Workbench v{} ", version);

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let inner = block.inner(popup_area);
    frame.render_widget(block, popup_area);

    // Compact content with copyright and license
    let content = Paragraph::new(vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("Copyright ", Style::default().fg(Color::DarkGray)),
            Span::styled("(c) 2025-2026 Martin Schmid", Style::default().fg(Color::White)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("License: ", Style::default().fg(Color::DarkGray)),
            Span::styled("MIT License", Style::default().fg(Color::Green)),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "Press F12 for Open Source Components",
            Style::default().fg(Color::DarkGray),
        )),
    ]);
    frame.render_widget(content, inner);
}
