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
use std::collections::{HashMap, HashSet};
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
    pub depth: usize,
    pub expanded: bool,
}

#[derive(Debug, Clone)]
pub struct FileBrowserState {
    pub current_dir: PathBuf,
    pub root_dir: PathBuf,
    pub entries: Vec<FileEntry>,
    pub list_state: ListState,
    pub repo_root: Option<PathBuf>,
    pub git_info: Option<GitRepoInfo>,
    git_statuses: HashMap<PathBuf, GitFileStatus>,
    pub show_hidden: bool,
    pub expanded_dirs: HashSet<PathBuf>,
    /// Previous directory for F7 toggle (back from ~/.claude)
    pub previous_dir: Option<PathBuf>,
}

impl FileBrowserState {
    pub fn new(show_hidden: bool) -> Self {
        let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let root_dir = current_dir.clone();
        let repo_root = git::find_repo_root(&current_dir);
        let mut s = Self {
            current_dir,
            root_dir: root_dir.clone(),
            entries: Vec::new(),
            list_state: ListState::default(),
            repo_root,
            git_info: None,
            git_statuses: HashMap::new(),
            show_hidden,
            expanded_dirs: HashSet::new(),
            previous_dir: None,
        };
        // Start with root expanded
        s.expanded_dirs.insert(root_dir);
        s.load_tree();
        s
    }

    /// Build the flat entry list from the tree structure
    pub fn load_tree(&mut self) {
        self.entries.clear();
        self.list_state.select(None);

        // Update repo root and git status from root_dir
        self.repo_root = git::find_repo_root(&self.root_dir);
        let (statuses, git_info) = git::get_status_for_directory(&self.root_dir);
        self.git_statuses = statuses;
        self.git_info = git_info;

        // Add ".." entry if root has a parent directory
        if self.root_dir.parent().is_some() {
            self.entries.push(FileEntry {
                path: self.root_dir.clone(), // Special marker - path points to current root
                name: "..".to_string(),
                is_dir: true,
                size: 0,
                modified: None,
                git_status: GitFileStatus::Clean,
                depth: 0,
                expanded: false,
            });
        }

        // Build tree recursively from root
        self.build_tree_recursive(&self.root_dir.clone(), 0);

        if !self.entries.is_empty() {
            self.list_state.select(Some(0));
        }
    }

    /// Recursively build the flat list from directory tree
    fn build_tree_recursive(&mut self, dir: &PathBuf, depth: usize) {
        let is_expanded = self.expanded_dirs.contains(dir);

        if let Ok(read_entries) = fs::read_dir(dir) {
            let mut children: Vec<(PathBuf, String, bool)> = Vec::new();

            for entry in read_entries.flatten() {
                let path = entry.path();
                let is_dir = path.is_dir();
                let name = path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();

                // Filter hidden files
                if !self.show_hidden && name.starts_with('.') {
                    continue;
                }

                children.push((path, name, is_dir));
            }

            // Sort: directories first, then alphabetically
            children.sort_by(|a, b| {
                if a.2 == b.2 {
                    a.1.to_lowercase().cmp(&b.1.to_lowercase())
                } else if a.2 {
                    std::cmp::Ordering::Less
                } else {
                    std::cmp::Ordering::Greater
                }
            });

            for (path, name, is_dir) in children {
                let metadata = fs::metadata(&path).ok();
                let size = metadata.as_ref().map(|m| m.len()).unwrap_or(0);
                let modified = metadata.and_then(|m| m.modified().ok());

                let git_status = if is_dir {
                    git::aggregate_directory_status(&path, &self.git_statuses)
                } else {
                    self.git_statuses
                        .get(&path)
                        .copied()
                        .unwrap_or(GitFileStatus::Clean)
                };

                let expanded = is_dir && self.expanded_dirs.contains(&path);

                self.entries.push(FileEntry {
                    path: path.clone(),
                    name,
                    is_dir,
                    size,
                    modified,
                    git_status,
                    depth,
                    expanded,
                });

                // If directory is expanded, recurse into it
                if is_dir && is_expanded && expanded {
                    self.build_tree_recursive(&path, depth + 1);
                }
            }
        }
    }

