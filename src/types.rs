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
