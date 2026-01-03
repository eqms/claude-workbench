use ratatui::{
    widgets::{Block, List, ListItem, ListState},
    style::{Style, Modifier, Color},
    Frame,
    prelude::Rect,
};
use std::path::PathBuf;
use std::fs;
// use anyhow::Result;

#[derive(Debug, Clone)]
pub struct FileEntry {
    pub path: PathBuf,
    pub name: String,
    pub is_dir: bool,
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

        if let Ok(entries) = fs::read_dir(&self.current_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                let is_dir = path.is_dir();
                let name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
                
                // Simple ignore filter (should be configurable)
                if name.starts_with('.') && name != ".." {
                    continue;
                }
                
                self.entries.push(FileEntry {
                    path,
                    name,
                    is_dir,
                });
            }
        }
        
        // Sort: Directories first, then files
        self.entries.sort_by(|a, b| {
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
}

pub fn render(f: &mut Frame, area: Rect, state: &mut FileBrowserState, is_focused: bool) {
    let items: Vec<ListItem> = state.entries.iter().map(|entry| {
        let icon = if entry.is_dir { "📁 " } else { "📄 " };
        let content = format!("{}{}", icon, entry.name);
        ListItem::new(content)
    }).collect();

    let border_style = if is_focused {
        Style::default().fg(Color::Green)
    } else {
        Style::default()
    };
    
    let title = format!(" Files: {} ", state.current_dir.display());
    let block = Block::bordered()
        .title(title)
        .border_style(border_style);

    let list = List::new(items)
        .block(block)
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED));

    f.render_stateful_widget(list, area, &mut state.list_state);
}
