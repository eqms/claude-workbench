---
phase: 01-security-hardening
plan: 01
subsystem: security
tags: [tempfile, O_EXCL, symlink-attack, temp-file, browser-preview, pdf_export]

requires: []
provides:
  - "default_preview_file() in src/browser/pdf_export.rs returning Result<NamedTempFile> (O_EXCL)"
  - "App::temp_preview_files: Vec<tempfile::NamedTempFile> (auto-delete on drop)"
  - "markdown_to_html() returning Vec<NamedTempFile>"
  - "text_to_html() returning NamedTempFile"
affects:
  - "01-02 and subsequent plans using browser preview temp files"

tech-stack:
  added: []
  patterns:
    - "NamedTempFile-as-lifetime-guard: temp file handle kept in App struct prevents premature deletion while browser reads file"
    - "O_EXCL creation: all temp file creation goes through tempfile::Builder, never predictable paths"

key-files:
  created: []
  modified:
    - "src/browser/pdf_export.rs"
    - "src/browser/markdown.rs"
    - "src/browser/syntax.rs"
    - "src/app/mod.rs"
    - "src/app/file_ops.rs"
    - "src/app/drawing.rs"

key-decisions:
  - "Return NamedTempFile handle from all preview-generation functions so caller controls lifetime (drop = delete)"
  - "Remove manual cleanup_temp_files() — replaced by RAII via Vec<NamedTempFile> in App"
  - "Write HTML content into NamedTempFile after link rewriting (not before) in markdown_to_html, preserving correct link-map behavior"
  - "cargo fmt applied to pre-existing formatting issues in job_state.rs and dependency_checker.rs"

patterns-established:
  - "Preview temp files: always use tempfile::Builder::new().prefix().suffix().tempfile_in() — never construct paths manually"
  - "NamedTempFile lifetime: push handle into App::temp_preview_files BEFORE passing path to browser open"

requirements-completed:
  - SEC-04

duration: 35min
completed: 2026-05-11
---

# Phase 01 Plan 01: Secure Temp File Creation Summary

**Symlink-attack vector (CR-03/SEC-04) eliminated: all browser-preview temp files now created with O_EXCL via tempfile::Builder, replacing the predictable `/tmp/{project}-{stem}-{date}.html` pattern**

## Performance

- **Duration:** ~35 min
- **Started:** 2026-05-11T00:00:00Z
- **Completed:** 2026-05-11T00:35:00Z
- **Tasks:** 2
- **Files modified:** 8

## Accomplishments
- Replaced `default_preview_filename()` (returns predictable `PathBuf`) with `default_preview_file()` (returns `Result<NamedTempFile>` via O_EXCL open)
- Changed `markdown_to_html()` and `text_to_html()` to return `NamedTempFile` handles instead of `PathBuf`
- Changed `App::temp_preview_files` from `Vec<PathBuf>` to `Vec<tempfile::NamedTempFile>` — files auto-delete when App drops
- Removed manual `cleanup_temp_files()` and its call site — RAII replaces manual deletion loop
- Added 4 unit tests: `.html` suffix, stem-only prefix when project name empty, uniqueness across calls, auto-delete on drop

## Task Commits

1. **Task 1: Replace default_preview_filename with default_preview_file (TDD RED+GREEN)** - `4b84723` ([FIX])
2. **Task 2: Update callers — included in same commit** - `4b84723` ([FIX])

**Plan metadata:** (SUMMARY commit follows)

## Files Created/Modified
- `src/browser/pdf_export.rs` — replaced `default_preview_filename` with `default_preview_file`, added 4 unit tests
- `src/browser/markdown.rs` — `convert_single_md` returns `(String, NamedTempFile)`, `markdown_to_html` returns `Vec<NamedTempFile>`
- `src/browser/syntax.rs` — `text_to_html` returns `NamedTempFile`, writes via `tmp.write_all()`
- `src/app/mod.rs` — field `temp_preview_files: Vec<tempfile::NamedTempFile>`; removed `cleanup_temp_files()` call
- `src/app/file_ops.rs` — `open_in_browser` extracts `PathBuf` from `NamedTempFile.path()` before storing handle
- `src/app/drawing.rs` — removed `cleanup_temp_files()` method (replaced by RAII)
- `src/app/job_state.rs` — `cargo fmt` formatting fix (pre-existing)
- `src/setup/dependency_checker.rs` — `cargo fmt` formatting fix (pre-existing)

## Decisions Made
- Return `NamedTempFile` from all preview generators so callers control lifetime via RAII; path is extracted before storing handle
- Write HTML into `NamedTempFile` after inter-document link rewriting (not before), preserving correct `file://` URL generation in `link_map`
- `cargo fmt` cleaned pre-existing format drift in two unrelated files (required for `cargo fmt -- --check` to pass)

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] cargo fmt fixed pre-existing formatting in unrelated files**
- **Found during:** Task 2 verification (`cargo fmt -- --check`)
- **Issue:** `src/app/job_state.rs` and `src/setup/dependency_checker.rs` had pre-existing formatting drift that caused `cargo fmt -- --check` to fail
- **Fix:** `cargo fmt` applied to all source files; included both files in commit
- **Files modified:** `src/app/job_state.rs`, `src/setup/dependency_checker.rs`
- **Verification:** `cargo fmt -- --check` exits 0
- **Committed in:** `4b84723`

---

**Total deviations:** 1 auto-fixed (Rule 1 - pre-existing format drift in unrelated files)
**Impact on plan:** Necessary for `cargo fmt -- --check` acceptance criterion. No behavior change.

## Issues Encountered
- Pre-existing clippy errors (15 `collapsible_if` warnings in `file_ops.rs`, `keyboard/dialogs.rs`, `mouse.rs`, `typst_pdf.rs`) are out-of-scope and unchanged by this plan. `cargo clippy -- -D warnings` still reports 15 errors, all pre-existing.

## Threat Surface Scan

No new network endpoints, auth paths, file access patterns, or schema changes introduced. This plan reduces the threat surface — it does not expand it.

## Known Stubs
None.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- SEC-04 (CR-03) fully mitigated
- All preview temp files now use O_EXCL; predictable path attack impossible
- `cargo build` and `cargo test` (118 tests) pass cleanly
- Pre-existing clippy errors remain; plan 01-02 or later cleanup plan should address them

## Self-Check

- [x] `src/browser/pdf_export.rs` — `default_preview_file` fn exists, returns `Result<NamedTempFile>`
- [x] `src/app/mod.rs` — `temp_preview_files: Vec<tempfile::NamedTempFile>`
- [x] 4 unit tests pass: `test_preview_file_has_html_suffix`, `test_preview_file_empty_project_name`, `test_preview_files_are_unique`, `test_namedtempfile_deletes_on_drop`
- [x] `grep -rn "default_preview_filename" src/` — only appears in doc comment, not as function call
- [x] `cargo build` — clean
- [x] `cargo test` — 118 passed, 0 failed

## Self-Check: PASSED

---
*Phase: 01-security-hardening*
*Completed: 2026-05-11*
