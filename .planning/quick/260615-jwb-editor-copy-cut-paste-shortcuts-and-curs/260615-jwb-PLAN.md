---
phase: quick-260615-jwb
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - src/app/keyboard/preview.rs
  - src/app/keyboard/global.rs
  - src/ui/preview.rs
  - src/ui/help.rs
  - Cargo.toml
  - USAGE.md
  - README.md
  - /Users/picard/.claude/skills/claude-workbench/SKILL.md
autonomous: true
requirements: [quick-260615-jwb]

must_haves:
  truths:
    - "Ctrl+C in Edit mode copies the selection, or the current line when nothing is selected"
    - "Ctrl+X in Edit mode cuts the selection (or current line) — does NOT open the export dialog"
    - "Ctrl+V pastes from clipboard (unchanged behavior)"
    - "Ctrl+Z undoes last change; Ctrl+Shift+Z redoes"
    - "F3/F5/F6/F8 continue to work globally (maximize, toggles, settings) without interference in Edit mode"
    - "The cursor is visible and blinks in every terminal including Terminus when Edit mode has focus"
    - "In ReadOnly Preview and FileBrowser, Ctrl+X still opens the export dialog"
  artifacts:
    - path: "src/app/keyboard/preview.rs"
      provides: "Rewired Ctrl+C/X/Z handlers; Ctrl+F3/F5/F6/F8 branches removed"
    - path: "src/app/keyboard/global.rs"
      provides: "Ctrl+X falls through to editor when active_pane==Preview && mode==Edit"
    - path: "src/ui/preview.rs"
      provides: "copy_selection_or_line() + cut_selection_or_line() methods; hardware cursor via f.set_cursor_position(); software cursor only when unfocused"
  key_links:
    - from: "src/app/keyboard/preview.rs"
      to: "src/ui/preview.rs"
      via: "self.preview.copy_selection_or_line() / cut_selection_or_line()"
    - from: "src/ui/preview.rs (render)"
      to: "ratatui Frame"
      via: "f.set_cursor_position((content_area.x + cx, content_area.y + cy))"
---

<objective>
Fix three bugs in the Preview pane's Edit mode:

1. Shortcut collision — Ctrl+F3/F5/F6/F8 (MC-Edit block ops) are swallowed by global F-key handlers.
   Solution: remove the four Ctrl+F-key branches entirely; replace with standard Ctrl+C/X/Z shortcuts.

2. Copy/cut fail without selection — current copy_block()/move_block() yield nothing when no selection is active.
   Solution: new copy_selection_or_line() and cut_selection_or_line() fall back to the current line.

3. Cursor invisible in Terminus (and some iTerm2 configs) — software REVERSED|SLOW_BLINK span not rendered.
   Solution: emit a real hardware cursor via f.set_cursor_position() when focused; keep software span only for unfocused state.

Purpose: Standard copy/cut/paste/undo work reliably in every terminal; no shortcut collisions; cursor always visible.
Output: Patched Rust source, updated docs/labels, version bump 0.96.1 → 0.97.0.
</objective>

<execution_context>
@$HOME/.claude/get-shit-done/workflows/execute-plan.md
@$HOME/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@/Users/picard/gitbase/workbench/.planning/STATE.md
@/Users/picard/gitbase/workbench/CLAUDE.md
@/Users/picard/.claude/skills/claude-workbench/SKILL.md
</context>

<tasks>

<task type="auto">
  <name>Task 1: Rewire keyboard shortcuts + add PreviewState helper methods + fix Ctrl+X collision</name>
  <files>
    src/app/keyboard/preview.rs
    src/app/keyboard/global.rs
    src/ui/preview.rs
  </files>
  <action>
## A. src/app/keyboard/preview.rs — handle_preview_edit_key

### Remove the four Ctrl+F-key branches (lines ~204-217)

Delete these four `else if` arms entirely — they handle Ctrl+F3, Ctrl+F5, Ctrl+F6, Ctrl+F8.
After removal, those F-keys fall through to the global handlers (maximize, pane toggles, settings),
which is the desired behavior.

### Rewire Ctrl+C → copy_selection_or_line

