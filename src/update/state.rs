//! State machine and result types for the update flow.

/// Result of an update check
#[derive(Debug, Clone)]
pub enum UpdateCheckResult {
    /// Currently running the latest version
    UpToDate,
    /// A newer version is available
    UpdateAvailable {
        version: String,
        /// Release notes (body) from GitHub Release
        release_notes: Option<String>,
    },
    /// No releases found (may indicate missing assets for this platform)
    NoReleasesFound,
    /// Check failed with error message
    Error(String),
}

/// Result of an update operation
#[derive(Debug, Clone)]
pub enum UpdateResult {
    /// Update completed successfully
    Success {
        old_version: String,
        new_version: String,
    },
    /// Update failed with error message
    Error(String),
}

/// State for the update checker
#[derive(Debug, Clone, Default)]
pub struct UpdateState {
    /// Whether an update check is currently in progress
    pub checking: bool,
    /// Whether an update is currently being downloaded/installed
    pub updating: bool,
    /// Result of the last check (if any)
    pub available_version: Option<String>,
    /// Release notes for the available version
    pub release_notes: Option<String>,
    /// Scroll position for viewing release notes
    pub release_notes_scroll: u16,
    /// Error from last check (if any)
    pub error: Option<String>,
    /// Whether to show the update dialog
    pub show_dialog: bool,
    /// Progress message during update
    pub progress_message: Option<String>,
    /// Whether this was a manual check (show errors) vs automatic (silent fail)
    pub manual_check: bool,
    /// Log messages from the update process (for debugging in UI)
    pub log_messages: Vec<String>,
    /// Whether update completed successfully (show success message)
    pub update_success: bool,
    /// Version that was installed (for success message)
    pub installed_version: Option<String>,
}

impl UpdateState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Mark update as available
    pub fn set_available(&mut self, version: String, release_notes: Option<String>) {
        self.checking = false;
        self.available_version = Some(version);
        self.release_notes = release_notes;
        self.release_notes_scroll = 0;
        self.error = None;
        self.show_dialog = true;
    }

    /// Scroll release notes up
    pub fn scroll_release_notes_up(&mut self) {
        self.release_notes_scroll = self.release_notes_scroll.saturating_sub(1);
    }

    /// Scroll release notes down
    pub fn scroll_release_notes_down(&mut self, max_scroll: u16) {
        if self.release_notes_scroll < max_scroll {
            self.release_notes_scroll += 1;
        }
    }

    /// Mark as up-to-date
    pub fn set_up_to_date(&mut self) {
        self.checking = false;
        self.available_version = None;
        self.release_notes = None;
        self.release_notes_scroll = 0;
        self.error = None;
    }

    /// Set error state
    pub fn set_error(&mut self, msg: String) {
        self.checking = false;
        self.error = Some(msg);
    }

    /// Close the update dialog
    pub fn close_dialog(&mut self) {
        self.show_dialog = false;
        self.update_success = false;
        self.installed_version = None;
    }

    /// Start checking for updates
    pub fn start_check(&mut self) {
        self.checking = true;
        self.error = None;
    }

    /// Start updating
    pub fn start_update(&mut self) {
        self.updating = true;
        self.progress_message = Some("Connecting to GitHub...".to_string());
        self.log_messages.clear();
    }

    /// Add a log message for debugging
    pub fn add_log(&mut self, msg: String) {
        self.log_messages.push(msg);
    }

    /// Set progress message during update
    pub fn set_progress(&mut self, msg: String) {
        self.progress_message = Some(msg);
    }

    /// Update completed
    pub fn finish_update(&mut self) {
        self.updating = false;
        self.progress_message = None;
        self.show_dialog = false;
    }

    /// Set update success state
    pub fn set_success(&mut self, new_version: String) {
        self.updating = false;
        self.update_success = true;
        self.installed_version = Some(new_version);
        self.available_version = None;
        self.release_notes = None;
        self.error = None;
        self.progress_message = None;
        self.show_dialog = true;
    }

    /// Reset success state (when dialog is closed)
    pub fn clear_success(&mut self) {
        self.update_success = false;
        self.installed_version = None;
        self.show_dialog = false;
    }
}
