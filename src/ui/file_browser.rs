use ratatui::{
    widgets::{Block, List, ListItem, ListState},
    style::{Style, Modifier, Color},
    Frame,
    prelude::Rect,
    text::{Line, Span},
};
use std::path::PathBuf;
use std::fs;
use std::time::SystemTime;

#[derive(Debug, Clone)]
pub struct FileEntry {
    pub path: PathBuf,
    pub name: String,
    pub is_dir: bool,
    pub size: u64,
    pub modified: Option<SystemTime>,
}

#[derive(Debug, Clone)]
pub struct FileBrowserState {
    pub current_dir: PathBuf,
    pub entries: Vec<FileEntry>,
    pub list_state: ListState,
}

impl FileBrowserState {
    pub fn new() -> Self {
        let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let mut s = Self {
            current_dir,
            entries: Vec::new(),
            list_state: ListState::default(),
        };
        s.load_directory();
        s
    }

    pub fn load_directory(&mut self) {
        self.entries.clear();
        self.list_state.select(None);

        // Add ".." entry for parent directory (if not root)
        if self.current_dir.parent().is_some() {
            self.entries.push(FileEntry {
                path: self.current_dir.parent().unwrap().to_path_buf(),
                name: "..".to_string(),
                is_dir: true,
                size: 0,
                modified: None,
            });
        }

        if let Ok(entries) = fs::read_dir(&self.current_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                let is_dir = path.is_dir();
                let name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
                
                // Simple ignore filter (should be configurable)
                if name.starts_with('.') {
                    continue;
                }
                
                let metadata = fs::metadata(&path).ok();
                let size = metadata.as_ref().map(|m| m.len()).unwrap_or(0);
                let modified = metadata.and_then(|m| m.modified().ok());
                
                self.entries.push(FileEntry {
                    path,
                    name,
                    is_dir,
                    size,
                    modified,
                });
            }
        }
        
        // Sort: ".." first, then Directories, then files
        self.entries.sort_by(|a, b| {
            if a.name == ".." { return std::cmp::Ordering::Less; }
            if b.name == ".." { return std::cmp::Ordering::Greater; }
            if a.is_dir == b.is_dir {
                a.name.cmp(&b.name)
            } else if a.is_dir {
                std::cmp::Ordering::Less
            } else {
                std::cmp::Ordering::Greater
            }
        });

        if !self.entries.is_empty() {
             self.list_state.select(Some(0));
        }
    }

    pub fn up(&mut self) {
        let i = match self.list_state.selected() {
            Some(i) => {
                if i == 0 {
                    self.entries.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.list_state.select(Some(i));
    }

    pub fn down(&mut self) {
        let i = match self.list_state.selected() {
            Some(i) => {
                if i >= self.entries.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.list_state.select(Some(i));
    }
    
    pub fn enter_selected(&mut self) -> Option<PathBuf> {
        if let Some(i) = self.list_state.selected() {
            if let Some(entry) = self.entries.get(i) {
                if entry.is_dir {
                    self.current_dir = entry.path.clone();
                    self.load_directory();
                    return None;
                } else {
                    return Some(entry.path.clone());
                }
            }
        }
        None
    }

    pub fn go_parent(&mut self) {
        if let Some(parent) = self.current_dir.parent() {
            self.current_dir = parent.to_path_buf();
            self.load_directory();
        }
    }
    
    pub fn selected_file(&self) -> Option<PathBuf> {
        self.list_state.selected().and_then(|i| self.entries.get(i).map(|e| e.path.clone()))
    }
    
    pub fn refresh(&mut self) {
        let selected = self.list_state.selected();
        self.load_directory();
        // Try to restore selection
        if let Some(idx) = selected {
            if idx < self.entries.len() {
                self.list_state.select(Some(idx));
            }
        }
    }
}

pub fn render(f: &mut Frame, area: Rect, state: &mut FileBrowserState, is_focused: bool) {
    use ratatui::layout::{Layout, Direction, Constraint};
    use ratatui::widgets::Paragraph;
    
    // Split area: list + info bar
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(area);
    
    let list_area = chunks[0];
    let info_area = chunks[1];

    let items: Vec<ListItem> = state.entries.iter().map(|entry| {
        let icon = if entry.name == ".." { "↩️ " } else if entry.is_dir { "📁 " } else { "📄 " };
        let content = format!("{}{}", icon, entry.name);
        ListItem::new(content)
    }).collect();

    let border_style = if is_focused {
        Style::default().fg(Color::Green)
    } else {
        Style::default()
    };
    
    let title = format!(" {} ", state.current_dir.display());
    let block = Block::bordered()
        .title(title)
        .border_style(border_style);

    let list = List::new(items)
        .block(block)
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED));

    f.render_stateful_widget(list, list_area, &mut state.list_state);
    
    // File info bar
    let info_text = if let Some(idx) = state.list_state.selected() {
        if let Some(entry) = state.entries.get(idx) {
            if entry.name == ".." {
                " ↩️ Parent Directory".to_string()
            } else if entry.is_dir {
                format!(" 📁 Directory")
            } else {
                let size_kb = entry.size as f64 / 1024.0;
                let date_str = entry.modified
                    .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
                    .map(|d| {
                        let secs = d.as_secs();
                        // Date formatting (dd.mm.yyyy hh:mm:ss)
                        let days = secs / 86400;
                        let years = 1970 + days / 365;
                        let remaining_days = days % 365;
                        let months = remaining_days / 30 + 1;
                        let day = remaining_days % 30 + 1;
                        // Time
                        let day_secs = secs % 86400;
                        let hours = day_secs / 3600;
                        let minutes = (day_secs % 3600) / 60;
                        let seconds = day_secs % 60;
                        format!("{:02}.{:02}.{} {:02}:{:02}:{:02}", day, months, years, hours, minutes, seconds)
                    })
                    .unwrap_or_else(|| "---".to_string());
                format!(" 📄 {:.1} KB | {}", size_kb, date_str)
            }
        } else {
            String::new()
        }
    } else {
        String::new()
    };
    
    let info = Paragraph::new(info_text)
        .style(Style::default().fg(Color::DarkGray));
    f.render_widget(info, info_area);
}