    pub fn up(&mut self) {
        let i = match self.list_state.selected() {
            Some(i) => {
                if i == 0 {
                    0
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
                if i >= self.entries.len().saturating_sub(1) {
                    self.entries.len().saturating_sub(1)
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.list_state.select(Some(i));
    }

    /// Toggle expand/collapse for directories, return file path for files
    pub fn enter_selected(&mut self) -> Option<PathBuf> {
        if let Some(i) = self.list_state.selected() {
            if let Some(entry) = self.entries.get(i).cloned() {
                // Special handling for ".." entry - navigate to parent
                if entry.name == ".." {
                    self.navigate_root_up();
                    return None;
                }

                if entry.is_dir {
                    self.toggle_expand(&entry.path);
                    // Update current_dir to the selected directory
                    self.current_dir = entry.path;
                    return None;
                } else {
                    return Some(entry.path);
                }
            }
        }
        None
    }

    /// Toggle a directory's expanded state and rebuild the tree
    fn toggle_expand(&mut self, path: &PathBuf) {
        if self.expanded_dirs.contains(path) {
            self.expanded_dirs.remove(path);
        } else {
            self.expanded_dirs.insert(path.clone());
        }
        self.rebuild_tree();
    }

    /// Collapse current directory or navigate to parent
    ///
    /// Behavior:
    /// 1. If selected is ".." entry → navigate root_dir up one level
    /// 2. If selected is an expanded directory → collapse it
    /// 3. If parent is above root_dir → navigate root_dir up one level
    /// 4. If parent exists in entries → select it
    /// 5. Fallback → navigate root_dir up one level
    pub fn go_parent(&mut self) {
        let Some(i) = self.list_state.selected() else {
            return;
        };
        let Some(entry) = self.entries.get(i).cloned() else {
            return;
        };

        // Case 1: If ".." entry is selected, navigate up
        if entry.name == ".." {
            self.navigate_root_up();
            return;
        }

        // Case 2: Collapse expanded directory
        if entry.is_dir && self.expanded_dirs.contains(&entry.path) {
            self.expanded_dirs.remove(&entry.path);
            self.rebuild_tree();
            return;
        }

        // Get parent path
        let Some(parent_path) = entry.path.parent().map(|p| p.to_path_buf()) else {
            return;
        };

        // Case 3: If we're at or below root_dir and parent would be root_dir,
        // navigate the entire view up one level
        if parent_path == self.root_dir || entry.path == self.root_dir {
            self.navigate_root_up();
            return;
        }

        // Case 4: Find parent in entries list and select it
        if let Some(parent_idx) = self
            .entries
            .iter()
            .position(|e| e.path == parent_path && e.is_dir)
        {
            self.list_state.select(Some(parent_idx));
            self.current_dir = parent_path;
            return;
        }

        // Case 5: Parent not in list, navigate root up
        self.navigate_root_up();
    }

    /// Navigate the root directory up one level
    fn navigate_root_up(&mut self) {
        if let Some(parent) = self.root_dir.parent() {
            let old_root = self.root_dir.clone();
            self.root_dir = parent.to_path_buf();
            self.current_dir = self.root_dir.clone();
            self.expanded_dirs.clear();
            self.expanded_dirs.insert(self.root_dir.clone());
            // Keep old root expanded so we can see where we came from
            self.expanded_dirs.insert(old_root);
            self.load_tree();
        }
    }

    /// Rebuild the flat list while preserving selection
    fn rebuild_tree(&mut self) {
        let selected_path = self
            .list_state
            .selected()
            .and_then(|i| self.entries.get(i))
            .map(|e| (e.path.clone(), e.name.clone()));

        self.entries.clear();
        self.list_state.select(None);

        // Add ".." entry if root has a parent directory
        if self.root_dir.parent().is_some() {
            self.entries.push(FileEntry {
                path: self.root_dir.clone(),
                name: "..".to_string(),
                is_dir: true,
                size: 0,
                modified: None,
                git_status: GitFileStatus::Clean,
                depth: 0,
                expanded: false,
            });
        }

        self.build_tree_recursive(&self.root_dir.clone(), 0);

        // Restore selection by path and name (to distinguish ".." from regular entries)
        if let Some((path, name)) = selected_path {
            if let Some(idx) = self
                .entries
                .iter()
                .position(|e| e.path == path && e.name == name)
            {
                self.list_state.select(Some(idx));
            } else if !self.entries.is_empty() {
                self.list_state.select(Some(0));
            }
        } else if !self.entries.is_empty() {
            self.list_state.select(Some(0));
        }
    }

    pub fn selected_file(&self) -> Option<PathBuf> {
        self.list_state
            .selected()
            .and_then(|i| self.entries.get(i).map(|e| e.path.clone()))
    }

    pub fn refresh(&mut self) {
        let selected_path = self
            .list_state
            .selected()
            .and_then(|i| self.entries.get(i))
            .map(|e| e.path.clone());

        // Preserve expanded_dirs across refresh
        self.load_tree();

        // Restore selection by path
        if let Some(path) = selected_path {
            if let Some(idx) = self.entries.iter().position(|e| e.path == path) {
                self.list_state.select(Some(idx));
            }
        }
    }

    /// Legacy compatibility: load_directory calls load_tree
    pub fn load_directory(&mut self) {
        // When current_dir changes externally, reset tree to that dir
        self.root_dir = self.current_dir.clone();
        self.expanded_dirs.clear();
        self.expanded_dirs.insert(self.root_dir.clone());
        self.load_tree();
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
            // Tree indentation
            let indent = "    ".repeat(entry.depth);

            // Tree icon: special icon for "..", expanded/collapsed for dirs, file icon for files
            let tree_icon = if entry.name == ".." {
                "↑ 📁 " // Special icon for parent directory navigation
            } else if entry.is_dir {
                if entry.expanded {
                    "▼ 📁 "
                } else {
                    "▶ 📁 "
                }
            } else {
                "  📄 "
            };

            // Get git status symbol and style
            let status_symbol = entry.git_status.symbol();
            let status_style = style_for_git_status(entry.git_status);

            // Build the line with indent, tree icon, and name
            let line = Line::from(vec![
                Span::styled(status_symbol, status_style),
                Span::raw(" "),
                Span::raw(indent),
                Span::styled(format!("{}{}", tree_icon, entry.name), status_style),
            ]);

            ListItem::new(line)
        })
        .collect();

    let (border_style, border_type) = if is_focused {
        (Style::default().fg(Color::Green), BorderType::Double)
    } else {
        (Style::default(), BorderType::Rounded)
    };

    let title = format!(" {} ", state.root_dir.display());
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
            if entry.is_dir {
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