The existing Ctrl+C branch (line ~185) calls `self.preview.copy_block()`. Replace the call with:
`self.preview.copy_selection_or_line();`
No `update_modified()` or `update_edit_highlighting()` needed for copy-only.

### Rewire Ctrl+X → cut_selection_or_line

The existing Ctrl+X branch (line ~190) calls `self.preview.move_block()`. Replace with:
`self.preview.cut_selection_or_line();`
Keep the existing `self.preview.update_modified()` and `self.preview.update_edit_highlighting(&self.syntax_manager)` calls that follow.

### Add Ctrl+Z (undo) and Ctrl+Shift+Z (redo) branches

Add two new `else if` arms BEFORE the `else if key.modifiers.contains(KeyModifiers::SHIFT)` arm:

```
// Ctrl+Z: undo
else if key.code == KeyCode::Char('z')
    && key.modifiers.contains(KeyModifiers::CONTROL)
    && !key.modifiers.contains(KeyModifiers::SHIFT)
{
    if let Some(editor) = &mut self.preview.editor {
        editor.undo();
    }
    self.preview.update_modified();
    self.preview.update_edit_highlighting(&self.syntax_manager);
}
// Ctrl+Shift+Z: redo
else if key.code == KeyCode::Char('z')
    && key.modifiers.contains(KeyModifiers::CONTROL)
    && key.modifiers.contains(KeyModifiers::SHIFT)
{
    if let Some(editor) = &mut self.preview.editor {
        editor.redo();
    }
    self.preview.update_modified();
    self.preview.update_edit_highlighting(&self.syntax_manager);
}
```

Note: `editor.undo()` and `editor.redo()` both return `bool` — the return value can be discarded.
The editor field on PreviewState is `Option<TextArea>` — match the existing pattern used in other branches
(e.g. look at how `delete_line()` accesses the editor to confirm the exact field name).

### Leave Ctrl+V, Ctrl+Y, Ctrl+A, Ctrl+S, Ctrl+H, Shift+Arrow branches unchanged.

---

## B. src/app/keyboard/global.rs — Ctrl+X collision guard

The Ctrl+X handler starts at line ~127. Add an early return at the very top of that handler block,
BEFORE any `if self.active_pane == PaneId::FileBrowser` check:

```
if self.active_pane == PaneId::Preview
    && self.preview.mode == EditorMode::Edit
{
    return false;  // Let edit handler consume Ctrl+X as cut
}
```

This ensures that in Edit mode, Ctrl+X is never consumed by the export logic.
In ReadOnly Preview and FileBrowser, Ctrl+X continues to open the export chooser unchanged.

`EditorMode` and `PaneId` are already imported in global.rs (confirmed in grep output).

---

## C. src/ui/preview.rs — new PreviewState methods

Add two public methods to `impl PreviewState`, near the existing `copy_block()` (line ~561)
and `move_block()` (line ~576):

### copy_selection_or_line

```rust
pub fn copy_selection_or_line(&mut self) {
    if let Some(editor) = &mut self.editor {
        // If a selection is active (either tui-textarea native or block_marking mode),
        // delegate to copy_block behavior: editor.copy() then copy yank_text to clipboard.
        if editor.is_selecting() || self.block_marking {
            editor.copy();
            let yank = editor.yank_text();
            if !yank.is_empty() {
                crate::clipboard::copy_to_clipboard(&yank.to_string());
            }
        } else {
            // No selection: copy the current line (with trailing newline)
            let (row, _) = editor.cursor();
            let lines = editor.lines();
            let line_text = lines.get(row).cloned().unwrap_or_default();
            let to_copy = format!("{}\n", line_text);
            crate::clipboard::copy_to_clipboard(&to_copy);
        }
    }
}
```

### cut_selection_or_line

```rust
pub fn cut_selection_or_line(&mut self) {
    if let Some(editor) = &mut self.editor {
        if editor.is_selecting() || self.block_marking {
            // Delegate to existing move_block behavior
            // (copy yank_text first, then editor.cut())
            let yank_before = editor.yank_text().to_string();
            editor.cut();
            let yank_after = editor.yank_text().to_string();
            let text = if !yank_after.is_empty() { yank_after } else { yank_before };
            if !text.is_empty() {
                crate::clipboard::copy_to_clipboard(&text);
            }
        } else {
            // No selection: copy + delete current line
            let (row, _) = editor.cursor();
            let lines = editor.lines();
            let line_text = lines.get(row).cloned().unwrap_or_default();
            let to_copy = format!("{}\n", line_text);
            crate::clipboard::copy_to_clipboard(&to_copy);
            // Reuse delete_line() to remove it from the editor
            drop(editor); // release borrow
            self.delete_line();
        }
    }
}
```

