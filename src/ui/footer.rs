use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Paragraph, Widget},
};

pub struct Footer;

impl Widget for Footer {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let keys = vec![
            ("F1", "Files"),
            ("F2", "Preview"),
            ("F3", "Refresh"),
            ("F4", "Claude"),
            ("F5", "Git"),
            ("F6", "Term"),
            ("F9", "Menu"),
            ("?", "Help"),
        ];

        let mut spans = Vec::new();
        // Midnight Commander style: Blue background. 
        // Keys: Cyan or similar. Description: White/Gray.
        for (key, desc) in keys {
            // Key Block
            spans.push(Span::styled(
                format!(" {} ", key), 
                Style::default().bg(Color::Cyan).fg(Color::Black)
            ));
            // Desc Block
            spans.push(Span::styled(
                format!(" {} ", desc), 
                Style::default().bg(Color::Blue).fg(Color::White)
            ));
            // Spacer
            spans.push(Span::raw(" "));
        }

        // Version info on the right
        let version = env!("CARGO_PKG_VERSION");
        let version_text = format!(" v{} ", version);
        let version_width = version_text.len() as u16;
        
        // Calculate positions
        let keys_area = Rect::new(area.x, area.y, area.width.saturating_sub(version_width), area.height);
        let version_area = Rect::new(area.x + area.width.saturating_sub(version_width), area.y, version_width, area.height);

        // Render keys on left
        Paragraph::new(Line::from(spans))
            .style(Style::default().bg(Color::Blue))
            .render(keys_area, buf);
        
        // Render version on right
        Paragraph::new(version_text)
            .style(Style::default().bg(Color::DarkGray).fg(Color::White))
            .render(version_area, buf);
    }
}
