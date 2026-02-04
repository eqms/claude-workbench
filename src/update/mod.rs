//! Self-update functionality for claude-workbench
//!
//! Uses GitHub Releases as backend via the `self_update` crate.
//! Provides non-blocking update checks and update execution.

use self_update::backends::github::{ReleaseList, Update};
use std::io::Write;
use std::sync::mpsc;
use std::thread;

/// Log file path for update debugging
pub const LOG_FILE: &str = "/tmp/claude-workbench-update.log";

/// Write a log message to the update log file
pub fn log_update(msg: &str) {
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(LOG_FILE)
    {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let _ = writeln!(file, "[{}] {}", timestamp, msg);
    }
}

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
        // Also clear success state when dialog is closed
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
        self.available_version = None; // Clear to prevent showing "Update Available" again
        self.release_notes = None;
        self.error = None;
        self.progress_message = None;
        self.show_dialog = true; // Keep dialog open to show success
    }

    /// Reset success state (when dialog is closed)
    pub fn clear_success(&mut self) {
        self.update_success = false;
        self.installed_version = None;
        self.show_dialog = false;
    }
}

/// Fetch release notes for a specific version from GitHub
///
/// Returns the body text of the release if available.
pub fn fetch_release_notes(version: &str) -> Option<String> {
    let releases = ReleaseList::configure()
        .repo_owner(REPO_OWNER)
        .repo_name(REPO_NAME)
        .build()
        .ok()?
        .fetch()
        .ok()?;

    // Normalize version: strip 'v' prefix for comparison
    let version_normalized = version.strip_prefix('v').unwrap_or(version);

    releases
        .into_iter()
        .find(|r| {
            let release_version = r.version.strip_prefix('v').unwrap_or(&r.version);
            release_version == version_normalized
        })
        .and_then(|r| r.body)
}

/// Filter release notes to show only downloads for the current platform
///
/// This helps users see relevant download links instead of all platform variants.
pub fn filter_release_notes_for_platform(notes: &str) -> String {
    let current_os = std::env::consts::OS; // "macos", "linux", "windows"
    let current_arch = std::env::consts::ARCH; // "aarch64", "x86_64"

    // Platform-specific labels used in release notes
    let platform_label = match (current_os, current_arch) {
        ("macos", "aarch64") => "Apple Silicon",
        ("macos", "x86_64") => "macOS Intel",
        ("linux", "aarch64") => "Linux ARM64",
        ("linux", "x86_64") => "Linux x64",
        ("windows", "x86_64") => "Windows x64",
        ("windows", "aarch64") => "Windows ARM64",
        _ => return notes.to_string(), // Unknown platform: show all
    };

    let mut result = Vec::new();
    let mut in_table = false;

    for line in notes.lines() {
        // Detect table start (line starts with | and contains header-like content)
        if line.starts_with('|') && !in_table {
            in_table = true;
            result.push(line);
            continue;
        }

        // Detect table separator (|---|---|)
        if in_table && line.starts_with('|') && line.contains("---") {
            result.push(line);
            continue;
        }

        // In table: filter rows
        if in_table && line.starts_with('|') {
            if line.contains(platform_label) {
                result.push(line);
            }
            // Skip other platform rows
            continue;
        }

        // Table ends when line doesn't start with |
        if in_table && !line.starts_with('|') {
            in_table = false;
        }

        // Outside table: keep all content
        result.push(line);
    }

    result.join("\n")
}

/// Compare two semver versions, returns true if `new` is newer than `current`
pub fn version_newer(new: &str, current: &str) -> bool {
    let new_normalized = new.strip_prefix('v').unwrap_or(new);
    let current_normalized = current.strip_prefix('v').unwrap_or(current);

    let parse_version = |s: &str| -> (u32, u32, u32) {
        let parts: Vec<&str> = s.split('.').collect();
        let major = parts.first().and_then(|p| p.parse().ok()).unwrap_or(0);
        let minor = parts.get(1).and_then(|p| p.parse().ok()).unwrap_or(0);
        let patch = parts.get(2).and_then(|p| p.parse().ok()).unwrap_or(0);
        (major, minor, patch)
    };

    let new_v = parse_version(new_normalized);
    let current_v = parse_version(current_normalized);

    new_v > current_v
}

