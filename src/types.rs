use serde::{Deserialize, Serialize};

/// Claude Code permission mode for controlling tool access
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ClaudePermissionMode {
    #[default]
    Default,
    AcceptEdits,
    Plan,
    BypassPermissions,
    DangerouslySkipPermissions, // YOLO-Mode
}

impl ClaudePermissionMode {
    /// Get the CLI flag value for --permission-mode (None for YOLO which uses separate flag)
    pub fn cli_flag(&self) -> Option<&'static str> {
        match self {
            Self::DangerouslySkipPermissions => None,
            Self::Default => Some("default"),
            Self::AcceptEdits => Some("acceptEdits"),
            Self::Plan => Some("plan"),
            Self::BypassPermissions => Some("bypassPermissions"),
        }
    }

    /// Check if this is YOLO mode (uses --dangerously-skip-permissions flag)
    pub fn is_yolo(&self) -> bool {
        matches!(self, Self::DangerouslySkipPermissions)
    }

    /// Get display name for the mode
    pub fn name(&self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::AcceptEdits => "acceptEdits",
            Self::Plan => "plan",
            Self::BypassPermissions => "bypassPermissions",
            Self::DangerouslySkipPermissions => "dangerouslySkip",
        }
    }

    /// Get German description for the mode
    pub fn description_de(&self) -> &'static str {
        match self {
            Self::Default => "Standard - fragt bei jeder Tool-Nutzung",
            Self::AcceptEdits => "Akzeptiert Datei-Edits automatisch",
            Self::Plan => "Nur-Lesen-Modus, keine Änderungen",
            Self::BypassPermissions => "Voller Zugriff ohne Nachfragen",
            Self::DangerouslySkipPermissions => "⚠️ YOLO - Alle Sicherheitsabfragen aus!",
        }
    }

    /// Get all available modes
    pub fn all() -> &'static [Self] {
        &[
            Self::Default,
            Self::AcceptEdits,
            Self::Plan,
            Self::BypassPermissions,
            Self::DangerouslySkipPermissions,
        ]
    }
}

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

/// Help screen state with scrolling and search support
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
    // Search functionality
    /// Current search query
    pub search_query: String,
    /// Whether search input is active (focused)
    pub search_active: bool,
    /// Indices of lines matching the search query
    pub filtered_lines: Vec<usize>,
    /// Number of matches found (for display)
    pub match_count: usize,
}

impl HelpState {
    pub fn open(&mut self) {
        self.visible = true;
        self.scroll = 0;
        self.search_query.clear();
        self.search_active = false;
        self.filtered_lines.clear();
        self.match_count = 0;
    }

    pub fn close(&mut self) {
        self.visible = false;
        self.scroll = 0;
        self.popup_area = None;
        self.content_area = None;
        self.search_query.clear();
        self.search_active = false;
        self.filtered_lines.clear();
        self.match_count = 0;
    }

    /// Activate search input field
    pub fn start_search(&mut self) {
        self.search_active = true;
    }

    /// Deactivate search input field (keep query for navigation)
    pub fn stop_search(&mut self) {
        self.search_active = false;
    }

    /// Clear search query and results
    pub fn clear_search(&mut self) {
        self.search_query.clear();
        self.filtered_lines.clear();
        self.match_count = 0;
        self.scroll = 0;
    }

    /// Add a character to the search query
    pub fn search_add_char(&mut self, c: char) {
        self.search_query.push(c);
    }

    /// Remove last character from search query
    pub fn search_backspace(&mut self) {
        self.search_query.pop();
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

/// Mouse-based text selection in terminal panes (character-level)
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
    /// Starting X position (screen coordinate) for character-level selection
    pub start_x: u16,
    /// Current X position (screen coordinate) for character-level selection
    pub current_x: u16,
    /// Pane area for coordinate conversion
    pub pane_area: Option<ratatui::layout::Rect>,
}

impl MouseSelection {
    /// Start a new mouse selection with character-level coordinates
    pub fn start(&mut self, pane: PaneId, x: u16, y: u16, area: ratatui::layout::Rect) {
        self.selecting = true;
        self.source_pane = Some(pane);
        self.start_x = x;
        self.start_y = y;
        self.current_x = x;
        self.current_y = y;
        self.pane_area = Some(area);
    }

