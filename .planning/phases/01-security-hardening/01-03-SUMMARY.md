---
phase: 01-security-hardening
plan: 03
subsystem: security
tags: [rust, shlex, command-injection, allow-list, shell-fallback, browser, dependency-checker]

# Dependency graph
requires: []
provides:
  - "validate_program() allow-list guard in opener.rs rejecting shell metacharacters"
  - "shlex::split replacing hand-rolled split_command for correct single-quote handling"
  - "Shell interactive fallback ($SHELL -i -c) removed from dependency_checker.rs"
affects: [browser, setup, dependency-detection]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "validate_program pattern: allow-list via chars().all() before Command::new"
    - "shlex::split for shell-quoted config strings instead of hand-rolled parsers"

key-files:
  created: []
  modified:
    - src/browser/opener.rs
    - src/setup/dependency_checker.rs

key-decisions:
  - "validate_program uses an explicit allow-list (alphanumeric + _-./+) rather than a deny-list — safer by default"
  - "shlex::split replaces hand-rolled split_command to correctly handle single-quoted args like 'open -a Brave Browser'"
  - "Shell fallback removed entirely; all probed binaries are real executables on PATH (Assumption A3 confirmed)"
  - "validate_program is private (not pub) — called only within opener.rs before Command::new"

patterns-established:
  - "allow-list guard: call validate_program(program)? before any Command::new with user-supplied config"
  - "shlex::split: use for config strings that may contain quoted arguments"

requirements-completed:
  - SEC-02
  - SEC-03

# Metrics
duration: 18min
completed: 2026-05-11
---

# Phase 01 Plan 03: Browser/Editor Command Injection Hardening Summary

**Shell metacharacter allow-list guard and shlex-based argument splitting added to opener.rs; interactive-shell fallback ($SHELL -i -c) removed from dependency_checker.rs**

## Performance

- **Duration:** 18 min
- **Started:** 2026-05-11T00:00:00Z
- **Completed:** 2026-05-11T00:18:00Z
- **Tasks:** 2
- **Files modified:** 2

## Accomplishments
- `validate_program()` added to `opener.rs`: rejects any browser/editor config containing shell metacharacters (`;`, `|`, `&`, `$`, backtick, space, etc.) before `Command::new` executes
- `shlex::split` replaces hand-rolled `split_command` in both `open_file_with_browser` and `open_file_with_editor` — fixes single-quoted argument handling (e.g. `open -a 'Brave Browser'`)
- Interactive-shell fallback (`$SHELL -i -c`) removed from `dependency_checker.rs::check_command`; direct `Command::new` is now the only execution path
- 5 new unit tests: 2 in opener.rs (12 sub-assertions total), 2 new in dependency_checker.rs (plus 1 pre-existing)

## Task Commits

Each task was committed atomically:

1. **Task 1: validate_program + shlex::split in opener.rs** - `4b6046d` ([FIX])
2. **Task 2: Remove $SHELL -i -c fallback in dependency_checker.rs** - `6fe0862` ([FIX])

## Files Created/Modified
- `src/browser/opener.rs` - Added `validate_program()`, replaced `split_command` with `shlex::split` in both open functions, added test module with 12 assertions
- `src/setup/dependency_checker.rs` - Removed 35-line `$SHELL -i -c` block, added 2 new unit tests

## Decisions Made
- `validate_program` is a private allow-list function, not pub — no caller outside `opener.rs` needs it
- Allow-list characters: `[A-Za-z0-9_\-./+]` — includes `+` for `g++`-style programs and `/` for absolute paths
- Shell fallback removal is total (no Windows cfg guard retained) since direct `Command::new` already handles all platforms correctly
- The pre-existing `#[cfg(windows)]` shell-fallback stub was also removed as dead code

## Deviations from Plan
None - plan executed exactly as written.

## Issues Encountered
- clippy `-D warnings` reports pre-existing warnings in other files (unrelated `if-in-match` patterns). None are in the two files modified by this plan. Verified via `grep "opener\|dependency_checker"` on clippy output returning empty.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- SEC-02 and SEC-03 mitigations complete
- `validate_program` pattern established for any future config-to-exec paths
- Dependency checker now startup-safe on macOS with Fish shell (no interactive-shell side effects)

---
*Phase: 01-security-hardening*
*Completed: 2026-05-11*
