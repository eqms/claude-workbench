use ratatui::{widgets::{Block, Widget}, Frame, buffer::Buffer};
use ratatui::prelude::Rect;
use ratatui::style::{Color, Style};
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
    let border_style = if is_focused {
        Style::default().fg(Color::Green)
    } else {
        Style::default()
    };

    use ratatui::widgets::{Scrollbar, ScrollbarOrientation, ScrollbarState};

    let block = Block::bordered().title(title).border_style(border_style);
    let inner_area = block.inner(area);
    
    f.render_widget(block, area);

    if let Some(pty) = app.terminals.get(&pane_id) {
        let parser = pty.parser.lock().unwrap();
        let screen = parser.screen();
        
        TerminalWidget::new(screen).render(inner_area, f.buffer_mut());
        
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
}

impl<'a> TerminalWidget<'a> {
    fn new(screen: &'a vt100::Screen) -> Self {
        Self { screen }
    }
}

impl Widget for TerminalWidget<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let (rows, cols) = self.screen.size();
        
        for r in 0..area.height {
            if r >= rows { break; }
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