IMPORTANT: The `drop(editor)` borrow-release pattern may not compile due to Rust's borrow rules.
If `self.editor` is accessed mutably above, restructure by extracting the line text first,
calling `copy_to_clipboard` with a local, then calling `self.delete_line()` without a live
borrow of `self.editor`. The correct pattern:

```rust
} else {
    // Extract line text before taking another mutable borrow
    let to_copy = {
        let lines = self.editor.as_ref().unwrap().lines();
        let (row, _) = self.editor.as_ref().unwrap().cursor();
        format!("{}\n", lines.get(row).cloned().unwrap_or_default())
    };
    crate::clipboard::copy_to_clipboard(&to_copy);
    self.delete_line();
}
```

Use whichever form compiles. The semantics must be: copy line to clipboard, then delete it.
  </action>
  <verify>
    <automated>cd /Users/picard/gitbase/workbench && cargo build 2>&1 | tail -20</automated>
  </verify>
  <done>
    cargo build succeeds with zero errors. The four Ctrl+F-key branches are absent from preview.rs keyboard handler.
    copy_selection_or_line() and cut_selection_or_line() exist in src/ui/preview.rs impl PreviewState.
    The Ctrl+X guard is present in global.rs before the FileBrowser check.
  </done>
</task>

<task type="auto">
  <name>Task 2: Hardware cursor in Edit mode render</name>
  <files>
    src/ui/preview.rs
  </files>
  <action>
In the Edit mode render block in `render_preview` (around line 863-877), make two changes:

### 1. Conditionally suppress the software cursor span when focused

The software cursor is applied at line ~863-867 via `insert_cursor_into_line()`.
Wrap that call in `if !is_focused { ... }` so the software cursor span is ONLY rendered
when the pane does NOT have focus (useful as a position indicator in split-screen).
When focused, the hardware cursor (below) will show the real position.

Change the block from:
```rust
if idx == cursor_row {
    let line_with_cursor =
        insert_cursor_into_line(&line_with_selection, cursor_col, line_content);
    lines_with_cursor.push(line_with_cursor);
} else {
    lines_with_cursor.push(line_with_selection);
}
```
To:
```rust
if idx == cursor_row && !is_focused {
    let line_with_cursor =
        insert_cursor_into_line(&line_with_selection, cursor_col, line_content);
    lines_with_cursor.push(line_with_cursor);
} else {
    lines_with_cursor.push(line_with_selection);
}
```

### 2. Set the hardware cursor after rendering the paragraph (after line 877)

Insert immediately after `f.render_widget(paragraph, content_area);` (line 877):

```rust
// Set hardware cursor when focused and cursor is within visible content area
if is_focused {
    let cy = cursor_row.saturating_sub(scroll_offset) as u16;
    let cx = (cursor_col as u16).saturating_sub(state.horizontal_scroll);
    if cy < content_area.height && cx < content_area.width {
        f.set_cursor_position((content_area.x + cx, content_area.y + cy));
    }
}
```

`ratatui::init()` hides the cursor by default. Calling `set_cursor_position()` once per frame
makes it visible and uses the terminal's native blinking cursor — reliable in all terminals
including Terminus (which ignores REVERSED/SLOW_BLINK style modifiers).

`scroll_offset` and `state.horizontal_scroll` are already computed in the same scope.
`content_area`, `cursor_row`, and `cursor_col` are already local variables. No new fields needed.

Do NOT add `f.set_cursor_position()` for any other mode (ReadOnly, etc.) — only inside the
`if let Some(editor) = &state.editor { ... }` block, after the paragraph render.
  </action>
  <verify>
    <automated>cd /Users/picard/gitbase/workbench && cargo build 2>&1 | tail -10</automated>
  </verify>
  <done>
    cargo build succeeds. The `if !is_focused` guard wraps the insert_cursor_into_line call.
    f.set_cursor_position() is called after the paragraph render when is_focused is true.
  </done>