/// Check for updates synchronously with a specific version (for testing)
///
/// Allows overriding the current version for testing purposes.
pub fn check_for_update_with_version(current_version: &str) -> UpdateCheckResult {
    #[cfg(debug_assertions)]
    {
        eprintln!("[Update] Current version: {}", current_version);
        eprintln!("[Update] Checking GitHub: {}/{}", REPO_OWNER, REPO_NAME);
        eprintln!("[Update] Binary name: {}", BIN_NAME);
        eprintln!(
            "[Update] Platform: {}-{}",
            std::env::consts::OS,
            std::env::consts::ARCH
        );
    }

    // Use ReleaseList directly to get full release info including body
    match ReleaseList::configure()
        .repo_owner(REPO_OWNER)
        .repo_name(REPO_NAME)
        .build()
    {
        Ok(release_list) => match release_list.fetch() {
            Ok(releases) => {
                if releases.is_empty() {
                    #[cfg(debug_assertions)]
                    eprintln!("[Update] No releases found");
                    return UpdateCheckResult::NoReleasesFound;
                }

                // Find the latest release (first one is usually latest)
                let latest = &releases[0];
                let target_version = &latest.version;

                #[cfg(debug_assertions)]
                eprintln!("[Update] GitHub version: {}", target_version);

                // Normalize versions
                let current_normalized =
                    current_version.strip_prefix('v').unwrap_or(current_version);
                let target_normalized = target_version.strip_prefix('v').unwrap_or(target_version);

                if target_normalized == current_normalized {
                    #[cfg(debug_assertions)]
                    eprintln!("[Update] Already up-to-date");
                    UpdateCheckResult::UpToDate
                } else if version_newer(target_version, current_version) {
                    #[cfg(debug_assertions)]
                    eprintln!(
                        "[Update] Update available: {} -> {}",
                        current_normalized, target_normalized
                    );
                    UpdateCheckResult::UpdateAvailable {
                        version: target_version.clone(),
                        release_notes: latest
                            .body
                            .as_ref()
                            .map(|n| filter_release_notes_for_platform(n)),
                    }
                } else {
                    // Current is newer (development version)
                    #[cfg(debug_assertions)]
                    eprintln!("[Update] Current version is newer than latest release");
                    UpdateCheckResult::UpToDate
                }
            }
            Err(e) => {
                #[cfg(debug_assertions)]
                eprintln!("[Update] Error fetching releases: {}", e);
                UpdateCheckResult::Error(format!("{}", e))
            }
        },
        Err(e) => {
            #[cfg(debug_assertions)]
            eprintln!("[Update] Error: {}", e);
            UpdateCheckResult::Error(format!("{}", e))
        }
    }
}

/// Check for updates synchronously (blocking)
///
/// This should be called from a background thread.
pub fn check_for_update_sync() -> UpdateCheckResult {
    check_for_update_with_version(CURRENT_VERSION)
}

/// Start an async update check
///
/// Returns a receiver that will receive the result when the check is complete.
pub fn check_for_update_async() -> mpsc::Receiver<UpdateCheckResult> {
    check_for_update_async_with_version(None)
}

/// Start an async update check with optional fake version for testing
///
/// If fake_version is provided, it will be used instead of CURRENT_VERSION.
pub fn check_for_update_async_with_version(
    fake_version: Option<String>,
) -> mpsc::Receiver<UpdateCheckResult> {
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let version = fake_version.as_deref().unwrap_or(CURRENT_VERSION);
        let result = check_for_update_with_version(version);
        let _ = tx.send(result);
    });
    rx
}

/// Get the target triple for the current platform
fn get_target() -> &'static str {
    // Map OS/ARCH to Rust target triple
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("macos", "aarch64") => "aarch64-apple-darwin",
        ("macos", "x86_64") => "x86_64-apple-darwin",
        ("linux", "aarch64") => "aarch64-unknown-linux-gnu",
        ("linux", "x86_64") => "x86_64-unknown-linux-gnu",
        ("windows", "x86_64") => "x86_64-pc-windows-msvc",
        ("windows", "aarch64") => "aarch64-pc-windows-msvc",
        _ => "unknown",
    }
}

