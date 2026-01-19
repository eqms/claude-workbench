use crate::git;
use crate::types::{GitFileStatus, GitRepoInfo};
use ratatui::{
    prelude::Rect,
    style::{Color, Modifier, Style},
    widgets::{
        Block, BorderType, List, ListItem, ListState, Scrollbar, ScrollbarOrientation,
        ScrollbarState,
    },
    Frame,
};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::time::SystemTime;

/// Format file modification date for display using local timezone
fn format_file_date(utc_secs: u64) -> String {
    let time_t = utc_secs as libc::time_t;
    let mut tm: libc::tm = unsafe { std::mem::zeroed() };

    // Platform-specific timezone conversion
    #[cfg(unix)]
    unsafe {
        libc::localtime_r(&time_t, &mut tm);
    }

    #[cfg(windows)]
    unsafe {
        // Windows uses localtime_s with swapped argument order
        libc::localtime_s(&mut tm, &time_t);
    }

    // tm_year is years since 1900, tm_mon is 0-11
    let year = tm.tm_year + 1900;
    let month = tm.tm_mon + 1;
    let day = tm.tm_mday;
    let hours = tm.tm_hour;
    let minutes = tm.tm_min;

    format!(
        "{:02}.{:02}.{} {:02}:{:02}",
        day, month, year, hours, minutes
    )
}

#[derive(Debug, Clone)]
pub struct FileEntry {
    pub path: PathBuf,
    pub name: String,
    pub is_dir: bool,
    pub size: u64,
    pub modified: Option<SystemTime>,
    pub git_status: GitFileStatus,
}

#[derive(Debug, Clone)]
pub struct FileBrowserState {
    pub current_dir: PathBuf,
    pub entries: Vec<FileEntry>,
    pub list_state: ListState,
    pub repo_root: Option<PathBuf>,
    pub git_info: Option<GitRepoInfo>,
    git_statuses: HashMap<PathBuf, GitFileStatus>,
    pub show_hidden: bool,
}

impl FileBrowserState {
    pub fn new(show_hidden: bool) -> Self {
        let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let repo_root = git::find_repo_root(&current_dir);
        let mut s = Self {
            current_dir,
            entries: Vec::new(),
            list_state: ListState::default(),
            repo_root,
            git_info: None,
            git_statuses: HashMap::new(),
            show_hidden,
        };
        s.load_directory();
        s
    }

    pub fn load_directory(&mut self) {
        self.entries.clear();
        self.list_state.select(None);

        // Update repo root for new directory
        self.repo_root = git::find_repo_root(&self.current_dir);

        // Fetch git status for the current directory
        let (statuses, git_info) = git::get_status_for_directory(&self.current_dir);
        self.git_statuses = statuses;
        self.git_info = git_info;

        // Add ".." entry for parent directory (if not root)
        if self.current_dir.parent().is_some() {
            self.entries.push(FileEntry {
                path: self.current_dir.parent().unwrap().to_path_buf(),
                name: "..".to_string(),
                is_dir: true,
                size: 0,
                modified: None,
                git_status: GitFileStatus::Clean,
            });
        }

        if let Ok(entries) = fs::read_dir(&self.current_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                let is_dir = path.is_dir();
                let name = path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();

                // Filter hidden files based on config
                if !self.show_hidden && name.starts_with('.') {
                    continue;
                }

                let metadata = fs::metadata(&path).ok();
                let size = metadata.as_ref().map(|m| m.len()).unwrap_or(0);
                let modified = metadata.and_then(|m| m.modified().ok());

                // Determine git status
                let git_status = if is_dir {
                    // For directories, aggregate status from contained files
                    git::aggregate_directory_status(&path, &self.git_statuses)
                } else {
                    // For files, look up status directly
                    self.git_statuses
                        .get(&path)
                        .copied()
                        .unwrap_or(GitFileStatus::Clean)
                };

                self.entries.push(FileEntry {
                    path,
                    name,
                    is_dir,
                    size,
                    modified,
                    git_status,
                });
            }
        }

