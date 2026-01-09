use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EditorMode {
    #[default]
    ReadOnly,
    Edit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PaneId {
    FileBrowser,
    Preview,
    Claude,
    LazyGit,
    Terminal,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileType {
    Code,
    Markdown,
    Html,
    Json,
    Image,
    Directory,
    Unknown,
}

/// Git file status for visual highlighting in file browser
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GitFileStatus {
    #[default]
    Unknown,
    /// File not tracked by git (yellow)
    Untracked,
    /// File has uncommitted changes (orange)
    Modified,
    /// File is staged for commit (green)
    Staged,
    /// File is ignored by .gitignore (gray/dim)
    Ignored,
    /// File has merge conflicts (red + bold)
    Conflict,
    /// File is tracked and unchanged
    Clean,
}

impl GitFileStatus {
    /// Priority for directory status aggregation (higher = more important)
    pub fn priority(&self) -> u8 {
        match self {
            GitFileStatus::Conflict => 5,
            GitFileStatus::Modified => 4,
            GitFileStatus::Untracked => 3,
            GitFileStatus::Staged => 2,
            GitFileStatus::Clean => 1,
            GitFileStatus::Ignored => 0,
            GitFileStatus::Unknown => 0,
        }
    }

    /// Symbol to display before filename
    pub fn symbol(&self) -> &'static str {
        match self {
            GitFileStatus::Untracked => "?",
            GitFileStatus::Modified => "M",
            GitFileStatus::Staged => "+",
            GitFileStatus::Ignored => "·",
            GitFileStatus::Conflict => "!",
            GitFileStatus::Clean | GitFileStatus::Unknown => " ",
        }
    }
}

/// Git repository information for footer display
#[derive(Debug, Clone, Default)]
pub struct GitRepoInfo {
    pub branch: String,
    pub modified_count: usize,
    pub untracked_count: usize,
    pub staged_count: usize,
}

#[derive(Debug, Clone, Default)]
pub struct Rect {
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
}

/// Terminal line selection state for copying output to Claude
#[derive(Debug, Clone, Default)]
pub struct TerminalSelection {
    /// Whether selection mode is active
    pub active: bool,
    /// Starting line of selection (screen-relative)
    pub start_line: Option<usize>,
    /// Ending line of selection (screen-relative)
    pub end_line: Option<usize>,
    /// Source pane for the selection
    pub source_pane: Option<PaneId>,
}

impl TerminalSelection {
    pub fn start(&mut self, line: usize, pane: PaneId) {
        self.active = true;
        self.start_line = Some(line);
        self.end_line = Some(line);
        self.source_pane = Some(pane);
    }

    pub fn extend(&mut self, line: usize) {
        if self.active {
            self.end_line = Some(line);
        }
    }

    pub fn clear(&mut self) {
        self.active = false;
        self.start_line = None;
        self.end_line = None;
        self.source_pane = None;
    }

    pub fn line_range(&self) -> Option<(usize, usize)> {
        match (self.start_line, self.end_line) {
            (Some(start), Some(end)) => {
                let min = start.min(end);
                let max = start.max(end);
                Some((min, max))
            }
            _ => None,
        }
    }

    pub fn is_line_selected(&self, line: usize) -> bool {
        if let Some((min, max)) = self.line_range() {
            line >= min && line <= max
        } else {
            false
        }
    }
}

/// Drag and drop state for file browser to terminal panes
#[derive(Debug, Clone, Default)]
pub struct DragState {
    /// Whether a drag operation is in progress
    pub dragging: bool,
    /// The file/folder path being dragged
    pub dragged_path: Option<std::path::PathBuf>,
    /// Current mouse X position during drag
    pub current_x: u16,
    /// Current mouse Y position during drag
    pub current_y: u16,
}

impl DragState {
    pub fn start(&mut self, path: std::path::PathBuf, x: u16, y: u16) {
        self.dragging = true;
        self.dragged_path = Some(path);
        self.current_x = x;
        self.current_y = y;
    }

    pub fn update_position(&mut self, x: u16, y: u16) {
        if self.dragging {
            self.current_x = x;
            self.current_y = y;
        }
    }

    pub fn finish(&mut self) -> Option<std::path::PathBuf> {
        let path = self.dragged_path.take();
        self.clear();
        path
    }

    pub fn clear(&mut self) {
        self.dragging = false;
        self.dragged_path = None;
    }
}

/// Help screen state with scrolling support
#[derive(Debug, Clone, Default)]
pub struct HelpState {
    /// Whether help is visible
    pub visible: bool,
    /// Current scroll position (line offset)
    pub scroll: usize,
    /// Cached popup area for mouse events
    pub popup_area: Option<ratatui::layout::Rect>,
    /// Cached content area (inside borders)
    pub content_area: Option<ratatui::layout::Rect>,
    /// Total number of content lines
    pub total_lines: usize,
}

impl HelpState {
    pub fn open(&mut self) {
        self.visible = true;
        self.scroll = 0;
    }

    pub fn close(&mut self) {
        self.visible = false;
        self.scroll = 0;
        self.popup_area = None;
        self.content_area = None;
    }

    pub fn scroll_up(&mut self, amount: usize) {
        self.scroll = self.scroll.saturating_sub(amount);
    }

    pub fn scroll_down(&mut self, amount: usize) {
        let max_scroll = self.max_scroll();
        self.scroll = (self.scroll + amount).min(max_scroll);
    }

    pub fn scroll_to_top(&mut self) {
        self.scroll = 0;
    }

    pub fn scroll_to_bottom(&mut self) {
        self.scroll = self.max_scroll();
    }

    pub fn page_up(&mut self) {
        self.scroll_up(10);
    }

    pub fn page_down(&mut self) {
        self.scroll_down(10);
    }

    fn max_scroll(&self) -> usize {
        let visible_lines = self.content_area.map(|r| r.height as usize).unwrap_or(20);
        self.total_lines.saturating_sub(visible_lines)
    }

    /// Check if a point is inside the popup area
    pub fn contains(&self, x: u16, y: u16) -> bool {
        if let Some(area) = self.popup_area {
            x >= area.x && x < area.x + area.width && y >= area.y && y < area.y + area.height
        } else {
            false
        }
    }
}

/// Mouse-based text selection in terminal panes
#[derive(Debug, Clone, Default)]
pub struct MouseSelection {
    /// Whether mouse selection is in progress
    pub selecting: bool,
    /// Source pane for the selection
    pub source_pane: Option<PaneId>,
    /// Starting Y position (screen coordinate)
    pub start_y: u16,
    /// Current Y position (screen coordinate)
    pub current_y: u16,
    /// Pane area for coordinate conversion
    pub pane_area: Option<ratatui::layout::Rect>,
}

impl MouseSelection {
    /// Start a new mouse selection
    pub fn start(&mut self, pane: PaneId, y: u16, area: ratatui::layout::Rect) {
        self.selecting = true;
        self.source_pane = Some(pane);
        self.start_y = y;
        self.current_y = y;
        self.pane_area = Some(area);
    }

    /// Update selection during drag
    pub fn update(&mut self, y: u16) {
        if self.selecting {
            // Clamp Y to pane boundaries to prevent selection overflow
            if let Some(area) = self.pane_area {
                let min_y = area.y + 1;  // Account for top border
                let max_y = area.y + area.height.saturating_sub(2);  // Account for bottom border
                self.current_y = y.clamp(min_y, max_y);
            } else {
                self.current_y = y;
            }
        }
    }

    /// Convert screen Y to line index within pane (0-based, accounting for border)
    fn screen_y_to_line(&self, y: u16) -> Option<usize> {
        let area = self.pane_area?;
        // Account for top border (1 pixel)
        if y <= area.y || y >= area.y + area.height - 1 {
            return None;
        }
        Some((y - area.y - 1) as usize)
    }

    /// Get the selected line range (min, max) as terminal line indices
    pub fn line_range(&self) -> Option<(usize, usize)> {
        if !self.selecting {
            return None;
        }
        let start_line = self.screen_y_to_line(self.start_y)?;
        let end_line = self.screen_y_to_line(self.current_y)?;
        Some((start_line.min(end_line), start_line.max(end_line)))
    }

    /// Check if a line is selected
    pub fn is_line_selected(&self, line: usize) -> bool {
        if let Some((min, max)) = self.line_range() {
            line >= min && line <= max
        } else {
            false
        }
    }

    /// Finish selection and return (start_line, end_line, pane)
    pub fn finish(&mut self) -> Option<(usize, usize, PaneId)> {
        if !self.selecting {
            return None;
        }
        let (start, end) = self.line_range()?;
        let pane = self.source_pane?;
        self.clear();
        Some((start, end, pane))
    }

    /// Clear selection state
    pub fn clear(&mut self) {
        self.selecting = false;
        self.source_pane = None;
        self.pane_area = None;
    }

    /// Check if selection is active for a specific pane
    pub fn is_selecting_in(&self, pane: PaneId) -> bool {
        self.selecting && self.source_pane == Some(pane)
    }
}

/// Search state for Preview/Edit mode
#[derive(Debug, Clone, Default)]
pub struct SearchState {
    /// Whether search mode is active
    pub active: bool,
    /// Current search query
    pub query: String,
    /// Found matches: (line_index, start_col, end_col)
    pub matches: Vec<(usize, usize, usize)>,
    /// Index of currently highlighted match
    pub current_match: usize,
    /// Case-sensitive search
    pub case_sensitive: bool,
}

impl SearchState {
    /// Open search mode
    pub fn open(&mut self) {
        self.active = true;
        self.query.clear();
        self.matches.clear();
        self.current_match = 0;
    }

    /// Close search mode and clear state
    pub fn close(&mut self) {
        self.active = false;
        self.query.clear();
        self.matches.clear();
        self.current_match = 0;
    }

    /// Move to next match (wraps around)
    pub fn next_match(&mut self) {
        if !self.matches.is_empty() {
            self.current_match = (self.current_match + 1) % self.matches.len();
        }
    }

    /// Move to previous match (wraps around)
    pub fn prev_match(&mut self) {
        if !self.matches.is_empty() {
            self.current_match = self
                .current_match
                .checked_sub(1)
                .unwrap_or(self.matches.len() - 1);
        }
    }

    /// Get the line number of the current match
    pub fn current_match_line(&self) -> Option<usize> {
        self.matches.get(self.current_match).map(|(line, _, _)| *line)
    }

    /// Check if a position is a match (for highlighting)
    pub fn is_match(&self, line: usize, col: usize) -> bool {
        self.matches
            .iter()
            .any(|(l, start, end)| *l == line && col >= *start && col < *end)
    }

    /// Check if a match at (line, start_col) is the current match
    pub fn is_current_match(&self, line: usize, start_col: usize) -> bool {
        self.matches
            .get(self.current_match)
            .map(|(l, s, _)| *l == line && *s == start_col)
            .unwrap_or(false)
    }
}
