use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
    Frame,
};

use crate::types::ClaudePermissionMode;

/// State for the Claude permission mode selection dialog
#[derive(Debug, Clone, Default)]
pub struct PermissionModeState {
    pub visible: bool,
    pub selected: usize,
    pub confirmed: bool,
    /// Remote control toggle state
    pub remote_control: bool,
    /// Last highlighted mode index (for when cursor is on toggle row)
    last_mode_index: usize,
}

impl PermissionModeState {
    /// Open the dialog
    pub fn open(&mut self) {
        self.open_with_default(None, false);
    }

    /// Open the dialog with a pre-selected default mode and remote control state
    pub fn open_with_default(
        &mut self,
        default: Option<ClaudePermissionMode>,
        remote_control: bool,
    ) {
        self.visible = true;
        self.confirmed = false;
        self.remote_control = remote_control;
        let idx = default
            .and_then(|mode| ClaudePermissionMode::all().iter().position(|m| *m == mode))
            .unwrap_or(0);
        self.selected = idx;
        self.last_mode_index = idx;
    }

    /// Close the dialog without confirming
    pub fn close(&mut self) {
        self.visible = false;
        self.confirmed = false;
    }

    /// Confirm selection and close
    pub fn confirm(&mut self) {
        self.confirmed = true;
        self.visible = false;
    }

    /// Move selection up
    pub fn prev(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
            if !self.is_on_toggle() {
                self.last_mode_index = self.selected;
            }
        }
    }

    /// Move selection down
    pub fn next(&mut self) {
        let max_index = ClaudePermissionMode::all().len(); // modes.len() = toggle row
        if self.selected < max_index {
            self.selected += 1;
            if !self.is_on_toggle() {
                self.last_mode_index = self.selected;
            }
        }
    }

    /// Check if the remote control toggle row is selected
    pub fn is_on_toggle(&self) -> bool {
        self.selected == ClaudePermissionMode::all().len()
    }

    /// Toggle remote control state (only when on toggle row)
    pub fn toggle_remote_control(&mut self) {
        if self.is_on_toggle() {
            self.remote_control = !self.remote_control;
        }
    }

    /// Get the selected permission mode
    pub fn selected_mode(&self) -> ClaudePermissionMode {
        let modes = ClaudePermissionMode::all();
        let idx = if self.is_on_toggle() {
            self.last_mode_index
        } else {
            self.selected
        };
        modes.get(idx).copied().unwrap_or_default()
    }
}

