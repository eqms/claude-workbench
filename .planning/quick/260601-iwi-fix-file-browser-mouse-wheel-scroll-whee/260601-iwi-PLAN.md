---
phase: quick
plan: 260601-iwi
type: execute
wave: 1
depends_on: []
files_modified:
  - src/app/mouse.rs
  - tests/mouse_wheel_scroll.rs
autonomous: true
requirements: [IWI-fix-file-browser-mouse-wheel]
must_haves:
  truths:
    - "Mouse wheel ScrollDown on the files pane increments the scroll offset"
    - "Mouse wheel ScrollUp on the files pane decrements the scroll offset (floor 0)"
    - "Scroll delta is clamped so offset never exceeds (item_count - visible_height)"
    - "After a wheel scroll, selected is clamped into [offset, offset + visible_height - 1] so ratatui does NOT revert the offset on the next render"
    - "Click hit-test formula idx = list_state.offset() + relative_y still maps to the visually-clicked row"
    - "Click hit-test logic in src/app/mouse.rs is unchanged"
  artifacts:
    - path: "tests/mouse_wheel_scroll.rs"
      provides: "Deterministic unit tests for scroll_files_pane and clamp_selected_to_window helpers"
    - path: "src/app/mouse.rs"
      provides: "scroll_files_pane + clamp_selected_to_window helpers wired into ScrollDown/ScrollUp branches"
  key_links:
    - from: "tests/mouse_wheel_scroll.rs"
      to: "src/app/mouse.rs"
      via: "pub(crate) fn scroll_files_pane + clamp_selected_to_window"
      pattern: "scroll_files_pane|clamp_selected_to_window"
---

<objective>
Fix mouse wheel scrolling in the file browser pane.

Root cause (established): ScrollDown / ScrollUp events for the files pane call
`self.file_browser.down()` / `self.file_browser.up()`, which move the ratatui
`ListState` *selected* index by 1. The real scroll offset lives at
`self.file_browser.list_state.offset()` and is never updated, so the list appears
frozen unless the user reaches the edge of the current visible window.

There is no separate `file_browser_scroll` field. The scroll state IS ratatui's
`ListState` at `self.file_browser.list_state`, where `offset` tracks the first
visible row. During every `render_stateful_widget` call ratatui recomputes
`first_visible_index = get_items_bounds(selected, offset, height)` and sets
`state.offset = first_visible_index`. If `selected` is outside
`[offset, offset + visible_height - 1]` ratatui snaps the offset to wherever
`selected` is, discarding the wheel-computed offset.

Fix: compute the new offset with a pure helper, then clamp `selected` into the
new visible window before returning so that ratatui's render loop leaves the
offset intact.

Purpose: Make mouse wheel scroll the file list up and down, with correct clamping,
without moving the selection unless it falls outside the new visible window.
Output: Two pure helpers (`scroll_files_pane`, `clamp_selected_to_window`) with
unit tests, wired into the existing ScrollDown/ScrollUp arms for the files pane.
</objective>

<execution_context>
@/Users/picard/.claude/get-shit-done/workflows/execute-plan.md
@/Users/picard/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@/Users/picard/gitbase/workbench/.planning/PROJECT.md
@/Users/picard/gitbase/workbench/src/app/mouse.rs
</context>

<tasks>

<task type="auto" tdd="true">
  <name>Task 1: Write failing unit tests for scroll_files_pane and clamp_selected_to_window</name>
  <files>tests/mouse_wheel_scroll.rs</files>
  <read_first>
    src/app/mouse.rs — confirm the crate name from Cargo.toml `[lib]` / `[package] name`
    so the `use` path in the test file is correct. The helpers do not exist yet.
  </read_first>
  <behavior>
    scroll_files_pane cases:
    - scroll_files_pane(offset=0, item_count=20, visible_height=10, delta=3, Down)  -> 3
    - scroll_files_pane(offset=8, item_count=20, visible_height=10, delta=3, Down)  -> 10  (clamped to item_count - visible_height = 10)
    - scroll_files_pane(offset=10, item_count=20, visible_height=10, delta=3, Down) -> 10  (already at max, no change)
    - scroll_files_pane(offset=5, item_count=20, visible_height=10, delta=3, Up)    -> 2
    - scroll_files_pane(offset=2, item_count=20, visible_height=10, delta=3, Up)    -> 0   (floor 0)
    - scroll_files_pane(offset=0, item_count=20, visible_height=10, delta=3, Up)    -> 0   (already at top)
    - scroll_files_pane(offset=0, item_count=5,  visible_height=10, delta=1, Down)  -> 0   (list shorter than pane — max=0, no scroll)

    clamp_selected_to_window cases:
    - clamp_selected_to_window(selected=15, offset=5, visible_height=10) -> 14  (selected above window: clamp to offset + visible_height - 1 = 14)
    - clamp_selected_to_window(selected=3,  offset=5, visible_height=10) -> 5   (selected below window: clamp to offset)
    - clamp_selected_to_window(selected=8,  offset=5, visible_height=10) -> 8   (already inside window [5..14]: no change)
  </behavior>
  <action>
    Create `tests/mouse_wheel_scroll.rs` as a Rust integration test file.

    Import both helpers:
      use workbench::app::mouse::{scroll_files_pane, clamp_selected_to_window, ScrollDirection};
    (Replace `workbench` with the actual crate name from Cargo.toml if different.)

    Write one `#[test]` function per behavior case listed above, named descriptively
    (e.g. `scroll_down_clamps_to_max`, `scroll_up_floors_at_zero`,
    `clamp_selected_above_window`, `clamp_selected_below_window`,
    `clamp_selected_inside_window`).

    Helper signatures the tests must compile against:
      pub(crate) fn scroll_files_pane(offset: usize, item_count: usize,
                                      visible_height: usize, delta: usize,
                                      direction: ScrollDirection) -> usize

      pub(crate) fn clamp_selected_to_window(selected: usize, offset: usize,
                                             visible_height: usize) -> usize

    Where `ScrollDirection` is an enum `{ Down, Up }` defined in `src/app/mouse.rs`.

    At this stage neither function exists yet, so `cargo test` MUST fail with
    "cannot find function" or "unresolved import" — that is the RED state.
    Do not implement either function in this task.
  </action>
  <verify>
    <automated>cd /Users/picard/gitbase/workbench && cargo test --test mouse_wheel_scroll 2>&1 | grep -E "(error\[|FAILED|unresolved|cannot find)"</automated>
  </verify>
  <done>
    `cargo test --test mouse_wheel_scroll` fails to compile with an "unresolved"
    or "cannot find" error — confirming all 10 tests exist and neither helper is
    implemented yet (RED state confirmed).
  </done>
