# Implementation Plan - Phase 2: PTY Integration

## Goal
Implement fully functional terminal panes using `portable-pty` for process management and `tui-term` + `vt100` for rendering.

## Changes

### Dependencies
- Added `tui-term` and `vt100`.

### Core Logic (`src/terminal.rs` - NEW)
- Create `struct PseudoTerminal`:
  - Holds `portable_pty::PtyPair`.
  - Holds `vt100::Parser`.
  - Runs a background task/thread to read from PTY master → write to Parser.
  - method `resize(cols, rows)`.
  - method `write(input)`.

### App State (`src/app.rs`)
- Add `terminals: HashMap<PaneId, PseudoTerminal>` or similar struct to `App`.
- Initialize 3 terminals on startup:
  1. Claude Code (starts default shell or specific cmd).
  2. LazyGit (starts lazygit or fallback).
  3. Terminal (starts fish/default shell).

### UI (`src/ui/terminal_pane.rs`)
- Use `tui_term::widget::PseudoTerminal` (or manual implementation using the parser) to render the content.
- Actually `tui-term` provides the `PseudoTerminal` widget which takes a parser.

### Input Handling
- In `app.rs`, if focus is on a terminal pane, forward key events to that terminal's PTY.

## Verification
- Run app.
- Verify 3 panes show shell/lazygit output.
- Type commands in them.
