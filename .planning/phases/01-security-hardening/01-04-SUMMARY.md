---
phase: 01-security-hardening
plan: 04
subsystem: security
tags: [rust, clipboard, pty, shlex, semver, update-check, executable-bit]

requires: []
provides:
  - "is_executable() helper in clipboard.rs; which() rejects non-executable PATH entries on Unix"
  - "quote_path_for_cd() helper in pty.rs; sync_terminals* never injects unescaped paths into PTY"
  - "semver max_by release selection in check.rs; update check immune to API creation order"
affects: [02-signature-verification, any plan touching clipboard, pty, or update subsystems]

tech-stack:
  added: ["semver = '1' (direct dep, was transitive only)"]
  patterns:
    - "cfg(unix) guard for platform-specific security checks (is_executable)"
    - "Extract testable helper before impl pattern (quote_path_for_cd, is_executable)"
    - "log_update() for silent-failure audit trail in pty sync paths"
    - "semver::Version::parse + max_by for API-order-immune release selection"

key-files:
  created: []
  modified:
    - src/clipboard.rs
    - src/app/pty.rs
    - src/update/check.rs
    - Cargo.toml

key-decisions:
  - "Leave insert_path_at_cursor unwrap_or_else intact — different semantics (path insertion not shell command), pre-existing comment explains why fallback is safe there; only sync_terminals* are in-scope for WR-04"
  - "semver added as explicit direct dependency (was transitive via self_update) for clean API use"
  - "is_executable() gated behind #[cfg(unix)] — Windows has no executable bit concept, is_file() alone remains correct there"
  - "NoReleasesFound return path handles both empty releases and all-unparseable-tags cases via Option::None from max_by"

patterns-established:
  - "Unix security checks: use cfg(unix) + PermissionsExt::mode() & 0o111 pattern"
  - "PTY path quoting: always use quote_path_for_cd(); never fall back to unescaped on Err"
  - "Update checks: always select by semver max, never trust API list order"

requirements-completed: [SEC-01]

duration: 18min
completed: 2026-05-11
---

# Phase 01 Plan 04: WR-03/WR-04/WR-05 Security Hardening Summary

**Executable-bit guard in `which()`, shlex error propagation in `sync_terminals*`, and semver-ordered release selection replacing `releases[0]`**

## Performance

- **Duration:** ~18 min
- **Started:** 2026-05-11T12:55:00Z
- **Completed:** 2026-05-11T13:13:00Z
- **Tasks:** 2 (Task 1: WR-03; Task 2: WR-04 + WR-05)
- **Files modified:** 4

## Accomplishments

- `is_executable()` helper (Unix-gated, `PermissionsExt::mode() & 0o111`) added; `which()` now skips non-executable files on PATH, preventing silent "Permission denied" at subprocess spawn
- All 3 `sync_terminals*` call sites in `pty.rs` now use `match quote_path_for_cd()` + `log_update` on the `None` arm — unescaped path bytes can no longer reach the PTY shell
- `check.rs` release selection uses `semver::Version` `max_by` chain; backdated old-branch patch releases cannot suppress legitimate updates

## Task Commits

1. **Task 1: WR-03 executable-bit check** - `999fc2c` (fix)
2. **Task 2: WR-04 shlex propagation + WR-05 semver selection** - `3cb3668` (fix)

## Files Created/Modified

- `src/clipboard.rs` — Added `is_executable()` helper + updated `which()` with `#[cfg(unix)]` exec-bit gate; added `test_is_executable_respects_mode`
- `src/app/pty.rs` — Added `quote_path_for_cd()` helper + `use crate::update::log_update`; replaced 3 `unwrap_or_else` sites with `match`; added 2 tests
- `src/update/check.rs` — Added `use semver::Version`; replaced `releases[0]` with `max_by(semver)` chain + `let Some` guard; added 2 tests
- `Cargo.toml` — Added `semver = "1"` as direct dependency

## Decisions Made

- Kept `insert_path_at_cursor` `unwrap_or_else` unchanged — it inserts text at PTY cursor (no shell command formed), and the existing comment correctly notes NUL bytes are impossible on real filesystems. Out of scope for WR-04.
- Added semver as explicit dep rather than relying on transitive resolution from `self_update`.
- `is_executable` is `#[cfg(unix)]` only — on Windows `which()` retains `is_file()` behavior, which is correct (no executable bit concept).

## Deviations from Plan

None — plan executed exactly as written. The 4th `try_quote` site (`insert_path_at_cursor`) was noted and consciously left in scope per plan boundary (3 sync_terminals* sites only).

## Issues Encountered

- Pre-existing clippy `-D warnings` failures in `file_ops.rs`, `keyboard/dialogs.rs`, `mouse.rs`, `browser/typst_pdf.rs` (collapsible if-in-match). Not caused by our changes; deferred per scope boundary rule. Zero clippy warnings/errors in the 3 modified files.

## Next Phase Readiness

- WR-03, WR-04, WR-05 complete and verified with unit tests (5 new tests)
- `cargo test` 119 passed, 0 failed
- Ready for phase 01-05 or next wave

---
*Phase: 01-security-hardening*
*Completed: 2026-05-11*