        // Sort: ".." first, then Directories, then files
        self.entries.sort_by(|a, b| {
            if a.name == ".." {
                return std::cmp::Ordering::Less;
            }
            if b.name == ".." {
                return std::cmp::Ordering::Greater;
            }
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
        self.list_state
            .selected()
            .and_then(|i| self.entries.get(i).map(|e| e.path.clone()))
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

/// Get style for git status
fn style_for_git_status(status: GitFileStatus) -> Style {
    match status {
        GitFileStatus::Untracked => Style::default().fg(Color::Yellow),
        GitFileStatus::Modified => Style::default().fg(Color::Rgb(255, 165, 0)), // Orange
        GitFileStatus::Staged => Style::default().fg(Color::Green),
        GitFileStatus::Ignored => Style::default()
            .fg(Color::DarkGray)
            .add_modifier(Modifier::DIM),
        GitFileStatus::Conflict => Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        GitFileStatus::Clean | GitFileStatus::Unknown => Style::default(),
    }
}

pub fn render(f: &mut Frame, area: Rect, state: &mut FileBrowserState, is_focused: bool) {
    use ratatui::layout::{Constraint, Direction, Layout};
    use ratatui::text::{Line, Span};
    use ratatui::widgets::Paragraph;

    // Split area: list + info bar
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(area);

    let list_area = chunks[0];
    let info_area = chunks[1];

    let items: Vec<ListItem> = state
        .entries
        .iter()
        .map(|entry| {
            let icon = if entry.name == ".." {
                "↩️ "
            } else if entry.is_dir {
                "📁 "
            } else {
                "📄 "
            };

            // Get git status symbol and style
            let status_symbol = entry.git_status.symbol();
            let status_style = style_for_git_status(entry.git_status);

            // Build the line with colored status symbol and name
            let line = Line::from(vec![
                Span::styled(status_symbol, status_style),
                Span::raw(" "),
                Span::styled(format!("{}{}", icon, entry.name), status_style),
            ]);

            ListItem::new(line)
        })
        .collect();

    let (border_style, border_type) = if is_focused {
        (Style::default().fg(Color::Green), BorderType::Double)
    } else {
        (Style::default(), BorderType::Rounded)
    };

    let title = format!(" {} ", state.current_dir.display());
    let block = Block::bordered()
        .title(title)
        .border_style(border_style)
        .border_type(border_type);

    let list = List::new(items)
        .block(block)
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED));

    f.render_stateful_widget(list, list_area, &mut state.list_state);

    // Scrollbar
    let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
        .begin_symbol(Some("▲"))
        .end_symbol(Some("▼"));

    let mut scrollbar_state =
        ScrollbarState::new(state.entries.len()).position(state.list_state.selected().unwrap_or(0));

    // Render scrollbar in the inner area (inside the border)
    let scrollbar_area = Rect {
        x: list_area.x,
        y: list_area.y,
        width: list_area.width,
        height: list_area.height.saturating_sub(1), // Account for border
    };
    f.render_stateful_widget(scrollbar, scrollbar_area, &mut scrollbar_state);

    // File info bar with git branch info
    let git_info_str = if let Some(ref git_info) = state.git_info {
        let mut parts = vec![format!("🌿 {}", git_info.branch)];
        if git_info.modified_count > 0 {
            parts.push(format!("M:{}", git_info.modified_count));
        }
        if git_info.untracked_count > 0 {
            parts.push(format!("?:{}", git_info.untracked_count));
        }
        if git_info.staged_count > 0 {
            parts.push(format!("+:{}", git_info.staged_count));
        }
        parts.join(" ")
    } else {
        String::new()
    };

    let file_info_text = if let Some(idx) = state.list_state.selected() {
        if let Some(entry) = state.entries.get(idx) {
            if entry.name == ".." {
                " ↩️ Parent".to_string()
            } else if entry.is_dir {
                " 📁 Dir".to_string()
            } else {
                let size_kb = entry.size as f64 / 1024.0;
                let date_str = entry
                    .modified
                    .and_then(|m| m.duration_since(SystemTime::UNIX_EPOCH).ok())
                    .map(|d| format_file_date(d.as_secs()))
                    .unwrap_or_default();
                if date_str.is_empty() {
                    format!(" 📄 {:.1}K", size_kb)
                } else {
                    format!(" 📄 {:.1}K │ {}", size_kb, date_str)
                }
            }
        } else {
            String::new()
        }
    } else {
        String::new()
    };

    // Combine file info and git info
    let info_text = if git_info_str.is_empty() {
        file_info_text
    } else {
        format!("{} │ {}", file_info_text, git_info_str)
    };

    let info = Paragraph::new(info_text).style(Style::default().fg(Color::DarkGray));
    f.render_widget(info, info_area);
}
