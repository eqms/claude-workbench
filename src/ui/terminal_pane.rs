use crate::app::App;
use crate::types::PaneId;
use ratatui::prelude::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::{
    buffer::Buffer,
    widgets::{Block, BorderType, Paragraph, Widget, Wrap},
    Frame,
};

pub fn render(f: &mut Frame, area: Rect, pane_id: PaneId, app: &App) {
    let title = match pane_id {
        PaneId::Claude => " Claude Code ",
        PaneId::LazyGit => " LazyGit ",
        PaneId::Terminal => " Terminal ",
        _ => " Unknown ",
    };

    let is_focused = app.active_pane == pane_id;

    // Check if this pane is in selection mode (keyboard selection or mouse selection)
    let selection_active =
        app.terminal_selection.active && app.terminal_selection.source_pane == Some(pane_id);
    let mouse_selection_active = app.mouse_selection.is_selecting_in(pane_id);

    // Check if this pane is a drop target (dragging over it)
    let is_drop_target = app.drag_state.dragging && {
        let x = app.drag_state.current_x;
        let y = app.drag_state.current_y;
        x >= area.x && x < area.x + area.width && y >= area.y && y < area.y + area.height
    };

    // Check for Claude error - show red border if error
    let has_error = pane_id == PaneId::Claude && app.claude_error.is_some();
    let (border_style, border_type) = if is_drop_target {
        (
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
            BorderType::Double,
        )
    } else if selection_active || mouse_selection_active {
        (Style::default().fg(Color::Yellow), BorderType::Double)
    } else if has_error {
        (Style::default().fg(Color::Red), BorderType::Rounded)
    } else if is_focused {
        (Style::default().fg(Color::Green), BorderType::Double)
    } else {
        (Style::default(), BorderType::Rounded)
    };

    use ratatui::widgets::{Scrollbar, ScrollbarOrientation, ScrollbarState};

    let block = Block::bordered()
        .title(title)
        .border_style(border_style)
        .border_type(border_type);
    let inner_area = block.inner(area);

    f.render_widget(block, area);

    // Show error message for Claude if PTY failed
    if pane_id == PaneId::Claude {
        if let Some(error) = &app.claude_error {
            let error_lines: Vec<Line> = vec![
                Line::from(vec![
                    Span::styled("⚠ ", Style::default().fg(Color::Yellow)),
                    Span::styled(
                        "Claude CLI Error",
                        Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                    ),
                ]),
                Line::from(""),
            ]
            .into_iter()
            .chain(error.lines().map(|l| Line::from(l.to_string())))
            .collect();

            let error_paragraph = Paragraph::new(error_lines)
                .style(Style::default().fg(Color::White))
                .wrap(Wrap { trim: false });
            f.render_widget(error_paragraph, inner_area);
            return;
        }
    }

    if let Some(pty) = app.terminals.get(&pane_id) {
        // Check if PTY process has exited
        if pty.has_exited() && !app.config.pty.auto_restart {
            // Show "exited" message with restart hint
            let exit_lines: Vec<Line> = vec![
                Line::from(""),
                Line::from(vec![
                    Span::styled("⚠ ", Style::default().fg(Color::Yellow)),
                    Span::styled("Process exited", Style::default().fg(Color::Gray)),
                ]),
                Line::from(""),
                Line::from(vec![
                    Span::styled("Press ", Style::default().fg(Color::DarkGray)),
                    Span::styled(
                        "Enter",
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(" to restart", Style::default().fg(Color::DarkGray)),
                ]),
            ];
            let exit_paragraph = Paragraph::new(exit_lines)
                .style(Style::default())
                .wrap(Wrap { trim: false });
            f.render_widget(exit_paragraph, inner_area);
            return;
        }

        let parser = pty.parser.lock().unwrap();
        let screen = parser.screen();

        // Get selection range - keyboard selection is line-based, mouse selection is char-based
        let selection_range = if selection_active {
            app.terminal_selection.line_range()
        } else {
            None
        };

        // Get character-level mouse selection (only when mouse_selection is active)
        let char_selection = if mouse_selection_active {
            app.mouse_selection.char_range()
        } else {
            None
        };

        TerminalWidget::new(screen)
            .with_selection(selection_range)
            .with_char_selection(char_selection)
            .render(inner_area, f.buffer_mut());

        // Scrollbar
        let scrollback = screen.scrollback();
        // Assuming max history 1000 as configured.
        // Invert logic: scrollback 0 is bottom (pos 1000), scrollback 1000 is top (pos 0).
        let max_scroll = 1000;
        // Clamp scrollback to max
        let effective_scroll = scrollback.min(max_scroll);
        let scroll_pos = max_scroll - effective_scroll;

        let mut scrollbar_state = ScrollbarState::new(max_scroll).position(scroll_pos);
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("↑"))
            .end_symbol(Some("↓"));

        f.render_stateful_widget(
            scrollbar,
            area.inner(ratatui::layout::Margin {
                vertical: 1,
                horizontal: 0,
            }), // Inside border
            &mut scrollbar_state,
        );
    }
}

