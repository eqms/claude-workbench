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
