//! Self-update functionality for claude-workbench
//!
//! Uses GitHub Releases as backend via the `self_update` crate.
//! Provides non-blocking update checks and update execution.

mod check;
mod install;
mod log;
mod release_notes;
mod state;
mod version;

/// GitHub repository configuration (shared across submodules).
pub(crate) const REPO_OWNER: &str = "eqms";
pub(crate) const REPO_NAME: &str = "claude-workbench";
pub(crate) const BIN_NAME: &str = "claude-workbench";

pub use check::{
    check_for_update_async, check_for_update_async_with_version, check_for_update_sync,
    check_for_update_with_version,
};
pub use install::{
    perform_update_async, perform_update_sync, perform_update_to_version_sync, restart_application,
};
pub use log::{log_file_path, log_update};
pub use release_notes::{fetch_release_notes, filter_release_notes_for_platform};
pub use state::{UpdateCheckResult, UpdateResult, UpdateState};
pub use version::{current_version, version_newer, CURRENT_VERSION};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_current_version() {
        let version = current_version();
        assert!(!version.is_empty());
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

        state.start_check();
        assert!(state.checking);

        state.set_available("1.0.0".to_string(), Some("Release notes".to_string()));
        assert!(!state.checking);
        assert_eq!(state.available_version, Some("1.0.0".to_string()));
        assert_eq!(state.release_notes, Some("Release notes".to_string()));
        assert!(state.show_dialog);

        state.close_dialog();
        assert!(!state.show_dialog);
    }

    #[test]
    fn test_version_newer_basic() {
        assert!(version_newer("0.37.2", "0.37.1"));
        assert!(version_newer("0.38.0", "0.37.2"));
        assert!(version_newer("1.0.0", "0.99.99"));
        assert!(!version_newer("0.37.1", "0.37.2"));
        assert!(!version_newer("0.37.2", "0.37.2"));
    }

    #[test]
    fn test_version_newer_with_v_prefix() {
        assert!(version_newer("v0.37.2", "0.37.1"));
        assert!(version_newer("0.37.2", "v0.37.1"));
        assert!(version_newer("v0.37.2", "v0.37.1"));
        assert!(!version_newer("v0.37.1", "v0.37.2"));
        assert!(!version_newer("v0.37.2", "v0.37.2"));
    }

    #[test]
    fn test_version_newer_edge_cases() {
        assert!(version_newer("1.0.0", "0.0.0"));
        assert!(version_newer("0.1.0", "0.0.0"));
        assert!(version_newer("0.0.1", "0.0.0"));
        assert!(!version_newer("0.0.0", "0.0.0"));
        assert!(version_newer("0.38", "0.37"));
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

        state.scroll_release_notes_down(10);
        assert_eq!(state.release_notes_scroll, 1);

        state.scroll_release_notes_down(10);
        assert_eq!(state.release_notes_scroll, 2);

        state.scroll_release_notes_up();
        assert_eq!(state.release_notes_scroll, 1);

        state.scroll_release_notes_up();
        state.scroll_release_notes_up();
        assert_eq!(state.release_notes_scroll, 0);

        for _ in 0..20 {
            state.scroll_release_notes_down(5);
        }
        assert_eq!(state.release_notes_scroll, 5);
    }

    #[test]
    fn test_platform_identification() {
        let os = std::env::consts::OS;
        let arch = std::env::consts::ARCH;

        assert!(!os.is_empty(), "OS should be detected");
        assert!(!arch.is_empty(), "Architecture should be detected");

        let valid_os = ["linux", "macos", "darwin", "windows"];

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
        let notes = "## What's New\n\n- Feature 1\n- Feature 2\n\n## Bug Fixes\n\n- Fix 1";
        let filtered = filter_release_notes_for_platform(notes);
        assert_eq!(filtered, notes);
    }

    #[test]
    fn test_filter_release_notes_preserves_non_table_content() {
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

        assert!(filtered.contains("## What's New"));
        assert!(filtered.contains("Enhanced update check"));
        assert!(filtered.contains("### Installation"));
        assert!(filtered.contains("Run `./install.sh`"));
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

        assert!(filtered.contains("Done."));
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
