---
phase: quick-260615-jwb
plan: "01"
subsystem: preview-edit-mode
tags: [keyboard, shortcuts, cursor, clipboard, tui]
dependency_graph:
  requires: []
  provides: [standard-edit-shortcuts, hardware-cursor, ctrl-x-guard]
  affects: [src/ui/preview.rs, src/app/keyboard/preview.rs, src/app/keyboard/global.rs]
tech_stack:
  added: []
  patterns: [hardware-cursor-set_cursor_position, selection-or-line-fallback]
key_files:
  created: []
  modified:
    - src/app/keyboard/preview.rs
    - src/app/keyboard/global.rs
    - src/ui/preview.rs
    - src/ui/help.rs
    - Cargo.toml
    - USAGE.md
    - README.md
    - /Users/picard/.claude/skills/claude-workbench/SKILL.md
decisions:
  - "Ctrl+Z handled before Shift branch to avoid Ctrl+Shift+Z being caught by SHIFT arm"
  - "cut_selection_or_line extracts line text in a scoped borrow before calling delete_line() to avoid double-borrow"
  - "Software cursor span kept for unfocused state (split-screen position indicator)"
  - "Hardware cursor only emitted inside the Edit mode block, not in ReadOnly"
metrics:
  duration: "~35 minutes"
  completed: "2026-06-15T12:33:00Z"
  tasks_completed: 3
  files_changed: 8
---

# Phase quick-260615-jwb Plan 01: Editor Copy/Cut/Paste Shortcuts and Cursor Fix Summary

Standard Ctrl+C/X/V/Z copy/cut/paste/undo in Preview Edit mode with line-fallback and hardware cursor via f.set_cursor_position().

## What Was Built

Three interrelated fixes to the Preview pane's Edit mode:

**1. Shortcut rewiring (Task 1)**

Removed the four MC-Edit Ctrl+F-key branches (`Ctrl+F3`, `Ctrl+F5`, `Ctrl+F6`, `Ctrl+F8`) from `handle_preview_edit_key`. These were swallowed by global F-key handlers (maximize, pane toggles, settings) before reaching the edit handler, making them unreliable. Replaced with:

- `Ctrl+C` → `copy_selection_or_line()` — copies selection or current line
- `Ctrl+X` → `cut_selection_or_line()` — cuts selection or current line
- `Ctrl+Z` → `editor.undo()`
- `Ctrl+Shift+Z` → `editor.redo()`

Added guard in `global.rs` Ctrl+X handler: when `active_pane == Preview && mode == Edit`, returns `false` immediately so the edit handler consumes the key. Export dialog remains available via Ctrl+X in ReadOnly Preview and FileBrowser.

**2. New PreviewState methods (Task 1)**

- `copy_selection_or_line()`: when selection/block_marking active, delegates to `editor.copy()` + clipboard; otherwise copies current line text with trailing newline
- `cut_selection_or_line()`: when selection active, delegates to `editor.cut()` + clipboard + resets block_marking; otherwise extracts line in a scoped borrow, copies to clipboard, then calls `delete_line()`

**3. Hardware cursor (Task 2)**

In the Edit mode render block of `render_preview`:
- Software cursor span (`insert_cursor_into_line`) now wrapped in `if idx == cursor_row && !is_focused` — suppressed when pane has focus to avoid double cursor
- After `f.render_widget(paragraph, content_area)`, `f.set_cursor_position()` emits the terminal's native blinking cursor when focused and cursor is within visible bounds

**4. Docs and version bump (Task 3)**

- `render_edit_shortcuts`: block_marking branch now shows identical shortcuts to non-block-marking (no more `^F3`/`^F8`)
- `help.rs`: "MC Edit Style Selection" section replaced with "Edit Mode Shortcuts" listing Ctrl+C/X/V/Z/Shift+Z
- `Cargo.toml`: 0.96.1 → 0.97.0
- `USAGE.md`: block ops section removed, standard shortcuts updated, line-fallback note added
- `README.md`: "What's New v0.97.0" entry added
- `SKILL.md`: version bumped, MC Edit section rewritten to reflect standard shortcuts

## Commits

| Hash | Message |
|------|---------|
| 02b50db | [CHG] rewire Edit mode shortcuts: Ctrl+C/X/Z/Shift+Z + hardware cursor |
| 37fb1f0 | [CHG] v0.97.0: docs, labels, version bump |

## Deviations from Plan

None — plan executed exactly as written. The borrow issue in `cut_selection_or_line` was anticipated in the plan with the scoped-borrow pattern, which compiled correctly on first attempt.

## Decisions Made

1. **Ctrl+Z before SHIFT branch**: Placed the Ctrl+Z and Ctrl+Shift+Z arms before the `key.modifiers.contains(KeyModifiers::SHIFT)` branch. The Ctrl+Shift+Z arm checks for both `CONTROL` and `SHIFT`, so if it were inside the SHIFT arm it would still work, but placing it before is cleaner and matches the plan's intent.

2. **Scoped borrow for cut line-fallback**: `cut_selection_or_line` extracts line text inside a `{}` block that drops all borrows before calling `self.delete_line()`. This satisfies Rust's borrow checker without `drop(editor)`.

3. **Software cursor as unfocused indicator**: Keeping the software cursor span for unfocused state is useful in split-screen to show where the cursor is. Suppressing it only when focused avoids the double-cursor visual.

4. **Hardware cursor bounds check**: `cy < content_area.height && cx < content_area.width` prevents setting the cursor outside the content area when the cursor is off-screen (horizontally scrolled or vertically out of view).

## Known Stubs

None.

## Threat Flags

None — changes are purely client-side editor operations on user-opened files. No new network endpoints, auth paths, or file access patterns introduced.

## Self-Check: PASSED

- src/app/keyboard/preview.rs: FOUND (Ctrl+F3/F5/F6/F8 branches: 0; copy_selection_or_line calls: present)
- src/app/keyboard/global.rs: FOUND (EditorMode::Edit guard: present)
- src/ui/preview.rs: FOUND (copy_selection_or_line/cut_selection_or_line methods: 2; hardware cursor block: present)
- Cargo.toml version: "0.97.0" — VERIFIED
- SKILL.md version: "0.97.0" — VERIFIED
- Commits 02b50db, 37fb1f0: VERIFIED
- cargo build: clean
- cargo test: 3 passed (all CLI tests)
- cargo fmt --check: clean
- cargo clippy: 0 errors
