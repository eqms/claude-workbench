use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
    Frame,
};

use crate::types::{ClaudeEffort, ClaudeModel, ClaudePermissionMode};

/// Sections inside the Claude startup dialog. Navigated via Tab / Shift+Tab.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DialogSection {
    #[default]
    Permission,
    Model,
    Effort,
    Session,
    Worktree,
    RemoteControl,
}

impl DialogSection {
    fn next(self) -> Self {
        match self {
            Self::Permission => Self::Model,
            Self::Model => Self::Effort,
            Self::Effort => Self::Session,
            Self::Session => Self::Worktree,
            Self::Worktree => Self::RemoteControl,
            Self::RemoteControl => Self::Permission,
        }
    }

    fn prev(self) -> Self {
        match self {
            Self::Permission => Self::RemoteControl,
            Self::Model => Self::Permission,
            Self::Effort => Self::Model,
            Self::Session => Self::Effort,
            Self::Worktree => Self::Session,
            Self::RemoteControl => Self::Worktree,
        }
    }
}

/// Unified state for the Claude startup dialog (v0.81.0+).
///
/// Combines permission mode, model, effort, session name, worktree and
/// remote control into one vertical multi-section dialog.
#[derive(Debug, Clone, Default)]
pub struct PermissionModeState {
    pub visible: bool,
    pub confirmed: bool,
    pub section: DialogSection,

    pub permission_selected: usize,
    pub model_selected: usize,
    pub effort_selected: usize,

    pub session_name: String,
    pub session_cursor: usize,

    pub worktree: String,
    pub worktree_cursor: usize,

    pub remote_control: bool,
}

impl PermissionModeState {
    /// Legacy alias for callers that don't have all defaults yet.
    pub fn open(&mut self) {
        self.open_with_defaults(None, ClaudeModel::Unset, ClaudeEffort::Unset, "", "", false);
    }

    /// Open dialog with full set of default values (called from pty.rs).
    pub fn open_with_defaults(
        &mut self,
        default_mode: Option<ClaudePermissionMode>,
        default_model: ClaudeModel,
        default_effort: ClaudeEffort,
        default_session_name: &str,
        default_worktree: &str,
        remote_control: bool,
    ) {
        self.visible = true;
        self.confirmed = false;
        self.section = DialogSection::Permission;

        self.permission_selected = default_mode
            .and_then(|m| ClaudePermissionMode::all().iter().position(|x| *x == m))
            .unwrap_or(0);
        self.model_selected = ClaudeModel::all()
            .iter()
            .position(|m| *m == default_model)
            .unwrap_or(0);
        self.effort_selected = ClaudeEffort::all()
            .iter()
            .position(|e| *e == default_effort)
            .unwrap_or(0);

        self.session_name = default_session_name.to_string();
        self.session_cursor = self.session_name.chars().count();

        self.worktree = default_worktree.to_string();
        self.worktree_cursor = self.worktree.chars().count();

        self.remote_control = remote_control;
    }

    pub fn close(&mut self) {
        self.visible = false;
        self.confirmed = false;
    }

    pub fn confirm(&mut self) {
        self.confirmed = true;
        self.visible = false;
    }

    // ── Section navigation ────────────────────────────────────────────────

    pub fn next_section(&mut self) {
        self.section = self.section.next();
    }

    pub fn prev_section(&mut self) {
        self.section = self.section.prev();
    }

    // ── Intra-section navigation (↑↓ for lists, ←→ for radios) ───────────

    pub fn prev_item(&mut self) {
        match self.section {
            DialogSection::Permission if self.permission_selected > 0 => {
                self.permission_selected -= 1;
            }
            DialogSection::Model if self.model_selected > 0 => {
                self.model_selected -= 1;
            }
            DialogSection::Effort if self.effort_selected > 0 => {
                self.effort_selected -= 1;
            }
            _ => {}
        }
    }

    pub fn next_item(&mut self) {
        match self.section {
            DialogSection::Permission
                if self.permission_selected + 1 < ClaudePermissionMode::all().len() =>
            {
                self.permission_selected += 1;
            }
            DialogSection::Model if self.model_selected + 1 < ClaudeModel::all().len() => {
                self.model_selected += 1;
            }
            DialogSection::Effort if self.effort_selected + 1 < ClaudeEffort::all().len() => {
                self.effort_selected += 1;
            }
            _ => {}
        }
    }

    // ── Text input (Session + Worktree) with UTF-8-safe cursor ───────────

    fn active_text_mut(&mut self) -> Option<(&mut String, &mut usize)> {
        match self.section {
            DialogSection::Session => Some((&mut self.session_name, &mut self.session_cursor)),
            DialogSection::Worktree => Some((&mut self.worktree, &mut self.worktree_cursor)),
            _ => None,
        }
    }

