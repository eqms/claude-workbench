//! Update dialog for self-update functionality
//!
//! Shows update availability and allows user to trigger update.

use crate::update::{UpdateState, CURRENT_VERSION};
use ratatui::{
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

/// Cached button areas for mouse click detection
#[derive(Debug, Clone, Default)]
pub struct UpdateDialogAreas {
    pub popup_area: Option<Rect>,
    pub update_button_area: Option<Rect>,
    pub later_button_area: Option<Rect>,
}

/// Button selection in the update dialog
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum UpdateDialogButton {
    #[default]
    Update,
    Later,
}

impl UpdateDialogButton {
    pub fn toggle(&self) -> Self {
        match self {
            UpdateDialogButton::Update => UpdateDialogButton::Later,
            UpdateDialogButton::Later => UpdateDialogButton::Update,
        }
    }
}

/// Render the update dialog
///
/// Returns the button areas for mouse click detection.
pub fn render(
    frame: &mut Frame,
    area: Rect,
    state: &UpdateState,
    selected_button: UpdateDialogButton,
) -> UpdateDialogAreas {
    let mut areas = UpdateDialogAreas::default();

    // Calculate centered popup area
    // Make dialog larger for better readability of versions and error messages
    let has_notes = state.release_notes.is_some();
    let has_error = state.error.is_some();
    let popup_width = if has_notes || has_error {
        80u16.min(area.width.saturating_sub(4)) // 80 chars for notes/errors
    } else {
        70u16.min(area.width.saturating_sub(4)) // 70 chars default
    };
    let popup_height = if state.updating {
        12 // More space for progress messages
    } else if has_error {
        18 // Space for detailed error + context
    } else if has_notes {
        25 // Larger dialog for release notes
    } else {
        15
    };
    let popup_height = popup_height.min(area.height.saturating_sub(4));

    let popup_x = (area.width.saturating_sub(popup_width)) / 2;
    let popup_y = (area.height.saturating_sub(popup_height)) / 2;

    let popup_area = Rect::new(popup_x, popup_y, popup_width, popup_height);
    areas.popup_area = Some(popup_area);

    // Clear background
    frame.render_widget(Clear, popup_area);

    let title = if state.update_success {
        " Update Complete "
    } else if state.updating {
        " Updating... "
    } else if state.available_version.is_some() {
        " Update Available "
    } else if state.checking {
        " Checking for Updates... "
    } else if state.error.is_some() {
        " Update Check Failed "
    } else {
        " Up to Date "
    };

    let block = Block::default()
        .title(title)
        .title_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let inner = block.inner(popup_area);
    frame.render_widget(block, popup_area);

    // Content layout
    let chunks = Layout::vertical([
        Constraint::Min(1),    // Content
        Constraint::Length(3), // Buttons
    ])
    .split(inner);

    // Render content based on state (order matters!)
    if state.update_success {
        // Update completed successfully - show success message
        render_success(frame, chunks[0], state.installed_version.as_deref());
        render_close_button(frame, chunks[1]);
    } else if state.updating {
        render_updating(frame, chunks[0], state);
    } else if state.checking {
        render_checking(frame, chunks[0]);
    } else if let Some(ref error) = state.error {
        render_error(frame, chunks[0], error);
        render_close_button(frame, chunks[1]);
    } else if let Some(ref new_version) = state.available_version {
        render_update_available(
            frame,
            chunks[0],
            new_version,
            state.release_notes.as_deref(),
            state.release_notes_scroll,
        );
        areas = render_buttons(frame, chunks[1], selected_button, areas);
    } else {
        render_up_to_date(frame, chunks[0]);
        render_close_button(frame, chunks[1]);
    }

    areas
}

fn render_checking(frame: &mut Frame, area: Rect) {
    let content = Paragraph::new(vec![
        Line::from(""),
        Line::from(Span::styled(
            "Checking for updates...",
            Style::default().fg(Color::Yellow),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "Please wait",
            Style::default().fg(Color::DarkGray),
        )),
    ])
    .alignment(Alignment::Center);
    frame.render_widget(content, area);
}

fn render_updating(frame: &mut Frame, area: Rect, state: &UpdateState) {
    let message = state
        .progress_message
        .as_deref()
        .unwrap_or("Downloading update...");

    // Check if this is an error (contains "failed" or "error")
    let is_error = message.to_lowercase().contains("failed")
        || message.to_lowercase().contains("error");

    let color = if is_error { Color::Red } else { Color::Yellow };

    let hint = if is_error {
        "Press Esc to close and try again later."
    } else {
        "Please wait, do not close the application."
    };

    // Animated spinner based on current time (cycles every 100ms)
    let spinner_chars = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];
    let spinner_index = if !is_error {
        let millis = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0);
        (millis / 100) as usize % spinner_chars.len()
    } else {
        0
    };

    let progress_indicator = if is_error {
        "✗ ".to_string()
    } else {
        format!("{} ", spinner_chars[spinner_index])
    };

    let mut lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            format!("{}{}", progress_indicator, message),
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
    ];

    // Show animated progress bar for downloads
    if !is_error {
        let bar_width = 30;
        let progress_millis = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0);
        // Moving highlight effect (cycles through the bar)
        let pos = ((progress_millis / 50) % (bar_width as u128 * 2)) as usize;
        let mut bar = String::new();
        for i in 0..bar_width {
            let dist = if pos < bar_width {
                (i as i32 - pos as i32).unsigned_abs() as usize
            } else {
                let rev_pos = bar_width * 2 - pos;
                (i as i32 - rev_pos as i32).unsigned_abs() as usize
            };
            if dist == 0 {
                bar.push('█');
            } else if dist == 1 {
                bar.push('▓');
            } else if dist == 2 {
                bar.push('▒');
            } else {
                bar.push('░');
            }
        }
        lines.push(Line::from(Span::styled(
            format!("[{}]", bar),
            Style::default().fg(Color::Cyan),
        )));
        lines.push(Line::from(""));
    }

    lines.push(Line::from(Span::styled(
        hint,
        Style::default().fg(Color::DarkGray),
    )));

    let content = Paragraph::new(lines).alignment(Alignment::Center);
    frame.render_widget(content, area);
}

