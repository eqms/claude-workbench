//! Version parsing and comparison helpers.

/// Current version from Cargo.toml
pub const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Get the current version string
pub fn current_version() -> &'static str {
    CURRENT_VERSION
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