    /// Update selection during drag with character-level coordinates
    pub fn update(&mut self, x: u16, y: u16) {
        if self.selecting {
            // Clamp coordinates to pane boundaries to prevent selection overflow
            if let Some(area) = self.pane_area {
                let min_x = area.x + 1; // Account for left border
                let max_x = area.x + area.width.saturating_sub(2); // Account for right border
                let min_y = area.y + 1; // Account for top border
                let max_y = area.y + area.height.saturating_sub(2); // Account for bottom border
                self.current_x = x.clamp(min_x, max_x);
                self.current_y = y.clamp(min_y, max_y);
            } else {
                self.current_x = x;
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

    /// Convert screen X to column index within pane (0-based, accounting for border)
    fn screen_x_to_col(&self, x: u16) -> Option<usize> {
        let area = self.pane_area?;
        // Account for left border (1 pixel)
        if x <= area.x || x >= area.x + area.width - 1 {
            return None;
        }
        Some((x - area.x - 1) as usize)
    }

    /// Get the character-level selection range: ((start_row, start_col), (end_row, end_col))
    /// Returns coordinates normalized so start is always before end
    pub fn char_range(&self) -> Option<((usize, usize), (usize, usize))> {
        if !self.selecting {
            return None;
        }
        let start_line = self.screen_y_to_line(self.start_y)?;
        let end_line = self.screen_y_to_line(self.current_y)?;
        let start_col = self.screen_x_to_col(self.start_x)?;
        let end_col = self.screen_x_to_col(self.current_x)?;

        // Normalize: ensure start is before end (by row, then by column)
        if start_line < end_line || (start_line == end_line && start_col <= end_col) {
            Some(((start_line, start_col), (end_line, end_col)))
        } else {
            Some(((end_line, end_col), (start_line, start_col)))
        }
    }

    /// Get character selection for a specific line
    /// Returns Option<(start_col, end_col)> for the selected portion of this line
    pub fn get_line_selection(&self, line: usize) -> Option<(usize, usize)> {
        let ((start_row, start_col), (end_row, end_col)) = self.char_range()?;

        if line < start_row || line > end_row {
            // Line not in selection
            return None;
        }

        if start_row == end_row {
            // Single-line selection
            Some((start_col, end_col))
        } else if line == start_row {
            // First line of multi-line selection: from start_col to end of line (use large value)
            Some((start_col, usize::MAX))
        } else if line == end_row {
            // Last line of multi-line selection: from start to end_col
            Some((0, end_col))
        } else {
            // Middle line: entire line selected
            Some((0, usize::MAX))
        }
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

    /// Finish selection and return character-level range with pane
    /// Returns ((start_row, start_col), (end_row, end_col), pane)
    #[allow(clippy::type_complexity)]
    pub fn finish(&mut self) -> Option<((usize, usize), (usize, usize), PaneId)> {
        if !self.selecting {
            return None;
        }
        let char_range = self.char_range()?;
        let pane = self.source_pane?;
        self.clear();
        Some((char_range.0, char_range.1, pane))
    }

    /// Finish selection and return line-level range (for backwards compatibility)
    /// Returns (start_line, end_line, pane)
    pub fn finish_lines(&mut self) -> Option<(usize, usize, PaneId)> {
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
        self.start_x = 0;
        self.start_y = 0;
        self.current_x = 0;
        self.current_y = 0;
        self.pane_area = None;
    }

    /// Check if selection is active for a specific pane
    pub fn is_selecting_in(&self, pane: PaneId) -> bool {
        self.selecting && self.source_pane == Some(pane)
    }
}

/// Search mode: Search only vs. Search & Replace (MC Edit style)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SearchMode {
    #[default]
    Search,
    Replace,
}

/// Search state for Preview/Edit mode
#[derive(Debug, Clone, Default)]
pub struct SearchState {
    /// Whether search mode is active
    pub active: bool,
    /// Current search query
    pub query: String,
    /// Cursor position in query (char index, not byte)
    pub query_cursor: usize,
    /// Found matches: (line_index, start_col, end_col)
    pub matches: Vec<(usize, usize, usize)>,
    /// Index of currently highlighted match
    pub current_match: usize,
    /// Case-sensitive search
    pub case_sensitive: bool,
    /// Search mode: Search only or Search & Replace
    pub mode: SearchMode,
    /// Replacement text (for Replace mode)
    pub replace_text: String,
    /// Cursor position in replace_text (char index, not byte)
    pub replace_cursor: usize,
    /// Which field has focus: false = search, true = replace
    pub focus_on_replace: bool,
}

impl SearchState {
    /// Open search mode
    pub fn open(&mut self) {
        self.active = true;
        self.query.clear();
        self.query_cursor = 0;
        self.matches.clear();
        self.current_match = 0;
        self.mode = SearchMode::Search;
        self.replace_text.clear();
        self.replace_cursor = 0;
        self.focus_on_replace = false;
    }

    /// Close search mode and clear state
    pub fn close(&mut self) {
        self.active = false;
        self.query.clear();
        self.query_cursor = 0;
        self.matches.clear();
        self.current_match = 0;
        self.mode = SearchMode::Search;
        self.replace_text.clear();
        self.replace_cursor = 0;
        self.focus_on_replace = false;
    }

    /// Toggle between Search and Replace mode
    pub fn toggle_replace_mode(&mut self) {
        match self.mode {
            SearchMode::Search => {
                self.mode = SearchMode::Replace;
                self.focus_on_replace = false;
            }
            SearchMode::Replace => {
                self.mode = SearchMode::Search;
                self.focus_on_replace = false;
            }
        }
    }

    /// Switch focus between search and replace fields (Tab key)
    pub fn toggle_field_focus(&mut self) {
        if self.mode == SearchMode::Replace {
            self.focus_on_replace = !self.focus_on_replace;
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Cursor Navigation Methods (UTF-8 safe)
    // ─────────────────────────────────────────────────────────────────────────

    /// Get reference to active field text
    fn active_text(&self) -> &str {
        if self.focus_on_replace {
            &self.replace_text
        } else {
            &self.query
        }
    }

    /// Get mutable reference to active cursor
    fn active_cursor_mut(&mut self) -> &mut usize {
        if self.focus_on_replace {
            &mut self.replace_cursor
        } else {
            &mut self.query_cursor
        }
    }

    /// Get active cursor position
    pub fn active_cursor(&self) -> usize {
        if self.focus_on_replace {
            self.replace_cursor
        } else {
            self.query_cursor
        }
    }

    /// Move cursor left in active field
    pub fn cursor_left(&mut self) {
        let cursor = self.active_cursor_mut();
        if *cursor > 0 {
            *cursor -= 1;
        }
    }

    /// Move cursor right in active field
    pub fn cursor_right(&mut self) {
        let len = self.active_text().chars().count();
        let cursor = self.active_cursor_mut();
        if *cursor < len {
            *cursor += 1;
        }
    }

    /// Move cursor to start of active field
    pub fn cursor_home(&mut self) {
        *self.active_cursor_mut() = 0;
    }

    /// Move cursor to end of active field
    pub fn cursor_end(&mut self) {
        let len = self.active_text().chars().count();
        *self.active_cursor_mut() = len;
    }

    /// Insert character at cursor position in active field
    pub fn insert_char(&mut self, c: char) {
        let cursor = if self.focus_on_replace {
            self.replace_cursor
        } else {
            self.query_cursor
        };

        let text = if self.focus_on_replace {
            &mut self.replace_text
        } else {
            &mut self.query
        };

        // Convert char index to byte index for insertion
        let byte_pos: usize = text.chars().take(cursor).map(|c| c.len_utf8()).sum();
        text.insert(byte_pos, c);

        // Advance cursor
        if self.focus_on_replace {
            self.replace_cursor += 1;
        } else {
            self.query_cursor += 1;
        }
    }

    /// Delete character before cursor (Backspace)
    pub fn delete_char_before(&mut self) {
        let cursor = if self.focus_on_replace {
            self.replace_cursor
        } else {
            self.query_cursor
        };

        if cursor == 0 {
            return;
        }

        let text = if self.focus_on_replace {
            &mut self.replace_text
        } else {
            &mut self.query
        };

        // Find byte position of char to remove
        let byte_start: usize = text.chars().take(cursor - 1).map(|c| c.len_utf8()).sum();
        let char_to_remove = text.chars().nth(cursor - 1).unwrap();
        let byte_end = byte_start + char_to_remove.len_utf8();
        text.replace_range(byte_start..byte_end, "");

        // Move cursor back
        if self.focus_on_replace {
            self.replace_cursor -= 1;
        } else {
            self.query_cursor -= 1;
        }
    }

    /// Delete character at cursor (Delete key)
    pub fn delete_char_at(&mut self) {
        let cursor = if self.focus_on_replace {
            self.replace_cursor
        } else {
            self.query_cursor
        };

        let text = if self.focus_on_replace {
            &mut self.replace_text
        } else {
            &mut self.query
        };

        let len = text.chars().count();
        if cursor >= len {
            return;
        }

        // Find byte position of char to remove
        let byte_start: usize = text.chars().take(cursor).map(|c| c.len_utf8()).sum();
        let char_to_remove = text.chars().nth(cursor).unwrap();
        let byte_end = byte_start + char_to_remove.len_utf8();
        text.replace_range(byte_start..byte_end, "");
        // Cursor stays in place
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
        self.matches
            .get(self.current_match)
            .map(|(line, _, _)| *line)
    }

    /// Get current match position: (line, start_col, end_col)
    pub fn get_current_match(&self) -> Option<(usize, usize, usize)> {
        self.matches.get(self.current_match).copied()
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

/// Result of checking for remote changes
#[derive(Debug, Clone)]
pub enum GitRemoteCheckResult {
    /// No changes detected - local is up to date
    UpToDate,
    /// Remote has commits ahead of local
    RemoteAhead {
        commits_ahead: usize,
        branch: String,
    },
    /// Check failed (network error, no remote configured, etc.)
    Error(String),
}

/// State for tracking background git remote operations
#[derive(Debug, Clone, Default)]
pub struct GitRemoteState {
    /// Last checked repo root (to detect when user enters different repo)
    pub last_repo_root: Option<std::path::PathBuf>,
    /// Whether a check is currently in progress
    pub checking: bool,
}
