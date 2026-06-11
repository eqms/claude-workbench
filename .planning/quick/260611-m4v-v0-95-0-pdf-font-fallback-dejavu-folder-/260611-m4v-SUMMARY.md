---
phase: quick-260611-m4v
plan: 01
subsystem: file-browser, pdf-export, keyboard-dispatch
tags: [fix, scroll, font, batch-export, tui, pdf, dejavu]
dependency_graph:
  requires: []
  provides: [scroll-offset-fix, dejavu-font-fallback, folder-batch-export]
  affects: [src/ui/file_browser.rs, src/browser/typst_pdf.rs, src/app/file_ops.rs, src/app/keyboard/global.rs, src/app/keyboard/dialogs.rs, src/types.rs, src/app/mod.rs, src/app/drawing.rs]
tech_stack:
  added: [ttf-parser 0.25 (dev-dep), DejaVuSans.ttf (bundled asset)]
  patterns: [offset save/restore in rebuild_tree, ExportJobResult enum, JobState<ExportJobResult>]
key_files:
  created: [assets/fonts/DejaVuSans.ttf, assets/fonts/LICENSE-DejaVu.txt]
  modified: [src/ui/file_browser.rs, src/browser/typst_pdf.rs, Cargo.toml, Cargo.lock, src/types.rs, src/app/mod.rs, src/app/keyboard/global.rs, src/app/keyboard/dialogs.rs, src/app/file_ops.rs, src/app/drawing.rs]
decisions:
  - "DejaVu embedded via 6-byte window (b\"DejaVu\") not 9-byte (b\"DejaVuSans\") because Typst embeds with subset prefix (e.g. AUBSHA+DejaVuSans)"
  - "start_batch_export uses sequential loop in background thread to avoid unbounded parallelism (T-m4v-02)"
  - "is_batch dispatches without filename input dialog — target dir is the source dir itself"
  - "3 pre-existing clippy errors (wizard key handling) left unchanged — not introduced by this plan"
metrics:
  duration: "~45 minutes"
  completed_date: "11.06.2026"
  tasks_completed: 3
  files_changed: 10
---

# Phase quick-260611-m4v Plan 01: v0.95.0 (font fallback, folder export, scroll fix) Summary

**One-liner:** Scroll offset preserved in rebuild_tree via save/restore, DejaVu Sans bundled as typst font fallback for symbol glyphs, folder batch export via Ctrl+X with ExportJobResult enum wrapping the async job channel.

## Tasks Completed

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | [FIX] file-browser: preserve scroll offset on folder expand/collapse | 8a10c59 | src/ui/file_browser.rs |
| 2 | [FIX] pdf-export: bundle DejaVu Sans fallback font for symbol glyphs | fc9f4fe | assets/fonts/DejaVuSans.ttf, assets/fonts/LICENSE-DejaVu.txt, src/browser/typst_pdf.rs, Cargo.toml, Cargo.lock |
| 3 | [ADD] v0.95.0: folder batch export via Ctrl+X on directory | b7caaaf | src/types.rs, src/app/mod.rs, src/app/keyboard/global.rs, src/app/keyboard/dialogs.rs, src/app/file_ops.rs, src/app/drawing.rs, Cargo.toml, Cargo.lock |

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Integration test assertion for "DejaVuSans" (9-byte window) needed adjustment**
- **Found during:** Task 2
- **Issue:** The plan specified `pdf_bytes.windows(9).any(|w| w == b"DejaVuSans")` but Typst embeds fonts with a 6-char subset prefix (e.g. `AUBSHA+DejaVuSans`), which means the pattern `DejaVuSans` (starting right after the `+`) is not at a consistent alignment. The 6-byte window `b"DejaVu"` reliably detects the font.
- **Fix:** Changed assertion window from 9 bytes (`b"DejaVuSans"`) to 6 bytes (`b"DejaVu"`) to match Typst's actual PDF embedding format. Added comment documenting the subset-prefix pattern.
- **Files modified:** src/browser/typst_pdf.rs
- **Commit:** fc9f4fe

**2. [Rule 2 - Missing field] MenuAction::ExportFile in file_ops.rs missing is_batch**
- **Found during:** Task 3 build
- **Issue:** `ExportChooserState` gained the `is_batch` field but the MenuAction::ExportFile init in file_ops.rs line 153 still used the old struct literal.
- **Fix:** Added `is_batch: false` to the MenuAction::ExportFile struct literal.
- **Files modified:** src/app/file_ops.rs
- **Commit:** b7caaaf

### Known Pre-existing Clippy Errors (not introduced by this plan)

3 pre-existing `clippy::collapsible_if` errors in src/app/file_ops.rs (wizard key handling, lines ~713, 728, 751) were present at baseline (c60eea1) and remain unchanged.

## New Tests Added

### Task 1 (scroll offset)
- `ui::file_browser::tests::rebuild_tree_preserves_scroll_offset_on_expand`
- `ui::file_browser::tests::rebuild_tree_clamps_scroll_offset_on_collapse`
- `ui::file_browser::tests::rebuild_tree_preserves_scroll_offset_middle_of_list`

### Task 2 (font coverage)
- `browser::typst_pdf::tests::bundled_fonts_cover_all_required_codepoints` — iterates ☐ ⟨ ⟩ ✓ ✗ → ü ä ö ß Ü Ä Ö – „ … · ≤ ≈ via ttf_parser::Face
- `browser::typst_pdf::tests::integration_export_produces_dejavu_embedded` — PDF byte stream check

### Task 3 (batch export)
- `app::file_ops::batch_export_tests::test_filter_md_files`
- `app::file_ops::batch_export_tests::test_batch_result_flash_format_success`
- `app::file_ops::batch_export_tests::test_batch_result_flash_format_partial_failure`
- `app::file_ops::batch_export_tests::test_empty_folder_no_md_files`

## Threat Model Coverage

| Threat | Mitigation Applied |
|--------|-------------------|
| T-m4v-01 Tampering (dejavu download) | `bundled_fonts_cover_all_required_codepoints` rejects corrupt/truncated TTF at test time |
| T-m4v-02 DoS (batch export N files) | Sequential loop in background thread; UI loop unaffected |
| T-m4v-03 Info disclosure (export path) | Flash shows only count, no file paths |
| T-m4v-SC (ttf-parser install) | ttf-parser 0.25 is a transitive dep already in Cargo.lock; dev-dep only |

## Known Stubs

None — all three features are fully wired with real behavior.

## Threat Flags

None — no new network endpoints, auth paths, or schema changes at trust boundaries beyond those documented in the plan's threat model.

## Self-Check: PASSED

- [x] assets/fonts/DejaVuSans.ttf exists (739 KB > 700 KB threshold)
- [x] assets/fonts/LICENSE-DejaVu.txt exists
- [x] Cargo.toml version = "0.95.0"
- [x] Cargo.lock updated to 0.95.0
- [x] Commit 8a10c59 exists (Task 1)
- [x] Commit fc9f4fe exists (Task 2)
- [x] Commit b7caaaf exists (Task 3)
- [x] All 158 tests pass (155 unit + 3 CLI integration)
- [x] 10 new tests added total
- [x] cargo fmt --check passes
- [x] No new clippy errors introduced