/// Render the permission mode selection dialog
pub fn render(frame: &mut Frame, area: Rect, state: &PermissionModeState) {
    if !state.visible {
        return;
    }

    let modes = ClaudePermissionMode::all();

    // Calculate popup size (extra space for separator + toggle + spacing)
    let popup_width: u16 = 68;
    let popup_height: u16 = (modes.len() as u16 + 11).min(22);

    // Center the popup
    let popup_x = area.x + (area.width.saturating_sub(popup_width)) / 2;
    let popup_y = area.y + (area.height.saturating_sub(popup_height)) / 2;
    let popup_area = Rect::new(popup_x, popup_y, popup_width, popup_height);

    // Clear background
    frame.render_widget(Clear, popup_area);

    // Create layout with toggle section
    let chunks = Layout::vertical([
        Constraint::Length(3), // Title + instruction
        Constraint::Min(1),    // Mode list
        Constraint::Length(3), // Separator + toggle
        Constraint::Length(2), // Footer
    ])
    .split(popup_area);

    // Main block with border
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .title(" Claude Code Permission Mode ")
        .title_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        );
    frame.render_widget(block, popup_area);

    // Title/instruction area
    let title_area = Rect::new(
        chunks[0].x + 2,
        chunks[0].y + 1,
        chunks[0].width.saturating_sub(4),
        2,
    );
    let title = Paragraph::new("Wähle den Berechtigungsmodus für Claude Code:")
        .style(Style::default().fg(Color::White));
    frame.render_widget(title, title_area);

    // List area
    let list_area = Rect::new(
        chunks[1].x + 2,
        chunks[1].y,
        chunks[1].width.saturating_sub(4),
        chunks[1].height,
    );

    let items: Vec<ListItem> = modes
        .iter()
        .enumerate()
        .map(|(i, mode)| {
            let is_selected = i == state.selected;
            let selector = if is_selected { "▸ " } else { "  " };
            let is_yolo = mode.is_yolo();

            let name_style = if is_yolo {
                Style::default()
                    .fg(Color::Red)
                    .add_modifier(if is_selected {
                        Modifier::BOLD
                    } else {
                        Modifier::empty()
                    })
            } else {
                Style::default()
                    .fg(if is_selected {
                        Color::Yellow
                    } else {
                        Color::White
                    })
                    .add_modifier(if is_selected {
                        Modifier::BOLD
                    } else {
                        Modifier::empty()
                    })
            };

            let desc_style = if is_yolo {
                Style::default().fg(Color::Red)
            } else {
                Style::default().fg(Color::Gray)
            };

            let line = Line::from(vec![
                Span::styled(
                    selector,
                    Style::default().fg(if is_selected {
                        Color::Yellow
                    } else {
                        Color::DarkGray
                    }),
                ),
                Span::styled(format!("{:<18}", mode.name()), name_style),
                Span::styled(" - ", Style::default().fg(Color::DarkGray)),
                Span::styled(mode.description_de(), desc_style),
            ]);

            ListItem::new(line)
        })
        .collect();

    let list = List::new(items);
    frame.render_widget(list, list_area);

    // Toggle section: separator line + remote control checkbox
    let toggle_area = Rect::new(
        chunks[2].x + 1,
        chunks[2].y,
        chunks[2].width.saturating_sub(2),
        chunks[2].height,
    );

    let on_toggle = state.is_on_toggle();
    let separator = "─".repeat(toggle_area.width.saturating_sub(1) as usize);
    let checkbox = if state.remote_control { "x" } else { " " };
    let selector = if on_toggle { "▸ " } else { "  " };

    let toggle_style = Style::default()
        .fg(if on_toggle {
            Color::Yellow
        } else {
            Color::White
        })
        .add_modifier(if on_toggle {
            Modifier::BOLD
        } else {
            Modifier::empty()
        });

    let toggle_lines = vec![
        Line::from(Span::styled(
            separator,
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(vec![
            Span::styled(
                selector,
                Style::default().fg(if on_toggle {
                    Color::Yellow
                } else {
                    Color::DarkGray
                }),
            ),
            Span::styled(format!("[{}] ", checkbox), toggle_style),
            Span::styled("Remote Control", toggle_style),
            Span::styled(
                " - Session von anderen Geräten",
                Style::default().fg(Color::Gray),
            ),
        ]),
    ];
    let toggle_paragraph = Paragraph::new(toggle_lines);
    frame.render_widget(toggle_paragraph, toggle_area);

    // Footer with controls
    let footer_area = Rect::new(
        chunks[3].x + 2,
        chunks[3].y,
        chunks[3].width.saturating_sub(4),
        1,
    );
    let footer = Line::from(vec![
        Span::styled(" Enter ", Style::default().bg(Color::Cyan).fg(Color::Black)),
        Span::raw(" Wählen  "),
        Span::styled(
            " Space ",
            Style::default().bg(Color::DarkGray).fg(Color::White),
        ),
        Span::raw(" Toggle  "),
        Span::styled(
            " Esc ",
            Style::default().bg(Color::DarkGray).fg(Color::White),
        ),
        Span::raw(" Standard  "),
        Span::styled(
            " ↑↓ ",
            Style::default().bg(Color::DarkGray).fg(Color::White),
        ),
        Span::raw(" Nav"),
    ]);
    frame.render_widget(Paragraph::new(footer), footer_area);
}