</task>

<task type="auto">
  <name>Task 3: Docs, labels, version bump</name>
  <files>
    src/ui/preview.rs
    src/ui/help.rs
    Cargo.toml
    USAGE.md
    README.md
    /Users/picard/.claude/skills/claude-workbench/SKILL.md
  </files>
  <action>
## A. src/ui/preview.rs — render_edit_shortcuts block_marking branch

In `render_edit_shortcuts()` (line ~1499), the `block_marking` branch (lines ~1502-1509) currently shows:
`("^F3", "EndBlk")` and `("^F8", "Del")`.

Replace the entire `block_marking = true` shortcuts vec with entries that reflect the standard scheme:

```rust
vec![
    ("Sh+←→↑↓", "Mark"),
    ("^C", "Copy"),
    ("^X", "Cut"),
    ("^V", "Paste"),
    ("^Z", "Undo"),
    ("^Y", "DelLn"),
    ("^H", "S&R"),
    ("^S", "Save"),
    ("Esc", "Exit"),
]
```

(The block_marking branch can now show the same shortcuts as non-block-marking since the F-key
block ops are gone. Alternatively, keep it identical — no distinction needed now.)

## B. src/ui/help.rs — MC Edit Style Selection section

Find the section starting at line ~482 ("MC Edit Style Selection") and update it.
Remove references to Ctrl+F3, Ctrl+F5, Ctrl+F6, Ctrl+F8 block ops.
Replace with the standard shortcut table:

The section heading can remain "Edit Mode Shortcuts" or "Standard Edit Shortcuts".
Replace the four Ctrl+F-key spans (lines ~490-503) with:

- `Ctrl+C` — Copy selection, or current line if nothing selected
- `Ctrl+X` — Cut selection, or current line if nothing selected
- `Ctrl+V` — Paste from clipboard
- `Ctrl+Z` — Undo last change
- `Ctrl+Shift+Z` — Redo
- `Shift+Arrow` — Extend selection
- `Ctrl+Y` — Delete current line

Match the existing span formatting style (green key, raw description) used throughout help.rs.

## C. Cargo.toml — version bump

Change line 3: `version = "0.96.1"` → `version = "0.97.0"`

## D. USAGE.md

Find the Edit mode shortcuts section. Replace any mention of Ctrl+F3/F5/F6/F8 block operations
with the standard shortcuts: Ctrl+C (copy), Ctrl+X (cut), Ctrl+V (paste), Ctrl+Z (undo),
Ctrl+Shift+Z (redo). Add a note: "Ctrl+C and Ctrl+X without selection act on the current line."

## E. README.md

Find the edit mode or preview pane shortcuts section (if any Ctrl+F-key block ops are documented there).
Update to match the standard scheme. If no block-op documentation exists in README.md, add a brief
mention of the standard shortcuts in the Preview/Edit section.

## F. SKILL.md — /Users/picard/.claude/skills/claude-workbench/SKILL.md

1. Update frontmatter: `version: "0.96.1"` → `version: "0.97.0"`
2. Update `**Current Version:** 0.96.1` → `**Current Version:** 0.97.0`
3. Rewrite the "MC Edit Style Selection (v0.18.0+)" section:
   - Change heading to "Edit Mode Shortcuts (standard, updated v0.97.0)"
   - Remove the PreviewState method table entries for toggle_block_marking/copy_block/move_block/delete_block
     (or keep them as internal methods but remove from key mappings)
   - Replace key mappings list with:
     - `Shift+Arrow` — Extend selection
     - `Ctrl+C` — Copy selection or current line
     - `Ctrl+X` — Cut selection or current line (does NOT trigger export in Edit mode)
     - `Ctrl+V` — Paste from clipboard
     - `Ctrl+Z` — Undo
     - `Ctrl+Shift+Z` — Redo
     - `Ctrl+Y` — Delete current line
