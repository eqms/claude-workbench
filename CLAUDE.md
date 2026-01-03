# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

A Rust-based TUI (Terminal User Interface) multiplexer called "claude-workbench" that provides an integrated development environment with:
- File browser with preview pane
- Multiple embedded PTY terminals (Claude Code, LazyGit, User Terminal)
- Mouse and keyboard navigation
- Scrollback support for terminal panes


Built with Ratatui (TUI framework), Crossterm (terminal handling), and portable-pty (pseudo-terminal).

## Development Commands

### Build and Run
```bash
# Build the project
cargo build

# Run in development mode
cargo run

# Build release version
cargo build --release

# Run release version
cargo run --release

# Run with custom config
cargo run -- --config path/to/config.yaml
```

### Testing
```bash
# Run all tests
cargo test

# Run specific test
cargo test test_name

# Run tests with output
cargo test -- --nocapture
```

### Code Quality
```bash
# Check code without building
cargo check

# Run clippy linter
cargo clippy

# Format code
cargo fmt

# Check formatting
cargo fmt -- --check
```

## Architecture

### Core Components

**App State (`src/app.rs`)**
- Main application struct holding all state
- Manages 5 panes: FileBrowser, Preview, Claude, LazyGit, Terminal
- Event loop with 16ms polling for responsive UI
- Mouse and keyboard event routing based on active pane
- PTY synchronization: syncs `cd` commands to Terminal and Claude panes when directory changes

**PTY Management (`src/terminal.rs`)**
- `PseudoTerminal` wraps portable-pty with vt100 parser
- Background thread reads PTY output and feeds vt100 parser
- Scrollback support via `vt100::Parser` screen buffer
- Automatic parser reset on user input to scrollback position 0
- PTY resizing syncs terminal size with UI pane dimensions

**Layout System (`src/ui/layout.rs`)**
- Fixed 6-pane layout: Files (left), Preview (top-right), Claude/LazyGit/Terminal (bottom-right), Footer
- Returns 6 `Rect` structures for rendering
- Each terminal pane automatically resizes PTY when dimensions change (accounting for borders: -2px)

**Input Handling (`src/input/mod.rs`)**
- Maps crossterm key events to PTY byte sequences
- Handles special keys: arrows, function keys, modifiers (Ctrl, Alt, Shift)
- Shift+PageUp/Down and Shift+Up/Down for scrollback in terminal panes

### Key Design Patterns

**PTY Threading Model**
Each `PseudoTerminal` spawns a background thread that continuously reads PTY output and updates the shared `Arc<Mutex<vt100::Parser>>`. The main UI thread locks the parser only during rendering.

**Focus Management**
`App::active_pane` tracks which pane has focus. Mouse clicks and F-keys (F1-F6) switch focus. Only the active pane receives keyboard input (except global keys like `?` for help, Ctrl+Q to quit).

**Directory Sync Pattern**
When file browser changes directory, `App::sync_terminals()` sends `cd "path"\r` to Terminal and Claude panes. This keeps shell environments synchronized with the file browser's current working directory.

**Scrollback Auto-Reset**
Terminal panes automatically reset scrollback to 0 when user types (in `PseudoTerminal::write_input`), ensuring typed input appears at the bottom of the screen.

**Mouse Hit Testing**
Mouse events compute which pane was clicked using helper closure `is_inside(rect, x, y)`. This enables click-to-focus and scroll-in-pane behavior.

## Configuration

### Config Files

The application loads configuration from:
1. `./config.yaml` (local directory, highest priority)
2. `~/.config/claude-workbench/config.yaml` (user config)
3. Built-in defaults (fallback)

### Config Structure (`config.yaml`)
```yaml
terminal:
  shell_path: "/bin/bash"
  shell_args: []

ui:
  theme: "default"
```

### Session State

Session persistence is stubbed (`src/session/mod.rs`). Currently returns default state. Designed to save/restore last working directory and other session data.

## Key Keyboard Shortcuts

**Global**
- `?` - Toggle help screen
- `Ctrl+Q` - Quit application
- `F1`-`F6` - Switch focus between panes

**File Browser (F1)**
- `j`/`↓`, `k`/`↑` - Navigate files
- `l`/`→`/`Enter` - Enter directory or open file
- `h`/`←`/`Backspace` - Go to parent directory
- `q` - Quit (when file browser has focus)

**Preview Pane (F2)**
- `j`/`↓`, `k`/`↑` - Scroll preview

**Terminal Panes (F4/F5/F6)**
- `Shift+PageUp/PageDown` - Scroll 10 lines
- `Shift+↑/↓` - Scroll 1 line
- All other keys sent to PTY

**Mouse**
- Click pane to focus
- Scroll wheel to scroll content

## PTY Initialization

The three terminal panes are created in `App::new`:

1. **Claude Code PTY** (`PaneId::Claude`): `/bin/bash -c "echo 'Claude Code PTY'; exec bash"`
2. **LazyGit PTY** (`PaneId::LazyGit`): `lazygit`
3. **User Terminal** (`PaneId::Terminal`): Uses shell from config (default: `/bin/bash`)

All PTYs start in the file browser's current directory. After initialization, Claude and Terminal panes receive `\x0c` (Ctrl+L) to clear screen.

## Important Implementation Notes

**PTY Resize Timing**
PTY resize happens during every `draw()` call before rendering. This ensures terminal dimensions match UI layout even when window resizes.

**vt100 Parser Capacity**
Parser initialized with 1000-line scrollback buffer (`vt100::Parser::new(rows, cols, 1000)`). Increase this value for deeper scrollback history.

**Fish Shell Compatibility**
Environment sets `fish_features=no-query-term` to suppress Fish's DA (Device Attributes) query which can cause rendering artifacts.

**Border Accounting**
Terminal panes have 1px borders on all sides. When resizing PTY, subtract 2 from both width and height to get actual content area.

## UI Module Structure

- `layout.rs` - Computes 6-pane layout rectangles
- `file_browser.rs` - File browser rendering and state
- `preview.rs` - File preview rendering and state
- `terminal_pane.rs` - Renders PTY output using vt100 screen cells
- `footer.rs` - Status bar with keyboard shortcuts
- `help.rs` - Help overlay screen