fn render_error(frame: &mut Frame, area: Rect, error: &str) {
    // Split error by newlines for multi-line display
    let mut lines: Vec<Line> = vec![
        Line::from(""),
        Line::from(Span::styled(
            "Update failed:",
            Style::default()
                .fg(Color::Red)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
    ];

    // Add each line of the error message (no truncation)
    for line in error.lines() {
        // Wrap long lines at word boundaries or force-wrap if needed
        let max_width = area.width.saturating_sub(4) as usize;
        if line.len() <= max_width {
            lines.push(Line::from(Span::styled(
                line.to_string(),
                Style::default().fg(Color::Yellow),
            )));
        } else {
            // Word-wrap long lines
            let mut current = String::new();
            for word in line.split_whitespace() {
                if current.is_empty() {
                    current = word.to_string();
                } else if current.len() + 1 + word.len() <= max_width {
                    current.push(' ');
                    current.push_str(word);
                } else {
                    lines.push(Line::from(Span::styled(
                        current,
                        Style::default().fg(Color::Yellow),
                    )));
                    current = word.to_string();
                }
            }
            if !current.is_empty() {
                lines.push(Line::from(Span::styled(
                    current,
                    Style::default().fg(Color::Yellow),
                )));
            }
        }
    }

    // Add hint at the bottom
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "Check network connection and try again.",
        Style::default().fg(Color::DarkGray),
    )));

    let content = Paragraph::new(lines).alignment(Alignment::Center);
    frame.render_widget(content, area);
}

fn render_success(frame: &mut Frame, area: Rect, new_version: Option<&str>) {
    let version_text = new_version.unwrap_or("latest");

    let content = Paragraph::new(vec![
        Line::from(""),
        Line::from(Span::styled(
            "✓ Update Successful!",
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("Installed Version: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("v{}", version_text),
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "Please restart the application",
            Style::default().fg(Color::Yellow),
        )),
        Line::from(Span::styled(
            "to use the new version.",
            Style::default().fg(Color::Yellow),
        )),
    ])
    .alignment(Alignment::Center);
    frame.render_widget(content, area);
}

fn render_up_to_date(frame: &mut Frame, area: Rect) {
    let content = Paragraph::new(vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("Current Version: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                CURRENT_VERSION,
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "You are running the latest version.",
            Style::default().fg(Color::Green),
        )),
    ])
    .alignment(Alignment::Center);
    frame.render_widget(content, area);
}

