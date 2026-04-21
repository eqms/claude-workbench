//! Update check logic (synchronous and async).

use std::sync::mpsc;
use std::thread;

use self_update::backends::github::ReleaseList;

use super::release_notes::filter_release_notes_for_platform;
use super::state::UpdateCheckResult;
use super::version::{version_newer, CURRENT_VERSION};
use super::{REPO_NAME, REPO_OWNER};

/// Check for updates synchronously with a specific version (for testing)
///
/// Allows overriding the current version for testing purposes.
pub fn check_for_update_with_version(current_version: &str) -> UpdateCheckResult {
    #[cfg(debug_assertions)]
    {
        eprintln!("[Update] Current version: {}", current_version);
        eprintln!("[Update] Checking GitHub: {}/{}", REPO_OWNER, REPO_NAME);
        eprintln!("[Update] Binary name: {}", super::BIN_NAME);
        eprintln!(
            "[Update] Platform: {}-{}",
            std::env::consts::OS,
            std::env::consts::ARCH
        );
    }

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

                let latest = &releases[0];
                let target_version = &latest.version;

                #[cfg(debug_assertions)]
                eprintln!("[Update] GitHub version: {}", target_version);

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
pub(super) fn get_target() -> &'static str {
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