    pub fn insert_char(&mut self, c: char) {
        if let Some((text, cursor)) = self.active_text_mut() {
            let byte_pos: usize = text.chars().take(*cursor).map(|c| c.len_utf8()).sum();
            text.insert(byte_pos, c);
            *cursor += 1;
        }
    }

    pub fn delete_char_before(&mut self) {
        if let Some((text, cursor)) = self.active_text_mut() {
            if *cursor == 0 {
                return;
            }
            let byte_start: usize = text.chars().take(*cursor - 1).map(|c| c.len_utf8()).sum();
            if let Some(ch) = text.chars().nth(*cursor - 1) {
                let byte_end = byte_start + ch.len_utf8();
                text.replace_range(byte_start..byte_end, "");
                *cursor -= 1;
            }
        }
    }

    pub fn delete_char_at(&mut self) {
        if let Some((text, cursor)) = self.active_text_mut() {
            let len = text.chars().count();
            if *cursor >= len {
                return;
            }
            let byte_start: usize = text.chars().take(*cursor).map(|c| c.len_utf8()).sum();
            if let Some(ch) = text.chars().nth(*cursor) {
                let byte_end = byte_start + ch.len_utf8();
                text.replace_range(byte_start..byte_end, "");
            }
        }
    }

    pub fn cursor_left(&mut self) {
        if let Some((_, cursor)) = self.active_text_mut() {
            if *cursor > 0 {
                *cursor -= 1;
            }
        }
    }

    pub fn cursor_right(&mut self) {
        if let Some((text, cursor)) = self.active_text_mut() {
            let len = text.chars().count();
            if *cursor < len {
                *cursor += 1;
            }
        }
    }

    pub fn cursor_home(&mut self) {
        if let Some((_, cursor)) = self.active_text_mut() {
            *cursor = 0;
        }
    }

    pub fn cursor_end(&mut self) {
        if let Some((text, cursor)) = self.active_text_mut() {
            *cursor = text.chars().count();
        }
    }

    pub fn toggle_remote_control(&mut self) {
        if self.section == DialogSection::RemoteControl {
            self.remote_control = !self.remote_control;
        }
    }

    // ── Accessors ────────────────────────────────────────────────────────

    pub fn selected_permission_mode(&self) -> ClaudePermissionMode {
        ClaudePermissionMode::all()
            .get(self.permission_selected)
            .copied()
            .unwrap_or_default()
    }

    pub fn selected_model(&self) -> ClaudeModel {
        ClaudeModel::all()
            .get(self.model_selected)
            .copied()
            .unwrap_or_default()
    }

    pub fn selected_effort(&self) -> ClaudeEffort {
        ClaudeEffort::all()
            .get(self.effort_selected)
            .copied()
            .unwrap_or_default()
    }

    pub fn is_text_field_active(&self) -> bool {
        matches!(
            self.section,
            DialogSection::Session | DialogSection::Worktree
        )
    }
}

// ─── Render ──────────────────────────────────────────────────────────────

pub fn render(frame: &mut Frame, area: Rect, state: &PermissionModeState) {
    if !state.visible {
        return;
    }

    let modes = ClaudePermissionMode::all();
    let models = ClaudeModel::all();
    let efforts = ClaudeEffort::all();

    let popup_width: u16 = 76;
    // Dynamic height: title(2) + permission-list(6) + sep+header+model(3)
    //                 + sep+header+effort(3) + sep+header+session(3)
    //                 + worktree(2) + sep+header+remote(3) + footer(2) + borders(2)
    let popup_height: u16 = (modes.len() as u16 + 19).min(area.height.saturating_sub(2));

    let popup_x = area.x + (area.width.saturating_sub(popup_width)) / 2;
    let popup_y = area.y + (area.height.saturating_sub(popup_height)) / 2;
    let popup_area = Rect::new(popup_x, popup_y, popup_width, popup_height);

    frame.render_widget(Clear, popup_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .title(" Claude Code Startup ")
        .title_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        );
    frame.render_widget(block, popup_area);

    let chunks = Layout::vertical([
        Constraint::Length(2),                      // Title
        Constraint::Length(modes.len() as u16 + 1), // Permission list + header
        Constraint::Length(2),                      // Model (header + row)
        Constraint::Length(2),                      // Effort (header + row)
        Constraint::Length(3),                      // Session (header + 2 rows: Name + Worktree)
        Constraint::Length(2),                      // Remote Control (header + row)
        Constraint::Min(2),                         // Footer
    ])
    .split(popup_area.inner(ratatui::layout::Margin {
        horizontal: 2,
        vertical: 1,
    }));

    render_title(frame, chunks[0]);
    render_permission_section(frame, chunks[1], state, modes);
    render_model_section(frame, chunks[2], state, models);
    render_effort_section(frame, chunks[3], state, efforts);
    render_session_section(frame, chunks[4], state);
    render_remote_section(frame, chunks[5], state);
    render_footer(frame, chunks[6]);
}