/// Perform the actual update (blocking)
///
/// This downloads and replaces the current binary.
/// The application should be restarted after this completes.
pub fn perform_update_sync() -> UpdateResult {
    log_update("=== perform_update_sync() STARTED ===");

    let target = get_target();
    let os = std::env::consts::OS;
    let arch = std::env::consts::ARCH;

    // Build detailed context for error messages (always available, not just debug)
    let context = format!(
        "Version: {} | Target: {} | Platform: {}-{}",
        CURRENT_VERSION, target, os, arch
    );

    log_update(&format!("Context: {}", context));
    log_update(&format!("Repo: {}/{}", REPO_OWNER, REPO_NAME));
    log_update(&format!("Binary name: {}", BIN_NAME));

    log_update("Creating Update configuration...");

    match Update::configure()
        .repo_owner(REPO_OWNER)
        .repo_name(REPO_NAME)
        .bin_name(BIN_NAME)
        .target(target)
        .current_version(CURRENT_VERSION)
        .show_download_progress(false) // We show our own UI
        .show_output(false) // Suppress all output messages (TUI handles display)
        .no_confirm(true) // Skip Y/n prompt (TUI already confirmed)
        .build()
    {
        Ok(updater) => {
            log_update("Update configuration OK, calling updater.update()...");
            match updater.update() {
                Ok(status) => {
                    log_update(&format!(
                        "UPDATE SUCCESS: {} -> {}",
                        CURRENT_VERSION,
                        status.version()
                    ));
                    UpdateResult::Success {
                        old_version: CURRENT_VERSION.to_string(),
                        new_version: status.version().to_string(),
                    }
                }
                Err(e) => {
                    // Detailed error with context for troubleshooting
                    let error_msg = format!("{}\n\n[{}]", e, context);
                    log_update(&format!("UPDATE FAILED: {}", error_msg));
                    UpdateResult::Error(error_msg)
                }
            }
        }
        Err(e) => {
            // Configuration error with full context
            let error_msg = format!("Configuration failed: {}\n\n[{}]", e, context);
            log_update(&format!("CONFIG FAILED: {}", error_msg));
            UpdateResult::Error(error_msg)
        }
    }
}

/// Perform update to a specific version (for testing/downgrade)
///
/// This allows updating to any version, including older ones.
/// Useful for testing the update mechanism without releasing new versions.
pub fn perform_update_to_version_sync(target_version: &str) -> UpdateResult {
    log_update(&format!(
        "=== perform_update_to_version_sync({}) STARTED ===",
        target_version
    ));

    let target = get_target();
    let os = std::env::consts::OS;
    let arch = std::env::consts::ARCH;

    // Build detailed context for error messages
    let context = format!(
        "Current: {} | Target: {} | Platform target: {} | OS-Arch: {}-{}",
        CURRENT_VERSION, target_version, target, os, arch
    );

    log_update(&format!("Context: {}", context));

    // Normalize target version (ensure it has 'v' prefix for GitHub tags)
    let target_tag = if target_version.starts_with('v') {
        target_version.to_string()
    } else {
        format!("v{}", target_version)
    };

    log_update(&format!("Target tag: {}", target_tag));

    match Update::configure()
        .repo_owner(REPO_OWNER)
        .repo_name(REPO_NAME)
        .bin_name(BIN_NAME)
        .target(target)
        .target_version_tag(&target_tag) // Specific version instead of latest
        .current_version(CURRENT_VERSION)
        .show_download_progress(false)
        .show_output(false)
        .no_confirm(true)
        .build()
    {
        Ok(updater) => {
            log_update(&format!(
                "Update configuration OK, updating to {}...",
                target_tag
            ));
            match updater.update() {
                Ok(status) => {
                    log_update(&format!(
                        "UPDATE TO VERSION SUCCESS: {} -> {}",
                        CURRENT_VERSION,
                        status.version()
                    ));
                    UpdateResult::Success {
                        old_version: CURRENT_VERSION.to_string(),
                        new_version: status.version().to_string(),
                    }
                }
                Err(e) => {
                    let error_msg = format!("{}\n\n[{}]", e, context);
                    log_update(&format!("UPDATE TO VERSION FAILED: {}", error_msg));
                    UpdateResult::Error(error_msg)
                }
            }
        }
        Err(e) => {
            let error_msg = format!("Configuration failed: {}\n\n[{}]", e, context);
            log_update(&format!("CONFIG FAILED: {}", error_msg));
            UpdateResult::Error(error_msg)
        }
    }
}

