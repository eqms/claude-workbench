---
phase: 01-security-hardening
plan: "02"
subsystem: update
tags: [security, cfg-gate, release-hardening, restart, tdd]
dependency_graph:
  requires: []
  provides: [sec/update-downgrade-gate, sec/restart-arg-filter]
  affects: [src/main.rs, src/update/install.rs, tests/cli.rs]
tech_stack:
  added: []
  patterns: [cfg(debug_assertions) field gating, extract-then-test helper pattern]
key_files:
  created:
    - tests/cli.rs (update_to_flag_not_present_in_release_build test appended)
  modified:
    - src/main.rs
    - src/update/install.rs
decisions:
  - "Gate --update-to at field level in Args struct (not at call site) so clap's parser never sees the flag in release mode"
  - "Skip --update-to value arg in filter_restart_args via skip_next sentinel to avoid re-forwarding version string"
  - "Add mixed-case test to cover real-world restart scenario with config + update-to combo"
metrics:
  duration: "~12 minutes"
  completed: "2026-05-11"
  tasks_completed: 2
  tasks_total: 2
  files_changed: 3
---

# Phase 01 Plan 02: Gate --update-to to Debug Builds + Strip One-Shot Restart Args Summary

Gate `--update-to` behind `#[cfg(debug_assertions)]` (CR-02) and strip one-shot flags in `restart_application()` (IN-02 backport): release binaries cannot be downgraded via CLI flag; restarted processes cannot loop on one-shot operations.

## Tasks Completed

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | Gate --update-to behind #[cfg(debug_assertions)] | 93a6d56 | src/main.rs, tests/cli.rs |
| 2 | Filter one-shot flags in restart_application() | eb6b6cb | src/update/install.rs |

## What Was Built

### Task 1 — CR-02: --update-to debug-only gate

- Added `#[cfg(debug_assertions)]` to the `update_to: Option<String>` field in `Args` struct in `src/main.rs`
- Wrapped the `args.update_to` usage site in `#[cfg(debug_assertions)]` block
- Release binary: `--update-to` is unknown to clap, exits with code 2 and "unexpected argument"
- Debug binary: `--update-to` still recognized for development/testing
- Integration test `update_to_flag_not_present_in_release_build` added to `tests/cli.rs` (gated `#[cfg(not(debug_assertions))]`, runs under `cargo test --release`)

### Task 2 — IN-02: One-shot flag filtering in restart_application()

- Extracted `filter_restart_args()` helper in `src/update/install.rs`
- Strips: `--update-to` (+ its value), `--check-update`, `--clipboard-diag`, `--ssh-paste-diag`
- `restart_application()` now calls `filter_restart_args(std::env::args().skip(1))`
- 3 unit tests: removes all one-shot flags, keeps safe flags (`--config path`), mixed scenario

## Verification Results

```
# Release build: --update-to unknown
./target/release/claude-workbench --update-to 0.1.0
error: unexpected argument '--update-to' found
Exit: 2  ✓

# Debug build: --update-to recognized
./target/debug/claude-workbench --update-to
error: a value is required for '--update-to <UPDATE_TO>' but none was supplied  ✓

# Integration test (release)
cargo test --release -- update_to_flag_not_present
test update_to_flag_not_present_in_release_build ... ok  ✓

# Unit tests
cargo test test_filter_restart
test_filter_restart_args_mixed ... ok
test_filter_restart_args_keeps_safe_flags ... ok
test_filter_restart_args_removes_one_shot_flags ... ok
3 passed  ✓
```

## Threat Mitigations Applied

| Threat ID | Category | Mitigation |
|-----------|----------|------------|
| T-02-01 | Elevation of Privilege | `#[cfg(debug_assertions)]` on `update_to` field — flag absent from release clap struct |
| T-02-02 | Elevation of Privilege | `filter_restart_args()` strips all one-shot flags before re-exec |

## Deviations from Plan

**1. [Rule 2 - Enhancement] Added mixed-case unit test**
- Found during: Task 2
- Issue: Plan specified 2 tests; real-world restart scenario (config flag + update-to combo) warranted a third
- Fix: Added `test_filter_restart_args_mixed` covering `--config path --check-update --update-to version` → `["--config", "path"]`
- Files modified: src/update/install.rs
- Commit: eb6b6cb

**2. [Rule 2 - Correctness] --update-to value arg skipped in filter_restart_args**
- Found during: Task 2 implementation
- Issue: Plan's simple `.filter(|a| !matches!(a.as_str(), ...))` would leave "0.1.0" (the version value) in the args list; a future release binary could receive an unknown positional arg
- Fix: Used `skip_next` sentinel to consume the value following `--update-to`
- Files modified: src/update/install.rs
- Commit: eb6b6cb

## Known Stubs

None.

## Threat Flags

None — no new network endpoints, auth paths, or file access patterns introduced.

## Self-Check: PASSED

- src/main.rs modified: update_to field gated with #[cfg(debug_assertions)] ✓
- src/update/install.rs modified: filter_restart_args() helper present ✓
- tests/cli.rs modified: update_to_flag_not_present_in_release_build test present ✓
- Commit 93a6d56 exists ✓
- Commit eb6b6cb exists ✓
- Release binary exits 2 for --update-to ✓
- Debug binary recognizes --update-to ✓
- All unit tests pass ✓
- Integration test passes under cargo test --release ✓
