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

    let block = Block::bordered().title(title).border_style(border_style);
    let inner_area = block.inner(area);
    
    f.render_widget(block, area);

    if let Some(pty) = app.terminals.get(&pane_id) {
        // RESIZE PTY if needed (hacky interior mutability or update step needed properly)
        // Since we are in render (immutable app), we interpret the parser state.
        // To resize properly, we should do it in the update loop. 
        // For now, let's assume PTY size logic is handled elsewhere or ignored.
        // (We will add a resize helper in App later).
        
        let parser = pty.parser.lock().unwrap();
        let screen = parser.screen();
        
        // Render the screen content to the buffer
        // Simple iteration over vt100::Screen
        
        // NOTE: vt100 screen size might differ from visual area if not resized.
        // We clip.
        
        TerminalWidget::new(screen).render(inner_area, f.buffer_mut());
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