struct TerminalWidget<'a> {
    screen: &'a vt100::Screen,
    /// Line-based selection range (for keyboard selection mode)
    selection_range: Option<(usize, usize)>,
    /// Character-level selection: ((start_row, start_col), (end_row, end_col))
    char_selection: Option<((usize, usize), (usize, usize))>,
}

impl<'a> TerminalWidget<'a> {
    fn new(screen: &'a vt100::Screen) -> Self {
        Self {
            screen,
            selection_range: None,
            char_selection: None,
        }
    }

    fn with_selection(mut self, range: Option<(usize, usize)>) -> Self {
        self.selection_range = range;
        self
    }

    fn with_char_selection(mut self, char_sel: Option<((usize, usize), (usize, usize))>) -> Self {
        self.char_selection = char_sel;
        self
    }

    /// Check if a specific cell (row, col) is within the character-level selection
    fn is_char_selected(&self, row: usize, col: usize) -> bool {
        let Some(((start_row, start_col), (end_row, end_col))) = self.char_selection else {
            return false;
        };

        if row < start_row || row > end_row {
            return false;
        }

        if start_row == end_row {
            // Single-line selection
            col >= start_col && col <= end_col
        } else if row == start_row {
            // First line of multi-line: from start_col to end of line
            col >= start_col
        } else if row == end_row {
            // Last line of multi-line: from start to end_col
            col <= end_col
        } else {
            // Middle lines: entire line selected
            true
        }
    }
}

impl Widget for TerminalWidget<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let (rows, cols) = self.screen.size();

        for r in 0..area.height {
            if r >= rows {
                break;
            }

            let row_idx = r as usize;

            // Check if this row is selected (line-based keyboard selection)
            let row_selected = self
                .selection_range
                .is_some_and(|(start, end)| row_idx >= start && row_idx <= end);

            for c in 0..area.width {
                if c >= cols {
                    break;
                }

                let col_idx = c as usize;

                let cell = self.screen.cell(r, c);
                if let Some(cell) = cell {
                    let char_val = cell.contents().chars().next().unwrap_or(' ');

                    let x = area.x + c;
                    let y = area.y + r;
                    if x < buf.area.width && y < buf.area.height {
                        if let Some(buf_cell) = buf.cell_mut((x, y)) {
                            buf_cell.set_char(char_val);

                            // Map Colors
                            let fg = map_color(cell.fgcolor());
                            let bg = map_color(cell.bgcolor());

                            let mut style = Style::default();
                            if let Some(f) = fg {
                                style = style.fg(f);
                            }
                            if let Some(b) = bg {
                                style = style.bg(b);
                            }

                            // Apply selection highlighting
                            // Character-level selection (mouse) takes precedence, then line-based (keyboard)
                            if self.is_char_selected(row_idx, col_idx) {
                                // Mouse selection: use inverted colors for better visibility
                                style = style.bg(Color::LightYellow).fg(Color::Black);
                            } else if row_selected {
                                // Keyboard selection: DarkGray background
                                style = style.bg(Color::DarkGray);
                            }

                            // Attributes (Bold, Italic, etc.)
                            if cell.bold() {
                                style = style.add_modifier(ratatui::style::Modifier::BOLD);
                            }
                            if cell.italic() {
                                style = style.add_modifier(ratatui::style::Modifier::ITALIC);
                            }
                            if cell.inverse() {
                                style = style.add_modifier(ratatui::style::Modifier::REVERSED);
                            }
                            if cell.underline() {
                                style = style.add_modifier(ratatui::style::Modifier::UNDERLINED);
                            }

                            buf_cell.set_style(style);
                        }
                    }
                }
            }
        }

        // Draw cursor
        if !self.screen.hide_cursor() {
            let (cr, cc) = self.screen.cursor_position();
            let cx = area.x + cc;
            let cy = area.y + cr;

            if cr < area.height && cc < area.width && cx < buf.area.width && cy < buf.area.height {
                // Invert style at cursor position for visibility
                if let Some(cell) = buf.cell_mut((cx, cy)) {
                    let style = cell.style();
                    cell.set_style(style.add_modifier(ratatui::style::Modifier::REVERSED));
                }
            }
        }
    }
}

fn map_color(c: vt100::Color) -> Option<Color> {
    match c {
        vt100::Color::Default => None,
        vt100::Color::Idx(i) => Some(Color::Indexed(i)),
        vt100::Color::Rgb(r, g, b) => Some(Color::Rgb(r, g, b)),
    }
}
