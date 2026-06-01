---
quick_id: 260601-iwi
status: complete
date: 2026-06-01
commit: affd62c
---

# Quick Task 260601-iwi — Summary

## Task
Fix file-browser mouse-wheel scroll: the wheel moved the selection instead of
the viewport, breaking the click→item mapping after scrolling.

## Root Cause (systematic-debugging Phase 1)
`MouseEventKind::ScrollDown`/`ScrollUp` over the files pane in `src/app/mouse.rs`
called `file_browser.down()`/`up()`, which mutate `list_state.select(±1)` — they
move the **selection**, not the viewport. ratatui 0.30 couples a `ListState`'s
`offset` to its `selected` (during render it sets `offset = first_visible_index`
so the selection stays visible). Because the wheel drove `selected` to the
visible edge, the highlight wandered while "scrolling" and the click hit-test
(`idx = list_state.offset() + relative_y`) mapped to the wrong row after a scroll.

The click hit-test itself was verified **correct** and was left unchanged.

## Implementation (test-first, co-located unit tests)
- **`src/app/mouse.rs`**
  - Added pure helper `scroll_files_pane(offset, item_count, visible_height, delta, direction) -> usize` — viewport offset math, clamped to `[0, item_count - visible_height]` (down) / floored at 0 (up).
  - Added pure helper `clamp_selected_to_window(selected, offset, visible_height) -> usize` — keeps the selection inside `[offset, offset + visible_height - 1]` so ratatui does not snap the offset back on the next render.
  - Added `ScrollDirection { Down, Up }` enum.
  - Added `App::scroll_file_browser(direction)` — wires both helpers into the wheel handler: sets `*list_state.offset_mut()`, clamps the selection, and refreshes the preview only when the clamp actually changed the selection.
  - Rewired the `ScrollDown`/`ScrollUp` files-pane branches to call `scroll_file_browser` instead of `down()`/`up()`.
  - Added `#[cfg(test)] mod tests` with **10** deterministic unit tests (4 scroll-down, 3 scroll-up, 3 clamp), per TESTING.md co-located convention.
- **`src/app/mod.rs`** — added `pub files_pane_height: u16` field (init 0).
- **`src/app/drawing.rs`** — cache `files_pane_height = files.height.saturating_sub(3)` each frame (top border + bottom border + 1-line info bar), matching the click hit-test geometry.

## Deviations from PLAN.md
1. **Visible height `-3`, not `-2`.** The plan's `read_first` suggested `files.height.saturating_sub(2)`. That is an off-by-one: the verified click hit-test uses `files.height - 3` visible rows. Using `-2` would make the clamp window one row too tall, causing ratatui to snap the offset back by one (the exact 1-row jump we set out to fix). Implemented with `-3`.
2. **Co-located unit tests, no lib target.** An earlier executor run (interrupted by an API error) had created `src/lib.rs` + a `tests/mouse_wheel_scroll.rs` integration test. Per operator decision and TESTING.md convention, those were discarded and the 10 tests live co-located in `mouse.rs`. No `src/lib.rs` / lib target was introduced.

## Verification
- `cargo test` — **140 passed** (130 baseline + 10 new), 3 ignored (network), 0 failed. `tests/cli.rs` 3/3 pass.
- `cargo clippy -- -D warnings` — the fix introduces **zero** new warnings. Proven by stashing the change and re-running clippy on the baseline: identical 16 pre-existing errors in the same untouched files.

## Out-of-scope finding (NOT fixed here)
`cargo clippy -- -D warnings` is **red on the baseline** (16 errors) due to Rust
1.95.0 toolchain drift surfacing newer lints (`collapsible_if`/`collapsible_match`,
`useless_vec`) on pre-existing code:
- `src/app/file_ops.rs` (7)
- `src/browser/typst_pdf.rs` (4)
- `src/app/keyboard/dialogs.rs` (2)
- `src/app/mouse.rs` (2 — the pre-existing pane-resize-drag block)
- `src/update/check.rs` (`useless_vec` in tests, under `--all-targets`)

Recommend a separate quick task (`clippy --fix` sweep + toolchain pin) — fixing
these here would be scope creep across unrelated files incl. Phase-1 security code.

## Manual verification (Task 3 — operator)
Live TUI smoke test (cannot be automated): launch `cargo run`, in the file
browser mouse-wheel down/up over a directory longer than the pane, confirm the
list **content** scrolls (not just the highlight), then click a visible row and
confirm the clicked file is selected.