</task>

<task type="auto" tdd="true">
  <name>Task 2: Implement scroll_files_pane and clamp_selected_to_window, wire into mouse handler</name>
  <files>src/app/mouse.rs</files>
  <read_first>
    src/app/mouse.rs — locate:
    1. The `MouseEventKind::ScrollDown` arm at ~line 918 (`self.file_browser.down()`)
       and the `MouseEventKind::ScrollUp` arm at ~line 989 (`self.file_browser.up()`).
    2. The click hit-test at ~line 405:
         let scroll_offset = self.file_browser.list_state.offset();
         let idx = scroll_offset + relative_y as usize;
       This formula must remain exactly as-is after this task.
    3. How `list_state.select(Some(idx))` is called — this same method is used for
       the selected clamp.
    4. Where visible height can be read. If the layout rect for the files pane is
       stored in `App` (e.g. `self.last_files_rect` or similar), use
       `rect.height.saturating_sub(2) as usize`. If not cached, add a
       `pub(crate) files_pane_height: u16` field on `App` (default 0) and assign it
       from the files layout rect each frame in the draw function:
         self.files_pane_height = files_rect.height.saturating_sub(2);
  </read_first>
  <action>
    Step 1 — Add `ScrollDirection` enum if it does not exist:

      pub(crate) enum ScrollDirection { Down, Up }

    Step 2 — Implement the pure offset helper (no `self`, no side-effects):

      pub(crate) fn scroll_files_pane(
          offset: usize,
          item_count: usize,
          visible_height: usize,
          delta: usize,
          direction: ScrollDirection,
      ) -> usize {
          let max = item_count.saturating_sub(visible_height);
          match direction {
              ScrollDirection::Down => (offset + delta).min(max),
              ScrollDirection::Up  => offset.saturating_sub(delta),
          }
      }

    Step 3 — Implement the pure selected-clamp helper (no `self`, no side-effects):

      pub(crate) fn clamp_selected_to_window(
          selected: usize,
          offset: usize,
          visible_height: usize,
      ) -> usize {
          let window_last = offset.saturating_add(visible_height).saturating_sub(1);
          selected.max(offset).min(window_last)
      }

    Step 4 — Wire into the existing mouse handler.

    IMPORTANT: The file browser has NO separate scroll field. The scroll offset is
    `self.file_browser.list_state.offset()` (read) and
    `*self.file_browser.list_state.offset_mut() = new_offset` (write). There is no
    `self.file_browser_scroll`.

    In the `MouseEventKind::ScrollDown` arm, replace the files-pane branch:

      // BEFORE:
      self.file_browser.down();

      // AFTER:
      {
          let item_count = self.file_browser.entries.len();
          let visible_h = self.files_pane_height as usize;
          let current_offset = self.file_browser.list_state.offset();
          let new_offset = scroll_files_pane(
              current_offset, item_count, visible_h, 3, ScrollDirection::Down,
          );
          *self.file_browser.list_state.offset_mut() = new_offset;
          // Clamp selected into the new visible window so ratatui does not revert
          // the offset on the next render (ratatui snaps offset to keep selected
          // visible; if selected is already in-window, offset is preserved as-is).
          if let Some(sel) = self.file_browser.list_state.selected() {
              let clamped = clamp_selected_to_window(sel, new_offset, visible_h);
              if clamped != sel {
                  self.file_browser.list_state.select(Some(clamped));
              }
          }
      }

    Mirror for `MouseEventKind::ScrollUp`:

      // BEFORE:
      self.file_browser.up();

      // AFTER:
      {
          let item_count = self.file_browser.entries.len();
          let visible_h = self.files_pane_height as usize;
          let current_offset = self.file_browser.list_state.offset();
          let new_offset = scroll_files_pane(
              current_offset, item_count, visible_h, 3, ScrollDirection::Up,
          );
          *self.file_browser.list_state.offset_mut() = new_offset;
          if let Some(sel) = self.file_browser.list_state.selected() {
              let clamped = clamp_selected_to_window(sel, new_offset, visible_h);
              if clamped != sel {
                  self.file_browser.list_state.select(Some(clamped));
              }
          }
      }

    Keep `self.update_preview()` calls immediately after each block, as before.

    Step 5 — If `files_pane_height` is not yet a field on `App`, add it:
      pub(crate) files_pane_height: u16   (default 0)
    Assign it in the draw function from the files layout rect:
      self.files_pane_height = files_rect.height.saturating_sub(2);

    Do NOT change the click hit-test branches (`MouseEventKind::Down` at ~line 405).
    The existing formula `idx = list_state.offset() + relative_y` is correct and must
    remain exactly as-is — it reads the same offset that wheel scroll now writes.
    Do NOT change scroll handling for Preview, Claude, LazyGit, or Terminal panes.
  </action>
  <verify>
    <automated>cd /Users/picard/gitbase/workbench && cargo test --test mouse_wheel_scroll && cargo clippy -- -D warnings 2>&1 | tail -20</automated>
  </verify>
  <done>
    All 10 unit tests in `tests/mouse_wheel_scroll.rs` pass (GREEN).
    `cargo clippy -- -D warnings` exits 0.
    `cargo build` succeeds.
    No changes to click hit-test branches.
    After a wheel scroll: `list_state.offset()` equals the computed new_offset AND
    `list_state.selected()` is clamped into `[new_offset, new_offset + visible_h - 1]`,
    which means ratatui's render loop leaves the offset intact on the very next frame.
  </done>
