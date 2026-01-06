//! About/License dialog

use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
    Frame,
};

/// About dialog state
#[derive(Debug, Clone, Default)]
pub struct AboutState {
    pub visible: bool,
    pub scroll: usize,
    pub selected: usize,
    /// Cached popup area for mouse hit testing
    pub popup_area: Option<Rect>,
    pub list_area: Option<Rect>,
}

impl AboutState {
    pub fn open(&mut self) {
        self.visible = true;
        self.scroll = 0;
        self.selected = 0;
    }

    pub fn close(&mut self) {
        self.visible = false;
        self.popup_area = None;
        self.list_area = None;
    }

    pub fn scroll_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
            // Adjust scroll to keep selection visible
            if self.selected < self.scroll {
                self.scroll = self.selected;
            }
        }
    }

    pub fn scroll_down(&mut self) {
        if self.selected < component_count() - 1 {
            self.selected += 1;
        }
    }

    /// Handle mouse click at coordinates
    pub fn handle_click(&mut self, x: u16, y: u16) -> bool {
        if let Some(list_area) = self.list_area {
            if x >= list_area.x && x < list_area.x + list_area.width
                && y >= list_area.y && y < list_area.y + list_area.height
            {
                let relative_y = y.saturating_sub(list_area.y) as usize;
                let clicked_idx = self.scroll + relative_y;
                if clicked_idx < component_count() {
                    self.selected = clicked_idx;
                    return true;
                }
            }
        }
        false
    }

    /// Handle scroll in the about dialog
    pub fn handle_scroll(&mut self, down: bool, visible_height: usize) {
        let max_scroll = component_count().saturating_sub(visible_height);
        if down {
            self.scroll = (self.scroll + 1).min(max_scroll);
        } else {
            self.scroll = self.scroll.saturating_sub(1);
        }
    }

    /// Ensure selection is visible after scroll
    pub fn ensure_selection_visible(&mut self, visible_height: usize) {
        if self.selected < self.scroll {
            self.scroll = self.selected;
        } else if self.selected >= self.scroll + visible_height {
            self.scroll = self.selected.saturating_sub(visible_height - 1);
        }
    }
}

/// Get the number of components
pub fn component_count() -> usize {
    COMPONENTS.len()
}

/// Component license information
#[allow(dead_code)]
struct ComponentLicense {
    name: &'static str,
    version: &'static str,
    license: &'static str,
    url: &'static str,  // Reserved for future use (clickable links)
}

const COMPONENTS: &[ComponentLicense] = &[
    ComponentLicense {
        name: "ratatui",
        version: "0.30.0",
        license: "MIT",
        url: "https://github.com/ratatui/ratatui",
    },
    ComponentLicense {
        name: "crossterm",
        version: "0.28.1",
        license: "MIT",
        url: "https://github.com/crossterm-rs/crossterm",
    },
    ComponentLicense {
        name: "tokio",
        version: "1.42.0",
        license: "MIT",
        url: "https://github.com/tokio-rs/tokio",
    },
    ComponentLicense {
        name: "portable-pty",
        version: "0.8.1",
        license: "MIT",
        url: "https://github.com/wez/wezterm",
    },
    ComponentLicense {
        name: "vt100",
        version: "0.16",
        license: "MIT",
        url: "https://github.com/doy/vt100-rust",
    },
    ComponentLicense {
        name: "syntect",
        version: "5.2",
        license: "MIT",
        url: "https://github.com/trishume/syntect",
    },
    ComponentLicense {
        name: "tui-textarea",
        version: "0.7.0",
        license: "MIT",
        url: "https://github.com/rhysd/tui-textarea",
    },
    ComponentLicense {
        name: "tui-markdown",
        version: "0.3",
        license: "MIT",
        url: "https://github.com/joshka/tui-markdown",
    },
    ComponentLicense {
        name: "serde",
        version: "1.0",
        license: "MIT/Apache-2.0",
        url: "https://github.com/serde-rs/serde",
    },
    ComponentLicense {
        name: "serde_yaml",
        version: "0.9",
        license: "MIT/Apache-2.0",
        url: "https://github.com/dtolnay/serde-yaml",
    },
    ComponentLicense {
        name: "anyhow",
        version: "1.0",
        license: "MIT/Apache-2.0",
        url: "https://github.com/dtolnay/anyhow",
    },
    ComponentLicense {
        name: "clap",
        version: "4.5",
        license: "MIT/Apache-2.0",
        url: "https://github.com/clap-rs/clap",
    },
    ComponentLicense {
        name: "dirs",
        version: "5.0",
        license: "MIT/Apache-2.0",
        url: "https://github.com/dirs-dev/dirs-rs",
    },
    ComponentLicense {
        name: "pulldown-cmark",
        version: "0.10",
        license: "MIT",
        url: "https://github.com/pulldown-cmark/pulldown-cmark",
    },
];

