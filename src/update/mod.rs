//! Self-update functionality for claude-workbench
//!
//! Uses GitHub Releases as backend via the `self_update` crate.
//! Provides non-blocking update checks and update execution.

use self_update::backends::github::Update;
use std::sync::mpsc;
use std::thread;

/// Current version from Cargo.toml
pub const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");

/// GitHub repository configuration
const REPO_OWNER: &str = "eqms";
const REPO_NAME: &str = "claude-workbench";
const BIN_NAME: &str = "claude-workbench";

/// Result of an update check
#[derive(Debug, Clone)]
pub enum UpdateCheckResult {
    /// Currently running the latest version
    UpToDate,
    /// A newer version is available
    UpdateAvailable { version: String },
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
    /// Error from last check (if any)
    pub error: Option<String>,
    /// Whether to show the update dialog
    pub show_dialog: bool,
    /// Progress message during update
    pub progress_message: Option<String>,
    /// Whether this was a manual check (show errors) vs automatic (silent fail)
    pub manual_check: bool,
}

impl UpdateState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Mark update as available
    pub fn set_available(&mut self, version: String) {
        self.checking = false;
        self.available_version = Some(version);
        self.error = None;
        self.show_dialog = true;
    }

    /// Mark as up-to-date
    pub fn set_up_to_date(&mut self) {
        self.checking = false;
        self.available_version = None;
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
    }

    /// Start checking for updates
    pub fn start_check(&mut self) {
        self.checking = true;
        self.error = None;
    }

    /// Start updating
    pub fn start_update(&mut self) {
        self.updating = true;
        self.progress_message = Some("Downloading update...".to_string());
    }

    /// Update completed
    pub fn finish_update(&mut self) {
        self.updating = false;
        self.progress_message = None;
        self.show_dialog = false;
    }
}

/// Check for updates synchronously (blocking)
///
/// This should be called from a background thread.
pub fn check_for_update_sync() -> UpdateCheckResult {
    match Update::configure()
        .repo_owner(REPO_OWNER)
        .repo_name(REPO_NAME)
        .bin_name(BIN_NAME)
        .current_version(CURRENT_VERSION)
        .build()
    {
        Ok(updater) => {
            // Get the target version (latest available)
            match updater.target_version() {
                Some(target) => {
                    // self_update returns the latest release version
                    // If it differs from current, an update is available
                    if target == CURRENT_VERSION {
                        UpdateCheckResult::UpToDate
                    } else {
                        UpdateCheckResult::UpdateAvailable { version: target }
                    }
                }
                None => UpdateCheckResult::UpToDate,
            }
        }
        Err(e) => UpdateCheckResult::Error(format!("{}", e)),
    }
}

/// Start an async update check
///
/// Returns a receiver that will receive the result when the check is complete.
pub fn check_for_update_async() -> mpsc::Receiver<UpdateCheckResult> {
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let result = check_for_update_sync();
        let _ = tx.send(result);
    });
    rx
}

/// Perform the actual update (blocking)
///
/// This downloads and replaces the current binary.
/// The application should be restarted after this completes.
pub fn perform_update_sync() -> UpdateResult {
    match Update::configure()
        .repo_owner(REPO_OWNER)
        .repo_name(REPO_NAME)
        .bin_name(BIN_NAME)
        .current_version(CURRENT_VERSION)
        .show_download_progress(false) // We show our own UI
        .build()
    {
        Ok(updater) => match updater.update() {
            Ok(status) => UpdateResult::Success {
                old_version: CURRENT_VERSION.to_string(),
                new_version: status.version().to_string(),
            },
            Err(e) => UpdateResult::Error(format!("Update failed: {}", e)),
        },
        Err(e) => UpdateResult::Error(format!("Failed to configure update: {}", e)),
    }
}

/// Start an async update
///
/// Returns a receiver that will receive the result when the update is complete.
pub fn perform_update_async() -> mpsc::Receiver<UpdateResult> {
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let result = perform_update_sync();
        let _ = tx.send(result);
    });
    rx
}

/// Get the current version string
pub fn current_version() -> &'static str {
    CURRENT_VERSION
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_current_version() {
        let version = current_version();
        assert!(!version.is_empty());
        // Version should be semver format
        let parts: Vec<&str> = version.split('.').collect();
        assert!(parts.len() >= 2, "Version should have at least major.minor");
    }

    #[test]
    fn test_update_state_default() {
        let state = UpdateState::new();
        assert!(!state.checking);
        assert!(!state.updating);
        assert!(state.available_version.is_none());
        assert!(state.error.is_none());
        assert!(!state.show_dialog);
    }

    #[test]
    fn test_update_state_transitions() {
        let mut state = UpdateState::new();

        // Start check
        state.start_check();
        assert!(state.checking);

        // Set available
        state.set_available("1.0.0".to_string());
        assert!(!state.checking);
        assert_eq!(state.available_version, Some("1.0.0".to_string()));
        assert!(state.show_dialog);

        // Close dialog
        state.close_dialog();
        assert!(!state.show_dialog);
    }
}