4. Add a row to "Recent Version History":
   `| v0.97.0 | **Edit Mode: Standard Copy/Cut/Paste/Undo + Hardware Cursor** — Replaced MC-Edit Ctrl+F3/F5/F6/F8 block shortcuts (broken due to global F-key collision) with standard Ctrl+C/X/V/Z. Ctrl+C and Ctrl+X without selection act on the current line. Ctrl+X no longer triggers export dialog in Edit mode. Hardware cursor via f.set_cursor_position() fixes invisible cursor in Terminus and iTerm2. |`

## G. Run quality gates

After all edits:
```bash
cd /Users/picard/gitbase/workbench
cargo test 2>&1 | tail -5
cargo fmt
cargo clippy 2>&1 | grep -E "^error" | head -20
```

Fix any clippy errors before committing. `cargo fmt` output is expected (no errors).
  </action>
  <verify>
    <automated>cd /Users/picard/gitbase/workbench && cargo build && cargo test 2>&1 | tail -5 && cargo fmt --check 2>&1 | head -5 && cargo clippy 2>&1 | grep "^error" | wc -l</automated>
  </verify>
  <done>
    cargo build succeeds. cargo test passes (≥106 tests). cargo fmt --check produces no diff.
    cargo clippy produces 0 error lines.
    Cargo.toml version is "0.97.0". SKILL.md frontmatter version is "0.97.0".
    help.rs no longer references Ctrl+F3/F5/F6/F8 for block ops.
  </done>
</task>

</tasks>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| clipboard←editor | Text extracted from editor buffer and sent to system clipboard |

## STRIDE Threat Register

| Threat ID | Category | Component | Disposition | Mitigation Plan |
|-----------|----------|-----------|-------------|-----------------|
| T-260615-01 | Information Disclosure | copy_selection_or_line() | accept | Clipboard content is user-authored; no secrets flow automatically |
| T-260615-02 | Tampering | cut_selection_or_line() | accept | Operates only on user-opened file in editor; no privilege escalation path |
</threat_model>

<verification>
Manual test sequence after cargo gates pass:

1. `cargo run` — open a Markdown or text file, press `E` to enter Edit mode.
2. **Cursor visible:** confirm the cursor blinks using the terminal's native cursor (not a reversed span). Test in Terminus if available.
3. **Ctrl+C without selection:** move cursor to any line, press Ctrl+C, paste in an external editor — the current line should paste (with newline).
4. **Ctrl+C with selection:** Shift+Down to select, Ctrl+C, paste elsewhere — selected text pastes.
5. **Ctrl+X without selection:** Ctrl+X on a line — line is removed from editor, clipboard contains the line.
6. **Ctrl+X with selection:** Shift+Arrow to select, Ctrl+X — selection removed, clipboard contains selected text.
7. **Ctrl+V:** paste works (unchanged).
8. **Ctrl+Z:** type some text, Ctrl+Z undoes it.
9. **F3/F5/F6/F8 in Edit mode:** F3 → maximizes Preview (global). F5 → toggles LazyGit. F6 → toggles Terminal. F8 → opens Settings. None interfere with editor.
10. **Ctrl+X in ReadOnly Preview:** press Esc to exit Edit mode, then Ctrl+X on a Markdown file → export chooser appears.
11. **Ctrl+X in FileBrowser:** navigate to a directory, Ctrl+X → folder batch-export chooser appears.
</verification>

<success_criteria>
- cargo build, cargo test, cargo fmt --check, cargo clippy all pass cleanly
- Cursor is visible in Edit mode with focus (hardware cursor, terminal-native)
- Standard Ctrl+C/X/V/Z work in Edit mode; line fallback works without selection
- No shortcut collision with F3/F5/F6/F8
- Export dialog still accessible from ReadOnly and FileBrowser via Ctrl+X
- Version is 0.97.0 in Cargo.toml and SKILL.md
</success_criteria>

<output>
Create `.planning/quick/260615-jwb-editor-copy-cut-paste-shortcuts-and-curs/260615-jwb-SUMMARY.md` when done.

Commit message: `[CHG] v0.97.0: standard copy/cut/paste/undo + hardware cursor in Edit mode`

Then push:
```bash
git push origin main && git push upstream main
git tag v0.97.0
git push origin v0.97.0 && git push upstream v0.97.0
```
</output>