/// Render the about dialog
pub fn render(frame: &mut Frame, area: Rect, state: &mut AboutState) {
    // Calculate centered popup area (60% width, 70% height)
    let popup_width = (area.width as f32 * 0.6) as u16;
    let popup_height = (area.height as f32 * 0.7) as u16;

    let popup_x = (area.width.saturating_sub(popup_width)) / 2;
    let popup_y = (area.height.saturating_sub(popup_height)) / 2;

    let popup_area = Rect::new(popup_x, popup_y, popup_width, popup_height);

    // Store popup area for mouse hit testing
    state.popup_area = Some(popup_area);

    // Clear background
    frame.render_widget(Clear, popup_area);

    let version = env!("CARGO_PKG_VERSION");
    let title = format!(" Claude Workbench v{} - About ", version);

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let inner = block.inner(popup_area);
    frame.render_widget(block, popup_area);

    // Split into header, content, and footer
    let chunks = Layout::vertical([
        Constraint::Length(6),  // Header
        Constraint::Min(1),     // Component list
        Constraint::Length(2),  // Footer
    ])
    .split(inner);

    // Header with copyright
    let header = Paragraph::new(vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("Copyright ", Style::default().fg(Color::DarkGray)),
            Span::styled("(c) 2025 Martin Schmid", Style::default().fg(Color::White)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("License: ", Style::default().fg(Color::DarkGray)),
            Span::styled("MIT License", Style::default().fg(Color::Green)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Open Source Components:", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        ]),
    ]);
    frame.render_widget(header, chunks[0]);

    // Component list with scroll
    let visible_height = chunks[1].height as usize;
    let max_scroll = COMPONENTS.len().saturating_sub(visible_height);
    let scroll = state.scroll.min(max_scroll);

    // Store list area for mouse hit testing
    state.list_area = Some(chunks[1]);

    // Ensure selection is visible
    state.ensure_selection_visible(visible_height);

    let items: Vec<ListItem> = COMPONENTS
        .iter()
        .enumerate()
        .skip(scroll)
        .take(visible_height)
        .map(|(idx, comp)| {
            let is_selected = idx == state.selected;
            let style = if is_selected {
                Style::default().bg(Color::DarkGray)
            } else {
                Style::default()
            };

            ListItem::new(Line::from(vec![
                Span::styled(
                    if is_selected { "▸ " } else { "  " },
                    style.fg(Color::Yellow),
                ),
                Span::styled(
                    format!("{:<13}", comp.name),
                    style.fg(Color::Cyan),
                ),
                Span::styled(
                    format!(" v{:<7}", comp.version),
                    style.fg(Color::DarkGray),
                ),
                Span::styled(
                    format!(" {:<14}", comp.license),
                    style.fg(Color::Green),
                ),
            ]))
        })
        .collect();

    let list = List::new(items);
    frame.render_widget(list, chunks[1]);

    // Scroll indicator
    if COMPONENTS.len() > visible_height {
        let indicator = format!(" [{}/{}] ", scroll + 1, COMPONENTS.len().saturating_sub(visible_height) + 1);
        let indicator_area = Rect::new(
            popup_area.x + popup_area.width - indicator.len() as u16 - 2,
            popup_area.y,
            indicator.len() as u16,
            1,
        );
        frame.render_widget(
            Paragraph::new(indicator).style(Style::default().fg(Color::DarkGray)),
            indicator_area,
        );
    }

    // Footer with navigation hint
    let footer = Paragraph::new(Line::from(vec![
        Span::styled("↑/↓", Style::default().fg(Color::Yellow)),
        Span::styled(" Scroll  ", Style::default().fg(Color::DarkGray)),
        Span::styled("Esc", Style::default().fg(Color::Yellow)),
        Span::styled(" Close", Style::default().fg(Color::DarkGray)),
    ]))
    .alignment(ratatui::layout::Alignment::Center);
    frame.render_widget(footer, chunks[2]);
}
