use ratatui::{
    widgets::{Block, Paragraph, Wrap},
    style::{Style, Color},
    Frame,
    prelude::Rect,
};
use std::path::PathBuf;
use std::fs;

#[derive(Debug, Clone, Default)]
pub struct PreviewState {
    pub current_file: Option<PathBuf>,
    pub content: String,
    pub scroll: u16,
}

impl PreviewState {
    pub fn new() -> Self {
        Self::default()
    }
    
    pub fn load_file(&mut self, path: PathBuf) {
        self.current_file = Some(path.clone());
        self.scroll = 0;
        
        // Simple heuristic for now: check extension or try to read as UTF-8
        if let Ok(content) = fs::read_to_string(&path) {
            self.content = content;
        } else {
            // Check if directory
            if path.is_dir() {
                 self.content = "[Directory]".to_string();
            } else {
                 self.content = "[Binary or unreadable file]".to_string();
            }
        }
    }
    
    pub fn scroll_down(&mut self) {
        self.scroll = self.scroll.saturating_add(1);
    }
    
    pub fn scroll_up(&mut self) {
        self.scroll = self.scroll.saturating_sub(1);
    }
}

pub fn render(f: &mut Frame, area: Rect, state: &PreviewState, is_focused: bool) {
    let title = if let Some(p) = &state.current_file {
        p.file_name().unwrap_or_default().to_string_lossy().to_string()
    } else {
        " Preview ".to_string()
    };
    
    let border_style = if is_focused {
        Style::default().fg(Color::Green)
    } else {
        Style::default()
    };

    let block = Block::bordered()
        .title(format!(" {} ", title))
        .border_style(border_style);
        
    let paragraph = Paragraph::new(state.content.as_str())
        .block(block)
        .wrap(Wrap { trim: false })
        .scroll((state.scroll, 0));

    f.render_widget(paragraph, area);
}
