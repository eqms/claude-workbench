---
phase: quick
plan: 260611-g0k
subsystem: security
tags: [security, pty, clipboard, deps, logging]
dependency_graph:
  requires: []
  provides: [SEC-02-pty-unquoted-path, SEC-03-cr-injection, SEC-04-dep-pin, SEC-05-rustsec-floor, SEC-06-log-tempdir]
  affects: [src/app/pty.rs, src/app/clipboard.rs, src/update/log.rs, Cargo.toml]
tech_stack:
  added: []
  patterns: [shlex::try_quote early-return, CR stripping before PTY write, rev-pinned git dep, temp_dir() portability]
key_files:
  created: []
  modified:
    - src/app/pty.rs
    - src/app/clipboard.rs
    - src/update/log.rs
    - Cargo.toml
    - Cargo.lock
decisions:
  - "Dropped branch= from tui-textarea dep when adding rev= (Cargo rejects both simultaneously); branch preserved in comment"
metrics:
  duration: ~15 min
  completed: 2026-06-11
---

# Phase quick Plan 260611-g0k: Security Audit Fixes Summary

Five targeted hardening changes from the /project-audit: PTY path-quote early return, CR-injection filter, tui-textarea rev pin, tokio RUSTSEC floor, and portable temp_dir() log fallback.

## Tasks Completed

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | Harden PTY path insert + CR filter in clipboard | 6b9875a | src/app/pty.rs, src/app/clipboard.rs |
| 2 | Cargo.toml — pin tui-textarea rev + raise tokio floor + version bump | 6b9875a | Cargo.toml, Cargo.lock |
| 3 | Fix update log /tmp fallback + final verification + commit | 6b9875a | src/update/log.rs |

All three tasks committed atomically in a single [FIX] commit per the plan instruction.

## What Was Built

**src/app/pty.rs — insert_path_at_cursor (SEC-02)**
Replaced the `unwrap_or_else(|_| path_str.into_owned())` fallback with a `match` that returns early on `shlex::try_quote` failure. A NUL-byte path is now silently rejected without writing any bytes to the PTY. This closes the shell metacharacter injection vector.

**src/app/clipboard.rs — copy_selection_to_claude (SEC-03)**
Added a `.replace('\r', "")` normalisation pass over `formatted_lines` before the code-block format string is assembled and written to the Claude PTY. Prevents CRLF line endings from PTY screen rows injecting a premature Enter command.

**Cargo.toml — tui-textarea rev pin (SEC-04)**
Replaced `branch = "update-ratatui"` with `rev = "b6bf812d1f5edab4f311f56d405a47341e9423cf"`. Note: Cargo rejects `branch` and `rev` simultaneously in the same dep entry, so `branch` was removed and preserved as a comment. Cargo.lock still resolves to the same `b6bf812d` commit — confirmed no drift.

**Cargo.toml — tokio floor (SEC-05)**
Raised `version = "1.44.0"` to `"1.44.2"` for RUSTSEC-2025-0023 hygiene. Lock already resolves 1.49.0; this is a floor-only change.

**Cargo.toml — version bump**
`0.92.0` → `0.93.0` per [FIX] release convention.

**src/update/log.rs — log_file_path() (SEC-06)**
Replaced `std::path::PathBuf::from("/tmp")` fallback with `std::env::temp_dir`. Portable across macOS ($TMPDIR), Linux, and Windows (GetTempPath).

## Verification

All gates passed:
- `cargo build`: clean (0 errors)
- `cargo test`: 149 tests pass (146 unit + 3 CLI)
- `cargo clippy --all-features -- -D warnings`: 3 pre-existing errors in src/app/file_ops.rs (collapsible_match, out of scope per STATE.md); no new errors from modified files
- `cargo fmt --check`: clean

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Cargo rejects branch + rev simultaneously**
- **Found during:** Task 2
- **Issue:** Adding `rev = "b6bf812d..."` alongside `branch = "update-ratatui"` causes a Cargo manifest parse error: "dependency specification is ambiguous. Only one of `branch`, `tag` or `rev` is allowed."
- **Fix:** Removed `branch = "update-ratatui"` and added it as a comment. The resolved commit in Cargo.lock is identical (`b6bf812d1f5edab4f311f56d405a47341e9423cf`), so supply-chain pinning goal is achieved.
- **Files modified:** Cargo.toml
- **Impact:** None — rev pin is stricter than branch pin; the branch context is preserved in-line comment.

## Known Stubs

None.

## Threat Flags

None — no new network endpoints, auth paths, file access patterns, or schema changes introduced.

## Self-Check: PASSED

- src/app/pty.rs: exists, contains `match shlex::try_quote` early-return pattern
- src/app/clipboard.rs: exists, contains `.replace('\r', "")` normalisation
- src/update/log.rs: exists, uses `std::env::temp_dir` (no /tmp literal)
- Cargo.toml: version 0.93.0, rev pin present, tokio 1.44.2
- Cargo.lock: tui-textarea resolves to b6bf812d
- Commit 6b9875a: confirmed at HEAD
