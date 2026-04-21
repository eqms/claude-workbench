//! Release notes fetching and platform-specific filtering.

use self_update::backends::github::ReleaseList;

use super::{REPO_NAME, REPO_OWNER};

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
    let current_os = std::env::consts::OS;
    let current_arch = std::env::consts::ARCH;

    let platform_label = match (current_os, current_arch) {
        ("macos", "aarch64") => "Apple Silicon",
        ("macos", "x86_64") => "macOS Intel",
        ("linux", "aarch64") => "Linux ARM64",
        ("linux", "x86_64") => "Linux x64",
        ("windows", "x86_64") => "Windows x64",
        ("windows", "aarch64") => "Windows ARM64",
        _ => return notes.to_string(),
    };

    let mut result = Vec::new();
    let mut in_table = false;

    for line in notes.lines() {
        if line.starts_with('|') && !in_table {
            in_table = true;
            result.push(line);
            continue;
        }

        if in_table && line.starts_with('|') && line.contains("---") {
            result.push(line);
            continue;
        }

        if in_table && line.starts_with('|') {
            if line.contains(platform_label) {
                result.push(line);
            }
            continue;
        }

        if in_table && !line.starts_with('|') {
            in_table = false;
        }

        result.push(line);
    }

    result.join("\n")
}
