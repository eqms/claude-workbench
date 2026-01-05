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
}

impl AboutState {
    pub fn open(&mut self) {
        self.visible = true;
        self.scroll = 0;
    }

    pub fn close(&mut self) {
        self.visible = false;
    }

    pub fn scroll_up(&mut self) {
        self.scroll = self.scroll.saturating_sub(1);
    }

    pub fn scroll_down(&mut self) {
        self.scroll = self.scroll.saturating_add(1);
    }
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
];

/// Render the about dialog
pub fn render(frame: &mut Frame, area: Rect, state: &AboutState) {
    // Calculate centered popup area (60% width, 70% height)
    let popup_width = (area.width as f32 * 0.6) as u16;
    let popup_height = (area.height as f32 * 0.7) as u16;

    let popup_x = (area.width.saturating_sub(popup_width)) / 2;
    let popup_y = (area.height.saturating_sub(popup_height)) / 2;

    let popup_area = Rect::new(popup_x, popup_y, popup_width, popup_height);

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
            Span::styled("(c) 2025 Equitania Software GmbH", Style::default().fg(Color::White)),
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

    let items: Vec<ListItem> = COMPONENTS
        .iter()
        .skip(scroll)
        .take(visible_height)
        .map(|comp| {
            ListItem::new(Line::from(vec![
                Span::styled(
                    format!("{:<15}", comp.name),
                    Style::default().fg(Color::Cyan),
                ),
                Span::styled(
                    format!(" v{:<8}", comp.version),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled(
                    format!(" {:<14}", comp.license),
                    Style::default().fg(Color::Green),
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
