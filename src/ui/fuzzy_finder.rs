use ratatui::{
    layout::Rect,
    style::{Color, Style, Modifier},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, List, ListItem, ListState},
    Frame,
};
use std::path::{Path, PathBuf};

pub struct FuzzyFinder {
    pub visible: bool,
    pub query: String,
    pub all_files: Vec<PathBuf>,
    pub filtered: Vec<PathBuf>,
    pub list_state: ListState,
    pub base_dir: PathBuf,
}

impl Default for FuzzyFinder {
    fn default() -> Self {
        Self {
            visible: false,
            query: String::new(),
            all_files: Vec::new(),
            filtered: Vec::new(),
            list_state: ListState::default(),
            base_dir: PathBuf::new(),
        }
    }
}

impl FuzzyFinder {
    pub fn open(&mut self, base_dir: &Path) {
        self.visible = true;
        self.query.clear();
        self.base_dir = base_dir.to_path_buf();
        self.all_files = collect_files(base_dir, 5); // Max depth 5
        self.filtered = self.all_files.clone();
        self.list_state.select(Some(0));
    }

    pub fn close(&mut self) {
        self.visible = false;
        self.query.clear();
        self.all_files.clear();
        self.filtered.clear();
    }

    pub fn push_char(&mut self, c: char) {
        self.query.push(c);
        self.update_filter();
    }

    pub fn pop_char(&mut self) {
        self.query.pop();
        self.update_filter();
    }

    fn update_filter(&mut self) {
        if self.query.is_empty() {
            self.filtered = self.all_files.clone();
        } else {
            let query_lower = self.query.to_lowercase();
            self.filtered = self.all_files
                .iter()
                .filter(|p| {
                    let name = p.to_string_lossy().to_lowercase();
                    fuzzy_match(&name, &query_lower)
                })
                .cloned()
                .collect();
        }
        
        // Reset selection
        if !self.filtered.is_empty() {
            self.list_state.select(Some(0));
        } else {
            self.list_state.select(None);
        }
    }

    pub fn next(&mut self) {
        if self.filtered.is_empty() { return; }
        let i = self.list_state.selected().unwrap_or(0);
        let next = if i >= self.filtered.len() - 1 { 0 } else { i + 1 };
        self.list_state.select(Some(next));
    }

    pub fn prev(&mut self) {
        if self.filtered.is_empty() { return; }
        let i = self.list_state.selected().unwrap_or(0);
        let prev = if i == 0 { self.filtered.len() - 1 } else { i - 1 };
        self.list_state.select(Some(prev));
    }

    pub fn selected(&self) -> Option<PathBuf> {
        self.list_state.selected().and_then(|i| self.filtered.get(i).cloned())
    }
}

// Simple fuzzy matching: all query chars must appear in order
fn fuzzy_match(text: &str, query: &str) -> bool {
    let mut text_chars = text.chars().peekable();
    for qc in query.chars() {
        loop {
            match text_chars.next() {
                Some(tc) if tc == qc => break,
                Some(_) => continue,
                None => return false,
            }
        }
    }
    true
}

fn collect_files(dir: &Path, max_depth: usize) -> Vec<PathBuf> {
    let mut files = Vec::new();
    collect_files_recursive(dir, dir, max_depth, 0, &mut files);
    files.sort();
    files
}

fn collect_files_recursive(base: &Path, dir: &Path, max_depth: usize, depth: usize, files: &mut Vec<PathBuf>) {
    if depth > max_depth { return; }
    
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    
    for entry in entries.flatten() {
        let path = entry.path();
        let name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
        
        // Skip hidden files and common ignore patterns
        if name.starts_with('.') || name == "node_modules" || name == "target" || name == "__pycache__" {
            continue;
        }
        
        if path.is_file() {
            // Store relative path
            if let Ok(rel) = path.strip_prefix(base) {
                files.push(rel.to_path_buf());
            }
        } else if path.is_dir() {
            collect_files_recursive(base, &path, max_depth, depth + 1, files);
        }
    }
}

pub fn render(f: &mut Frame, area: Rect, finder: &mut FuzzyFinder) {
    if !finder.visible { return; }

    // Modal size
    let width = (area.width * 70 / 100).min(80);
    let height = (area.height * 70 / 100).min(30);
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    let popup_area = Rect::new(x, y, width, height);

    f.render_widget(Clear, popup_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Fuzzy Find (Ctrl+P) ")
        .style(Style::default().bg(Color::Black).fg(Color::White));

    let inner = block.inner(popup_area);
    f.render_widget(block, popup_area);

    // Input line
    let input_area = Rect::new(inner.x, inner.y, inner.width, 1);
    let input_line = Line::from(vec![
        Span::styled("> ", Style::default().fg(Color::Cyan)),
        Span::styled(&finder.query, Style::default().fg(Color::Yellow)),
        Span::styled("_", Style::default().fg(Color::White).add_modifier(Modifier::SLOW_BLINK)),
    ]);
    f.render_widget(Paragraph::new(input_line), input_area);

    // Results count
    let count_area = Rect::new(inner.x, inner.y + 1, inner.width, 1);
    let count_text = format!("{}/{} files", finder.filtered.len(), finder.all_files.len());
    f.render_widget(
        Paragraph::new(count_text).style(Style::default().fg(Color::DarkGray)),
        count_area
    );

    // File list
    let list_area = Rect::new(inner.x, inner.y + 2, inner.width, inner.height.saturating_sub(2));
    
    let items: Vec<ListItem> = finder.filtered
        .iter()
        .take(list_area.height as usize)
        .map(|p| {
            let display = p.to_string_lossy().to_string();
            ListItem::new(display)
        })
        .collect();

    let list = List::new(items)
        .highlight_style(Style::default().bg(Color::Blue).fg(Color::White).add_modifier(Modifier::BOLD))
        .highlight_symbol("> ");

    f.render_stateful_widget(list, list_area, &mut finder.list_state);
}
