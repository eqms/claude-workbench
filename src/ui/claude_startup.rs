use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
    Frame,
};

use crate::config::ClaudePrefix;

/// State for the Claude startup prefix selection dialog
#[derive(Debug, Clone, Default)]
pub struct ClaudeStartupState {
    pub visible: bool,
    pub selected: usize,
    pub prefixes: Vec<ClaudePrefix>,
    pub shown_this_session: bool,
}

impl ClaudeStartupState {
    /// Open the dialog with the given prefixes
    pub fn open(&mut self, prefixes: Vec<ClaudePrefix>) {
        if !prefixes.is_empty() && !self.shown_this_session {
            self.visible = true;
            self.prefixes = prefixes;
            self.selected = 0;
        }
    }

    /// Close the dialog
    pub fn close(&mut self) {
        self.visible = false;
        self.shown_this_session = true;
    }

    /// Move selection up
    pub fn prev(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

    /// Move selection down
    pub fn next(&mut self) {
        if self.selected < self.prefixes.len().saturating_sub(1) {
            self.selected += 1;
        }
    }

    /// Get the selected prefix string (or empty if "Plain Start")
    pub fn selected_prefix(&self) -> Option<&str> {
        self.prefixes.get(self.selected).map(|p| p.prefix.as_str())
    }

    /// Check if there are prefixes configured
    pub fn has_prefixes(&self) -> bool {
        !self.prefixes.is_empty()
    }
}

/// Render the Claude startup dialog
pub fn render(frame: &mut Frame, area: Rect, state: &ClaudeStartupState) {
    if !state.visible || state.prefixes.is_empty() {
        return;
    }

    // Calculate popup size - width based on content
    let max_name_len = state.prefixes.iter().map(|p| p.name.len()).max().unwrap_or(10);
    let max_desc_len = state.prefixes.iter().map(|p| p.description.len()).max().unwrap_or(20);
    let content_width = (max_name_len + max_desc_len + 10).min(60) as u16;
    let popup_width = content_width.max(40);
    let popup_height = (state.prefixes.len() as u16 + 6).min(20);

    // Center the popup
    let popup_x = area.x + (area.width.saturating_sub(popup_width)) / 2;
    let popup_y = area.y + (area.height.saturating_sub(popup_height)) / 2;
    let popup_area = Rect::new(popup_x, popup_y, popup_width, popup_height);

    // Clear background
    frame.render_widget(Clear, popup_area);

    // Create layout
    let chunks = Layout::vertical([
        Constraint::Length(2), // Title
        Constraint::Min(1),    // List
        Constraint::Length(2), // Footer
    ])
    .split(popup_area);

    // Main block with border
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .title(" Claude Startup ")
        .title_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD));
    frame.render_widget(block, popup_area);

    // Title area
    let title_area = Rect::new(
        chunks[0].x + 1,
        chunks[0].y + 1,
        chunks[0].width.saturating_sub(2),
        1,
    );
    let title = Paragraph::new("Select an action:")
        .style(Style::default().fg(Color::White));
    frame.render_widget(title, title_area);

    // List area
    let list_area = Rect::new(
        chunks[1].x + 1,
        chunks[1].y,
        chunks[1].width.saturating_sub(2),
        chunks[1].height,
    );

    let items: Vec<ListItem> = state
        .prefixes
        .iter()
        .enumerate()
        .map(|(i, prefix)| {
            let is_selected = i == state.selected;
            let selector = if is_selected { "▸ " } else { "  " };

            let line = Line::from(vec![
                Span::styled(
                    selector,
                    Style::default().fg(if is_selected { Color::Yellow } else { Color::DarkGray }),
                ),
                Span::styled(
                    format!("{:<15}", prefix.name),
                    Style::default()
                        .fg(if is_selected { Color::Yellow } else { Color::White })
                        .add_modifier(if is_selected { Modifier::BOLD } else { Modifier::empty() }),
                ),
                Span::styled(
                    " - ",
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled(
                    &prefix.description,
                    Style::default().fg(Color::Gray),
                ),
            ]);

            ListItem::new(line)
        })
        .collect();

    let list = List::new(items);
    frame.render_widget(list, list_area);

    // Footer with controls
    let footer_area = Rect::new(
        chunks[2].x + 1,
        chunks[2].y,
        chunks[2].width.saturating_sub(2),
        1,
    );
    let footer = Line::from(vec![
        Span::styled(" Enter ", Style::default().bg(Color::Cyan).fg(Color::Black)),
        Span::raw(" Select  "),
        Span::styled(" Esc ", Style::default().bg(Color::DarkGray).fg(Color::White)),
        Span::raw(" Skip  "),
        Span::styled(" ↑↓ ", Style::default().bg(Color::DarkGray).fg(Color::White)),
        Span::raw(" Navigate"),
    ]);
    frame.render_widget(Paragraph::new(footer), footer_area);
}
