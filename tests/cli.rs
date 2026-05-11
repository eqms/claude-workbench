//! CLI integration tests for the claude-workbench binary.
//!
//! These tests exercise the binary's non-interactive entry points
//! (`--help`, `--version`) to verify that argument parsing and basic
//! startup wiring remain intact.

use std::process::Command;

fn workbench_binary() -> &'static str {
    env!("CARGO_BIN_EXE_claude-workbench")
}

#[test]
fn help_prints_usage_and_exits_zero() {
    let output = Command::new(workbench_binary())
        .arg("--help")
        .output()
        .expect("failed to invoke claude-workbench --help");

    assert!(
        output.status.success(),
        "--help should exit 0, got {:?}",
        output.status
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Usage"),
        "--help output should contain 'Usage'\nstdout: {}",
        stdout
    );
    assert!(
        stdout.contains("--check-update") || stdout.contains("check-update"),
        "--help output should mention --check-update flag"
    );
}

#[test]
fn version_prints_semver_and_exits_zero() {
    let output = Command::new(workbench_binary())
        .arg("--version")
        .output()
        .expect("failed to invoke claude-workbench --version");

    assert!(
        output.status.success(),
        "--version should exit 0, got {:?}",
        output.status
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let pkg_version = env!("CARGO_PKG_VERSION");
    assert!(
        stdout.contains(pkg_version),
        "--version output should contain {}, got: {}",
        pkg_version,
        stdout
    );

    let parts: Vec<&str> = pkg_version.split('.').collect();
    assert!(
        parts.len() >= 2,
        "Cargo.toml version should be at least major.minor"
    );
}

#[test]
fn unknown_flag_is_rejected() {
    let output = Command::new(workbench_binary())
        .arg("--definitely-not-a-real-flag")
        .output()
        .expect("failed to invoke binary with bogus flag");

    assert!(
        !output.status.success(),
        "unknown flag should fail, got status {:?}",
        output.status
    );
}

/// Verify that `--update-to` is not a recognized flag in release builds.
///
/// The flag is gated with `#[cfg(debug_assertions)]` so it is absent from
/// the clap argument set in release mode. Clap exits 2 for unknown arguments.
///
/// Only runs under `cargo test --release` (not(debug_assertions)).
#[test]
#[cfg(not(debug_assertions))]
fn update_to_flag_not_present_in_release_build() {
    let output = Command::new(workbench_binary())
        .args(["--update-to", "0.1.0"])
        .output()
        .expect("failed to invoke binary");
    // clap exits 2 for unknown arguments
    assert_eq!(
        output.status.code(),
        Some(2),
        "--update-to should be unknown in release builds"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("unexpected argument") || stderr.contains("unrecognized"),
        "stderr: {}",
        stderr
    );
}
