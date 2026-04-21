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