/// Start an async update
///
/// Returns a receiver that will receive the result when the update is complete.
pub fn perform_update_async() -> mpsc::Receiver<UpdateResult> {
    log_update("=== perform_update_async() CALLED ===");
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        log_update("Update thread STARTED");
        let result = perform_update_sync();
        log_update(&format!("Update result: {:?}", result));
        match tx.send(result) {
            Ok(_) => log_update("Result SENT through channel"),
            Err(e) => log_update(&format!("FAILED to send result: {}", e)),
        }
        log_update("Update thread FINISHED");
    });
    log_update("perform_update_async() returning receiver");
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
        state.set_available("1.0.0".to_string(), Some("Release notes".to_string()));
        assert!(!state.checking);
        assert_eq!(state.available_version, Some("1.0.0".to_string()));
        assert_eq!(state.release_notes, Some("Release notes".to_string()));
        assert!(state.show_dialog);

        // Close dialog
        state.close_dialog();
        assert!(!state.show_dialog);
    }

    #[test]
    fn test_version_newer_basic() {
        // Basic version comparisons
        assert!(version_newer("0.37.2", "0.37.1"));
        assert!(version_newer("0.38.0", "0.37.2"));
        assert!(version_newer("1.0.0", "0.99.99"));
        assert!(!version_newer("0.37.1", "0.37.2"));
        assert!(!version_newer("0.37.2", "0.37.2"));
    }

    #[test]
    fn test_version_newer_with_v_prefix() {
        // Versions with 'v' prefix should be handled correctly
        assert!(version_newer("v0.37.2", "0.37.1"));
        assert!(version_newer("0.37.2", "v0.37.1"));
        assert!(version_newer("v0.37.2", "v0.37.1"));
        assert!(!version_newer("v0.37.1", "v0.37.2"));
        assert!(!version_newer("v0.37.2", "v0.37.2"));
    }

    #[test]
    fn test_version_newer_edge_cases() {
        // Edge cases
        assert!(version_newer("1.0.0", "0.0.0"));
        assert!(version_newer("0.1.0", "0.0.0"));
        assert!(version_newer("0.0.1", "0.0.0"));
        assert!(!version_newer("0.0.0", "0.0.0"));
        // Two-part versions
        assert!(version_newer("0.38", "0.37"));
        // Development versions (current newer than release)
        assert!(!version_newer("0.37.0", "0.37.2"));
    }

    #[test]
    fn test_release_notes_scroll() {
        let mut state = UpdateState::new();
        state.set_available(
            "1.0.0".to_string(),
            Some("Line 1\nLine 2\nLine 3".to_string()),
        );

        assert_eq!(state.release_notes_scroll, 0);

        // Scroll down
        state.scroll_release_notes_down(10);
        assert_eq!(state.release_notes_scroll, 1);

        state.scroll_release_notes_down(10);
        assert_eq!(state.release_notes_scroll, 2);

        // Scroll up
        state.scroll_release_notes_up();
        assert_eq!(state.release_notes_scroll, 1);

        // Can't scroll past 0
        state.scroll_release_notes_up();
        state.scroll_release_notes_up();
        assert_eq!(state.release_notes_scroll, 0);

        // Can't scroll past max
        for _ in 0..20 {
            state.scroll_release_notes_down(5);
        }
        assert_eq!(state.release_notes_scroll, 5);
    }

    #[test]
    fn test_platform_identification() {
        // Verify platform detection works
        let os = std::env::consts::OS;
        let arch = std::env::consts::ARCH;

        assert!(!os.is_empty(), "OS should be detected");
        assert!(!arch.is_empty(), "Architecture should be detected");

        // Common expected values
        let valid_os = ["linux", "macos", "darwin", "windows"];

        // At least one should match (platform-dependent)
        assert!(
            valid_os.iter().any(|&v| os.to_lowercase().contains(v))
                || os == "macos"
                || os == "darwin",
            "Unknown OS: {}",
            os
        );
    }

    #[test]
    fn test_filter_release_notes_no_table() {
        // Release notes without a downloads table should pass through unchanged
        let notes = "## What's New\n\n- Feature 1\n- Feature 2\n\n## Bug Fixes\n\n- Fix 1";
        let filtered = filter_release_notes_for_platform(notes);
        assert_eq!(filtered, notes);
    }

    #[test]
    fn test_filter_release_notes_preserves_non_table_content() {
        // Non-table content should be preserved
        let notes = r#"## What's New

- Enhanced update check with CLI mode
- Better release notes display

### Downloads

| Platform | Download |
|---|---|
| Apple Silicon | [link1] |
| macOS Intel | [link2] |
| Linux x64 | [link3] |
| Windows x64 | [link4] |

### Installation

Run `./install.sh` to install."#;

        let filtered = filter_release_notes_for_platform(notes);

        // Should always contain the intro and installation sections
        assert!(filtered.contains("## What's New"));
        assert!(filtered.contains("Enhanced update check"));
        assert!(filtered.contains("### Installation"));
        assert!(filtered.contains("Run `./install.sh`"));

        // Should contain Downloads header
        assert!(filtered.contains("### Downloads"));
    }

    #[test]
    fn test_filter_release_notes_filters_table_rows() {
        let notes = r#"### Downloads

| Platform | Download |
|---|---|
| Apple Silicon | [arm64.tar.gz] |
| macOS Intel | [amd64.tar.gz] |
| Linux x64 | [linux-x64.tar.gz] |

Done."#;

        let filtered = filter_release_notes_for_platform(notes);

        // On any platform, we should see "Done." at the end
        assert!(filtered.contains("Done."));

        // The table header row should be present
        assert!(filtered.contains("### Downloads"));
    }

    #[test]
    fn test_filter_release_notes_empty_input() {
        let filtered = filter_release_notes_for_platform("");
        assert_eq!(filtered, "");
    }

    // ==========================================================================
    // Integration tests - require network access, run manually with:
    // cargo test --lib update -- --ignored --nocapture
    // ==========================================================================

    #[test]
    #[ignore]
    fn test_github_release_accessible() {
        eprintln!("Testing GitHub API accessibility...");
        eprintln!("Current version: {}", CURRENT_VERSION);

        let result = check_for_update_with_version(CURRENT_VERSION);

        match result {
            UpdateCheckResult::Error(e) => {
                panic!("GitHub API error: {}", e);
            }
            UpdateCheckResult::NoReleasesFound => {
                panic!(
                    "No releases found for platform: {}-{}",
                    std::env::consts::ARCH,
                    std::env::consts::OS
                );
            }
            UpdateCheckResult::UpToDate => {
                eprintln!("✅ GitHub API accessible - current version is up-to-date");
            }
            UpdateCheckResult::UpdateAvailable {
                version,
                release_notes,
            } => {
                eprintln!("✅ GitHub API accessible - update available: {}", version);
                if release_notes.is_some() {
                    eprintln!("   Release notes available");
                }
            }
        }
    }

    #[test]
    #[ignore]
    fn test_release_notes_fetchable() {
        eprintln!("Testing release notes fetching...");

        // Get the latest version from GitHub
        let result = check_for_update_with_version("0.0.0");

        let version_to_test = match result {
            UpdateCheckResult::UpdateAvailable { version, .. } => version,
            UpdateCheckResult::UpToDate => CURRENT_VERSION.to_string(),
            UpdateCheckResult::NoReleasesFound => {
                eprintln!("⚠️  No releases found, skipping test");
                return;
            }
            UpdateCheckResult::Error(e) => {
                panic!("Cannot test: {}", e);
            }
        };

        eprintln!("Fetching release notes for version: {}", version_to_test);

        if let Some(body) = fetch_release_notes(&version_to_test) {
            eprintln!("✅ Release notes fetched successfully");
            eprintln!("───────────────────────────────────");
            for line in body.lines().take(10) {
                eprintln!("  {}", line);
            }
            if body.lines().count() > 10 {
                eprintln!("  ... ({} more lines)", body.lines().count() - 10);
            }
            eprintln!("───────────────────────────────────");
        } else {
            eprintln!("⚠️  No release notes body for version {}", version_to_test);
        }
    }

    #[test]
    #[ignore]
    fn test_update_check_with_fake_version() {
        eprintln!("Testing update check with simulated older version...");

        let fake_version = "0.1.0";
        eprintln!("Fake version: {}", fake_version);
        eprintln!("Real version: {}", CURRENT_VERSION);

        let result = check_for_update_with_version(fake_version);

        match result {
            UpdateCheckResult::UpdateAvailable {
                version,
                release_notes,
            } => {
                eprintln!("✅ Update found: {} -> {}", fake_version, version);
                assert!(
                    version_newer(&version, fake_version),
                    "New version should be newer than fake version"
                );

                if let Some(notes) = release_notes {
                    eprintln!("   Release notes: {} chars", notes.len());
                }
            }
            UpdateCheckResult::UpToDate => {
                panic!("Expected update available for version {}", fake_version);
            }
            UpdateCheckResult::NoReleasesFound => {
                panic!("No releases found");
            }
            UpdateCheckResult::Error(e) => {
                panic!("Error: {}", e);
            }
        }
    }
}