fn render_update_available(
    frame: &mut Frame,
    area: Rect,
    new_version: &str,
    release_notes: Option<&str>,
    scroll: u16,
) {
    // Calculate layout: header info, separator, scrollable release notes
    let chunks = Layout::vertical([
        Constraint::Length(4), // Version info
        Constraint::Min(1),    // Release notes
    ])
    .split(area);

    // Version info section
    let version_content = Paragraph::new(vec![
        Line::from(vec![
            Span::styled("Current: ", Style::default().fg(Color::DarkGray)),
            Span::styled(CURRENT_VERSION, Style::default().fg(Color::Yellow)),
        ]),
        Line::from(vec![
            Span::styled("New:     ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                new_version,
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
    ])
    .alignment(Alignment::Center);
    frame.render_widget(version_content, chunks[0]);

    // Release notes section
    if let Some(notes) = release_notes {
        let notes_lines: Vec<Line> = notes
            .lines()
            .map(|line| Line::from(Span::styled(line, Style::default().fg(Color::White))))
            .collect();

        let notes_block = Block::default()
            .title(" What's New ")
            .title_style(Style::default().fg(Color::Cyan))
            .borders(Borders::TOP)
            .border_style(Style::default().fg(Color::DarkGray));

        let notes_inner = notes_block.inner(chunks[1]);
        frame.render_widget(notes_block, chunks[1]);

        let notes_widget = Paragraph::new(notes_lines)
            .scroll((scroll, 0))
            .style(Style::default().fg(Color::Gray));

        frame.render_widget(notes_widget, notes_inner);
    } else {
        let no_notes = Paragraph::new(Line::from(Span::styled(
            "A new version is available!",
            Style::default().fg(Color::Cyan),
        )))
        .alignment(Alignment::Center);
        frame.render_widget(no_notes, chunks[1]);
    }
}

fn render_buttons(
    frame: &mut Frame,
    area: Rect,
    selected: UpdateDialogButton,
    mut areas: UpdateDialogAreas,
) -> UpdateDialogAreas {
    let button_width = 18u16;
    let total_width = button_width * 2 + 4;
    let start_x = area.x + (area.width.saturating_sub(total_width)) / 2;

    // Update button
    let update_style = if selected == UpdateDialogButton::Update {
        Style::default()
            .fg(Color::Black)
            .bg(Color::Green)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Green)
    };

    let update_area = Rect::new(start_x, area.y + 1, button_width, 1);
    areas.update_button_area = Some(update_area);

    let update_btn = Paragraph::new(" Update Now ")
        .style(update_style)
        .alignment(Alignment::Center);
    frame.render_widget(update_btn, update_area);

    // Later button
    let later_style = if selected == UpdateDialogButton::Later {
        Style::default()
            .fg(Color::Black)
            .bg(Color::DarkGray)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let later_area = Rect::new(start_x + button_width + 4, area.y + 1, button_width, 1);
    areas.later_button_area = Some(later_area);

    let later_btn = Paragraph::new(" Later ")
        .style(later_style)
        .alignment(Alignment::Center);
    frame.render_widget(later_btn, later_area);

    areas
}

fn render_close_button(frame: &mut Frame, area: Rect) {
    let hint = Paragraph::new(Line::from(vec![
        Span::styled("Press ", Style::default().fg(Color::DarkGray)),
        Span::styled("Esc", Style::default().fg(Color::Yellow)),
        Span::styled(" or ", Style::default().fg(Color::DarkGray)),
        Span::styled("Enter", Style::default().fg(Color::Yellow)),
        Span::styled(" to close", Style::default().fg(Color::DarkGray)),
    ]))
    .alignment(Alignment::Center);
    frame.render_widget(hint, Rect::new(area.x, area.y + 1, area.width, 1));
}

/// Check if a point is inside a button area
pub fn check_button_click(areas: &UpdateDialogAreas, x: u16, y: u16) -> Option<UpdateDialogButton> {
    if let Some(area) = areas.update_button_area {
        if x >= area.x && x < area.x + area.width && y >= area.y && y < area.y + area.height {
            return Some(UpdateDialogButton::Update);
        }
    }
    if let Some(area) = areas.later_button_area {
        if x >= area.x && x < area.x + area.width && y >= area.y && y < area.y + area.height {
            return Some(UpdateDialogButton::Later);
        }
    }
    None
}

/// Check if a point is inside the popup area
pub fn is_inside_popup(areas: &UpdateDialogAreas, x: u16, y: u16) -> bool {
    if let Some(area) = areas.popup_area {
        x >= area.x && x < area.x + area.width && y >= area.y && y < area.y + area.height
    } else {
        false
    }
}
