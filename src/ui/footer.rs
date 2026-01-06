use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Paragraph, Widget},
};
use std::time::SystemTime;

use crate::types::{EditorMode, PaneId};

pub struct Footer {
    pub active_pane: PaneId,
    pub editor_mode: EditorMode,
    pub editor_modified: bool,
    pub selection_mode: bool,
}

/// Format current date/time for footer display
fn format_datetime() -> String {
    let now = SystemTime::now();
    let datetime = now.duration_since(SystemTime::UNIX_EPOCH).unwrap_or_default();
    let secs = datetime.as_secs();

    // Convert to local time components (simplified UTC-based calculation)
    let days = secs / 86400;
    let time_secs = secs % 86400;
    let hours = time_secs / 3600;
    let minutes = (time_secs % 3600) / 60;
    let seconds = time_secs % 60;

    // Calculate date from days since Unix epoch (1970-01-01)
    let (year, month, day) = days_to_date(days as i64);

    format!("{:02}.{:02}.{} {:02}:{:02}:{:02}", day, month, year, hours, minutes, seconds)
}

/// Convert days since Unix epoch to (year, month, day)
fn days_to_date(days: i64) -> (i32, u32, u32) {
    // Days since epoch (1970-01-01)
    let remaining = days + 719468; // Days from year 0 to 1970-01-01

    let era = if remaining >= 0 { remaining / 146097 } else { (remaining - 146096) / 146097 };
    let doe = (remaining - era * 146097) as u32;
    let yoe = (doe - doe/1460 + doe/36524 - doe/146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365*yoe + yoe/4 - yoe/100);
    let mp = (5*doy + 2) / 153;
    let d = doy - (153*mp + 2)/5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };

    (y as i32, m, d)
}

impl Default for Footer {
    fn default() -> Self {
        Self {
            active_pane: PaneId::FileBrowser,
            editor_mode: EditorMode::ReadOnly,
            editor_modified: false,
            selection_mode: false,
        }
    }
}

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
        ("o", "Open"),
        ("O", "Finder"),
        ("^,", "Settings"),
        ("F10", "Info"),
        ("F12", "Help"),
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
        // Context-dependent key display
        let keys: Vec<(&str, &str)> = if self.selection_mode {
            // Terminal selection mode keys
            vec![
                ("j/↓", "Down"),
                ("k/↑", "Up"),
                ("Enter", "Copy"),
                ("y", "Copy"),
                ("Esc", "Cancel"),
            ]
        } else if self.active_pane == PaneId::Preview && self.editor_mode == EditorMode::Edit {
            // Edit mode keys
            vec![
                ("^S", "Save"),
                ("Esc", "Exit"),
                ("^Z", "Undo"),
                ("^Y", "Redo"),
                ("F1", "Files"),
                ("F4", "Claude"),
                ("F12", "Help"),
            ]
        } else if self.active_pane == PaneId::Preview {
            // Preview mode - show Edit option
            vec![
                ("E", "Edit"),
                ("F1", "Files"),
                ("F3", "Refresh"),
                ("F4", "Claude"),
                ("F5", "Git"),
                ("F6", "Term"),
                ("^P", "Find"),
                ("F12", "Help"),
            ]
        } else if matches!(self.active_pane, PaneId::Claude | PaneId::LazyGit | PaneId::Terminal) {
            // Terminal pane keys - show ^S for selection mode
            vec![
                ("^S", "Select"),
                ("F1", "Files"),
                ("F2", "Preview"),
                ("F3", "Refresh"),
                ("F4", "Claude"),
                ("F5", "Git"),
                ("F6", "Term"),
                ("^P", "Find"),
                ("F12", "Help"),
            ]
        } else {
            // Default keys (file browser)
            vec![
                ("F1", "Files"),
                ("F2", "Preview"),
                ("F3", "Refresh"),
                ("F4", "Claude"),
                ("F5", "Git"),
                ("F6", "Term"),
                ("^P", "Find"),
                ("o", "Open"),
                ("O", "Finder"),
                ("^,", "Settings"),
                ("F10", "Info"),
                ("F12", "Help"),
            ]
        };

        let mut spans = Vec::new();

        // Show modified indicator in edit mode
        if self.editor_mode == EditorMode::Edit && self.editor_modified {
            spans.push(Span::styled(
                " [+] ",
                Style::default().bg(Color::Yellow).fg(Color::Black)
            ));
        }

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

        // Right side: datetime + version
        let datetime_text = format_datetime();
        let version = env!("CARGO_PKG_VERSION");
        let right_text = format!(" {} │ v{} ", datetime_text, version);
        let right_width = right_text.len() as u16;

        let keys_area = Rect::new(area.x, area.y, area.width.saturating_sub(right_width), area.height);
        let right_area = Rect::new(area.x + area.width.saturating_sub(right_width), area.y, right_width, area.height);

        Paragraph::new(Line::from(spans))
            .style(Style::default().bg(Color::Blue))
            .render(keys_area, buf);

        Paragraph::new(right_text)
            .style(Style::default().bg(Color::DarkGray).fg(Color::White))
            .render(right_area, buf);
    }
}

