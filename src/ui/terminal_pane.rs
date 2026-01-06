use ratatui::{widgets::{Block, Paragraph, Widget, Wrap}, Frame, buffer::Buffer};
use ratatui::prelude::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use crate::app::App;
use crate::types::PaneId;

pub fn render(f: &mut Frame, area: Rect, pane_id: PaneId, app: &App) {
    let title = match pane_id {
        PaneId::Claude => " Claude Code ",
        PaneId::LazyGit => " LazyGit ",
        PaneId::Terminal => " Terminal ",
        _ => " Unknown ",
    };

    let is_focused = app.active_pane == pane_id;

    // Check if this pane is in selection mode (keyboard selection or mouse selection)
    let selection_active = app.terminal_selection.active
        && app.terminal_selection.source_pane == Some(pane_id);
    let mouse_selection_active = app.mouse_selection.is_selecting_in(pane_id);

    // Check if this pane is a drop target (dragging over it)
    let is_drop_target = app.drag_state.dragging && {
        let x = app.drag_state.current_x;
        let y = app.drag_state.current_y;
        x >= area.x && x < area.x + area.width && y >= area.y && y < area.y + area.height
    };

    // Check for Claude error - show red border if error
    let has_error = pane_id == PaneId::Claude && app.claude_error.is_some();
    let border_style = if is_drop_target {
        Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
    } else if selection_active || mouse_selection_active {
        Style::default().fg(Color::Yellow)
    } else if has_error {
        Style::default().fg(Color::Red)
    } else if is_focused {
        Style::default().fg(Color::Green)
    } else {
        Style::default()
    };

    use ratatui::widgets::{Scrollbar, ScrollbarOrientation, ScrollbarState};

    let block = Block::bordered().title(title).border_style(border_style);
    let inner_area = block.inner(area);

    f.render_widget(block, area);

    // Show error message for Claude if PTY failed
    if pane_id == PaneId::Claude {
        if let Some(error) = &app.claude_error {
            let error_lines: Vec<Line> = vec![
                Line::from(vec![
                    Span::styled("⚠ ", Style::default().fg(Color::Yellow)),
                    Span::styled("Claude CLI Error", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
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
        let parser = pty.parser.lock().unwrap();
        let screen = parser.screen();

        // Get selection range if this pane is the source (keyboard or mouse selection)
        let selection_range = if selection_active {
            app.terminal_selection.line_range()
        } else if mouse_selection_active {
            app.mouse_selection.line_range()
        } else {
            None
        };

        TerminalWidget::new(screen)
            .with_selection(selection_range)
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
            area.inner(ratatui::layout::Margin { vertical: 1, horizontal: 0 }), // Inside border
            &mut scrollbar_state,
        );
    }
}

struct TerminalWidget<'a> {
    screen: &'a vt100::Screen,
    selection_range: Option<(usize, usize)>,
}

impl<'a> TerminalWidget<'a> {
    fn new(screen: &'a vt100::Screen) -> Self {
        Self { screen, selection_range: None }
    }

    fn with_selection(mut self, range: Option<(usize, usize)>) -> Self {
        self.selection_range = range;
        self
    }
}

impl Widget for TerminalWidget<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let (rows, cols) = self.screen.size();

        for r in 0..area.height {
            if r >= rows { break; }

            // Check if this row is selected
            let row_selected = self.selection_range.map_or(false, |(start, end)| {
                let row_idx = r as usize;
                row_idx >= start && row_idx <= end
            });

            for c in 0..area.width {
                if c >= cols { break; }

                let cell = self.screen.cell(r, c);
                if let Some(cell) = cell {
                    let char_val = cell.contents().chars().next().unwrap_or(' ');

                    let x = area.x + c;
                    let y = area.y + r;
                    if x < buf.area.width && y < buf.area.height {
                         if let Some(c) = buf.cell_mut((x, y)) {
                             c.set_char(char_val);

                             // Map Colors
                             let fg = map_color(cell.fgcolor());
                             let bg = map_color(cell.bgcolor());

                             let mut style = Style::default();
                             if let Some(f) = fg { style = style.fg(f); }
                             if let Some(b) = bg { style = style.bg(b); }

                             // Apply selection highlighting (DarkGray background)
                             if row_selected {
                                 style = style.bg(Color::DarkGray);
                             }

                             // Attributes (Bold, Italic, etc. - MVP skip or add basic)
                             if cell.bold() { style = style.add_modifier(ratatui::style::Modifier::BOLD); }
                             if cell.italic() { style = style.add_modifier(ratatui::style::Modifier::ITALIC); }
                             if cell.inverse() { style = style.add_modifier(ratatui::style::Modifier::REVERSED); }
                             if cell.underline() { style = style.add_modifier(ratatui::style::Modifier::UNDERLINED); }

                             c.set_style(style);
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
