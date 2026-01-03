use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Paragraph, Widget},
};

pub struct Footer;

// Returns (start_x, end_x, action_index) for each button
pub fn get_button_positions() -> Vec<(u16, u16, u8)> {
    let keys = vec![
        ("F1", "Files"),
        ("F2", "Preview"),
        ("F3", "Refresh"),
        ("F4", "Claude"),
        ("F5", "Git"),
        ("F6", "Term"),
        ("^P", "Find"),
        ("F9", "Menu"),
        ("?", "Help"),
    ];
    
    let mut positions = Vec::new();
    let mut x = 0u16;
    
    for (i, (key, desc)) in keys.iter().enumerate() {
        let key_width = format!(" {} ", key).len() as u16;
        let desc_width = format!(" {} ", desc).len() as u16;
        let total_width = key_width + desc_width + 1; // +1 for spacer
        
        positions.push((x, x + total_width, i as u8));
        x += total_width;
    }
    
    positions
}

impl Widget for Footer {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let keys = vec![
            ("F1", "Files"),
            ("F2", "Preview"),
            ("F3", "Refresh"),
            ("F4", "Claude"),
            ("F5", "Git"),
            ("F6", "Term"),
            ("^P", "Find"),
            ("F9", "Menu"),
            ("?", "Help"),
        ];

        let mut spans = Vec::new();
        for (key, desc) in keys {
            spans.push(Span::styled(
                format!(" {} ", key), 
                Style::default().bg(Color::Cyan).fg(Color::Black)
            ));
            spans.push(Span::styled(
                format!(" {} ", desc), 
                Style::default().bg(Color::Blue).fg(Color::White)
            ));
            spans.push(Span::raw(" "));
        }

        let version = env!("CARGO_PKG_VERSION");
        let version_text = format!(" v{} ", version);
        let version_width = version_text.len() as u16;
        
        let keys_area = Rect::new(area.x, area.y, area.width.saturating_sub(version_width), area.height);
        let version_area = Rect::new(area.x + area.width.saturating_sub(version_width), area.y, version_width, area.height);

        Paragraph::new(Line::from(spans))
            .style(Style::default().bg(Color::Blue))
            .render(keys_area, buf);
        
        Paragraph::new(version_text)
            .style(Style::default().bg(Color::DarkGray).fg(Color::White))
            .render(version_area, buf);
    }
}