fn render_title(frame: &mut Frame, area: Rect) {
    let text = Paragraph::new("Claude Code Startup-Optionen — Tab wechselt Sektion:")
        .style(Style::default().fg(Color::White));
    frame.render_widget(text, area);
}

fn render_permission_section(
    frame: &mut Frame,
    area: Rect,
    state: &PermissionModeState,
    modes: &[ClaudePermissionMode],
) {
    let is_active = state.section == DialogSection::Permission;
    let header_style = section_header_style(is_active);
    let header = Line::from(vec![
        Span::styled("[ Permission Mode ]", header_style),
        Span::styled(
            if is_active { "  ↑↓ wählen" } else { "" },
            Style::default().fg(Color::DarkGray),
        ),
    ]);

    let mut lines: Vec<ListItem> = Vec::new();
    lines.push(ListItem::new(header));

    for (i, mode) in modes.iter().enumerate() {
        let is_selected = i == state.permission_selected;
        let is_yolo = mode.is_yolo();
        let selector = if is_selected && is_active {
            "▸ "
        } else if is_selected {
            "● "
        } else {
            "  "
        };

        let name_color = if is_yolo {
            Color::Red
        } else if is_selected {
            Color::Yellow
        } else {
            Color::White
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
            Span::styled(
                format!("{:<18}", mode.name()),
                Style::default()
                    .fg(name_color)
                    .add_modifier(if is_selected {
                        Modifier::BOLD
                    } else {
                        Modifier::empty()
                    }),
            ),
            Span::styled(" - ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                mode.description_de(),
                Style::default().fg(if is_yolo { Color::Red } else { Color::Gray }),
            ),
        ]);
        lines.push(ListItem::new(line));
    }

    frame.render_widget(List::new(lines), area);
}

fn render_model_section(
    frame: &mut Frame,
    area: Rect,
    state: &PermissionModeState,
    models: &[ClaudeModel],
) {
    let is_active = state.section == DialogSection::Model;
    let chunks = Layout::vertical([Constraint::Length(1), Constraint::Length(1)]).split(area);

    let header = Paragraph::new(Line::from(vec![
        Span::styled("[ Model ]", section_header_style(is_active)),
        Span::styled(
            if is_active { "  ←→ wählen" } else { "" },
            Style::default().fg(Color::DarkGray),
        ),
    ]));
    frame.render_widget(header, chunks[0]);

    let mut spans: Vec<Span> = Vec::new();
    for (i, model) in models.iter().enumerate() {
        let is_selected = i == state.model_selected;
        let marker = if is_selected { "(•)" } else { "( )" };
        let style = if is_selected {
            Style::default()
                .fg(if is_active {
                    Color::Yellow
                } else {
                    Color::White
                })
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Gray)
        };
        spans.push(Span::styled(
            format!("  {} {}", marker, model.name()),
            style,
        ));
    }
    frame.render_widget(Paragraph::new(Line::from(spans)), chunks[1]);
}

fn render_effort_section(
    frame: &mut Frame,
    area: Rect,
    state: &PermissionModeState,
    efforts: &[ClaudeEffort],
) {
    let is_active = state.section == DialogSection::Effort;
    let chunks = Layout::vertical([Constraint::Length(1), Constraint::Length(1)]).split(area);

    let header = Paragraph::new(Line::from(vec![
        Span::styled("[ Effort ]", section_header_style(is_active)),
        Span::styled(
            if is_active { "  ←→ wählen" } else { "" },
            Style::default().fg(Color::DarkGray),
        ),
    ]));
    frame.render_widget(header, chunks[0]);

    let mut spans: Vec<Span> = Vec::new();
    for (i, effort) in efforts.iter().enumerate() {
        let is_selected = i == state.effort_selected;
        let marker = if is_selected { "(•)" } else { "( )" };
        let style = if is_selected {
            Style::default()
                .fg(if is_active {
                    Color::Yellow
                } else {
                    Color::White
                })
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Gray)
        };
        spans.push(Span::styled(
            format!(" {} {}", marker, effort.name()),
            style,
        ));
    }
    frame.render_widget(Paragraph::new(Line::from(spans)), chunks[1]);
}

