# Implementation Plan - Phase 3: File Browser & Preview

## Goal
Implement a functional file browser (directory tree navigation) and a basic preview pane that shows file content.

## Changes

### File Browser Logic (`src/ui/file_browser.rs` + `src/app.rs`)
- **State**: Add `file_browser_state` to `App`.
  - Current directory path.
  - List of entries (files/dirs).
  - Selected index.
- **Logic**:
  - `load_directory(path)` function.
  - Navigation handlers: `move_down`, `move_up`, `enter_directory`, `go_parent`.
- **UI**:
  - Render list of files.
  - Highlight selected file.

### Preview Logic (`src/ui/preview.rs`)
- **State**: Add `preview_state` to `App` (conceptually).
  - content string (or dedicated structure).
- **Logic**:
  - On file selection change (in browser), trigger `load_preview`.
  - Simple `std::fs::read_to_string` for now.
  - Detect binary/large files (limit read size).
- **UI**:
  - Render content to `Paragraph` widget.
  - Handle scrolling (if possible).

### Integration (`src/app.rs`)
- Connect Keyboard `Up/Down/Enter/Left` to File Browser logic when focused.
- Connect File Browser selection change to Preview loader.

## Verification
- Run app.
- Traverse directories.
- See file contents appear in middle pane.
