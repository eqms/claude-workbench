use serde::{Deserialize, Serialize};


#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Default)]
pub struct Rect {
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
}