fn render_session_section(frame: &mut Frame, area: Rect, state: &PermissionModeState) {
    let session_active = state.section == DialogSection::Session;
    let worktree_active = state.section == DialogSection::Worktree;
    let either_active = session_active || worktree_active;

    let chunks = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
    ])
    .split(area);

    let header = Paragraph::new(Line::from(vec![
        Span::styled("[ Session ]", section_header_style(either_active)),
        Span::styled(
            if either_active {
                "  Tab wechselt Feld"
            } else {
                ""
            },
            Style::default().fg(Color::DarkGray),
        ),
    ]));
    frame.render_widget(header, chunks[0]);

    frame.render_widget(
        text_field_line(
            "Name:    ",
            &state.session_name,
            state.session_cursor,
            session_active,
        ),
        chunks[1],
    );
    frame.render_widget(
        text_field_line(
            "Worktree:",
            &state.worktree,
            state.worktree_cursor,
            worktree_active,
        ),
        chunks[2],
    );
}

fn text_field_line<'a>(
    label: &'a str,
    text: &'a str,
    cursor: usize,
    active: bool,
) -> Paragraph<'a> {
    let label_style = if active {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Gray)
    };

    // Render text with visible cursor if active
    let mut text_spans: Vec<Span> = Vec::new();
    if active {
        let chars: Vec<char> = text.chars().collect();
        for (i, ch) in chars.iter().enumerate() {
            if i == cursor {
                text_spans.push(Span::styled(
                    ch.to_string(),
                    Style::default().bg(Color::Yellow).fg(Color::Black),
                ));
            } else {
                text_spans.push(Span::styled(
                    ch.to_string(),
                    Style::default().fg(Color::White),
                ));
            }
        }
        // Cursor at end of text
        if cursor >= chars.len() {
            text_spans.push(Span::styled(
                " ",
                Style::default().bg(Color::Yellow).fg(Color::Black),
            ));
        }
    } else if text.is_empty() {
        text_spans.push(Span::styled(
            "(leer)",
            Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::ITALIC),
        ));
    } else {
        text_spans.push(Span::styled(
            text.to_string(),
            Style::default().fg(Color::White),
        ));
    }

    let marker = if active { "▸ " } else { "  " };
    let mut spans = vec![
        Span::styled(
            marker,
            Style::default().fg(if active {
                Color::Yellow
            } else {
                Color::DarkGray
            }),
        ),
        Span::styled(format!("{} ", label), label_style),
    ];
    spans.extend(text_spans);
    Paragraph::new(Line::from(spans))
}

fn render_remote_section(frame: &mut Frame, area: Rect, state: &PermissionModeState) {
    let is_active = state.section == DialogSection::RemoteControl;
    let chunks = Layout::vertical([Constraint::Length(1), Constraint::Length(1)]).split(area);

    let header = Paragraph::new(Line::from(vec![
        Span::styled("[ Options ]", section_header_style(is_active)),
        Span::styled(
            if is_active { "  Space togglen" } else { "" },
            Style::default().fg(Color::DarkGray),
        ),
    ]));
    frame.render_widget(header, chunks[0]);

    let checkbox = if state.remote_control { "[x]" } else { "[ ]" };
    let marker = if is_active { "▸ " } else { "  " };
    let line = Paragraph::new(Line::from(vec![
        Span::styled(
            marker,
            Style::default().fg(if is_active {
                Color::Yellow
            } else {
                Color::DarkGray
            }),
        ),
        Span::styled(
            format!("{} Remote Control ", checkbox),
            Style::default()
                .fg(if is_active {
                    Color::Yellow
                } else {
                    Color::White
                })
                .add_modifier(if is_active {
                    Modifier::BOLD
                } else {
                    Modifier::empty()
                }),
        ),
        Span::styled("(--remote-control)", Style::default().fg(Color::DarkGray)),
    ]));
    frame.render_widget(line, chunks[1]);
}

fn render_footer(frame: &mut Frame, area: Rect) {
    let footer = Line::from(vec![
        Span::styled(" Enter ", Style::default().bg(Color::Cyan).fg(Color::Black)),
        Span::raw(" Bestätigen  "),
        Span::styled(
            " Tab ",
            Style::default().bg(Color::DarkGray).fg(Color::White),
        ),
        Span::raw(" Sektion  "),
        Span::styled(
            " Esc ",
            Style::default().bg(Color::DarkGray).fg(Color::White),
        ),
        Span::raw(" Standard  "),
        Span::styled(
            " Space ",
            Style::default().bg(Color::DarkGray).fg(Color::White),
        ),
        Span::raw(" Toggle"),
    ]);
    frame.render_widget(Paragraph::new(footer), area);
}

fn section_header_style(active: bool) -> Style {
    if active {
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    }
}