</task>

<task type="checkpoint:human-verify" gate="blocking">
  <name>Task 3: Verify live mouse wheel scroll in file browser</name>
  <what-built>
    Pure helpers scroll_files_pane and clamp_selected_to_window with unit tests,
    wired into ScrollDown/ScrollUp mouse events for the files pane. The offset is
    written via list_state.offset_mut() and selected is clamped into the new visible
    window so ratatui does not revert the offset on the next render.
  </what-built>
  <how-to-verify>
    1. Run: `cargo run` from /Users/picard/gitbase/workbench
    2. Navigate to a directory with more files than fit on screen (e.g. src/).
    3. Roll mouse wheel DOWN over the file browser pane — the list should scroll down
       smoothly, showing rows that were below the fold.
    4. Roll mouse wheel UP — the list should scroll back toward the top.
    5. Confirm scrolling stops at top (does not underflow) and at the last item (no
       blank space at bottom).
    6. Click a visible file in the file browser — confirm the correct file is
       selected (hit-test `idx = offset + relative_y` still works).
    7. Scroll down several times, then click a file — confirm the clicked row matches
       the file that was visually under the cursor (not offset by a stale value).
  </how-to-verify>
  <resume-signal>Type "approved" if scroll works, or describe the failure.</resume-signal>
</task>

</tasks>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| Mouse event → scroll offset | Raw u16 mouse coordinates from crossterm; clamping prevents out-of-bounds indexing |

## STRIDE Threat Register

| Threat ID | Category | Component | Disposition | Mitigation Plan |
|-----------|----------|-----------|-------------|-----------------|
| T-iwi-01 | Tampering | scroll offset (list_state.offset) | mitigate | `scroll_files_pane` clamps to `[0, item_count - visible_height]`; underflow impossible via `saturating_sub` |
| T-iwi-02 | Tampering | selected index after clamp | mitigate | `clamp_selected_to_window` uses `saturating_add`/`saturating_sub`; cannot exceed valid item range |
| T-iwi-03 | Denial of Service | mouse event flood | accept | crossterm event queue is rate-limited by terminal; no additional throttling needed for TUI |
</threat_model>

<verification>
- `cargo test --test mouse_wheel_scroll` — all 10 cases pass
- `cargo clippy -- -D warnings` — exits 0
- `cargo build` — succeeds
- Manual: wheel scroll moves file list; selected stays in visible window; click hit-test unchanged
</verification>

<success_criteria>
Mouse wheel ScrollDown and ScrollUp on the file browser pane scroll the file list
with correct clamping (floor 0, ceiling item_count - visible_height). After each
wheel scroll, selected is clamped into the new visible window so ratatui's render
does not revert the offset. Click hit-test formula (idx = offset + relative_y)
remains intact. All 10 unit tests green, clippy clean, no regression to click hit-test.
</success_criteria>

<output>
Create `.planning/quick/260601-iwi-fix-file-browser-mouse-wheel-scroll-whee/260601-iwi-SUMMARY.md` when done.
</output>
