# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

A Rust-based TUI (Terminal User Interface) multiplexer called "claude-workbench" that provides an integrated development environment with:
- File browser with preview pane
- Multiple embedded PTY terminals (Claude Code, LazyGit, User Terminal)
- Mouse and keyboard navigation
- Scrollback support for terminal panes


Built with Ratatui (TUI framework), Crossterm (terminal handling), and portable-pty (pseudo-terminal).

## Git Push Strategy

**IMPORTANT: This repository uses dual-remote push strategy.** Always push to both remotes:

```bash
git push origin main      # GitLab: gitlab.ownerp.io
git push upstream main    # GitHub: github.com/eqms/claude-workbench.git
```

Both repositories must be kept in sync for all commits. This ensures the project is available as Open Source on GitHub with pre-built binaries via GitHub Actions.

**Remotes:**
| Remote | URL | Purpose |
|--------|-----|---------|
| origin | git@gitlab.ownerp.io:ki/workbench.git | Primary development |
| upstream | git@github.com:eqms/claude-workbench.git | Open Source distribution |

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
- `file_browser.rs` - File browser rendering with git status colors
- `preview.rs` - File preview with syntax highlighting and markdown rendering
- `terminal_pane.rs` - Renders PTY output using vt100 screen cells
- `footer.rs` - Status bar with shortcuts, date/time, and version
- `help.rs` - Help overlay screen
- `about.rs` - About dialog with license info
- `settings.rs` - Settings menu
- `wizard_ui.rs` - Setup wizard
- `fuzzy_finder.rs` - Ctrl+P file finder
- `syntax.rs` - Syntax highlighting (syntect integration)
- `drag_ghost.rs` - Drag & drop visual feedback
- `claude_startup.rs` - Claude startup prefix dialog

## Browser Module (`src/browser/`)

- `opener.rs` - Platform-specific file opening (open/xdg-open/start)
- `markdown.rs` - Markdown to HTML conversion with styled template

## Recent Features (v0.10)

### Footer Date/Time Display
Footer now shows current date/time (DD.MM.YYYY HH:MM:SS) alongside version number.

### File Modification Date
File browser status bar shows modification date for selected files (DD.MM.YYYY HH:MM).

### Browser Preview (`o` key)
- HTML/HTM: Direct browser opening
- Markdown: Converts to styled HTML with dark mode support
- PDF: Opens in default PDF viewer
- Images: PNG/JPG/GIF/SVG/WebP in system viewer
- `O` (Shift+O): Open directory in Finder/file manager

### Git Status Integration
- Color-coded file status (untracked, modified, staged, ignored, conflict)
- Branch name and change counts in status bar
- Directory status aggregation

### Terminal Selection Mode (Ctrl+S)
Select and copy terminal output lines to Claude as code blocks.

### Environment Inheritance
PTY processes now inherit all parent environment variables (critical for Claude CLI which needs HOME, PATH, LANG, etc.).

## Update-System Testing

The application includes a self-update mechanism that downloads new versions from GitHub Releases. This section documents how to test the update system.

### CLI Options for Update Testing

```bash
# Check for updates without starting the TUI
./claude-workbench --check-update

# Simulate older version to trigger update availability
./claude-workbench --check-update --fake-version 0.37.0

# Update to a specific version (for testing/downgrade)
./claude-workbench --update-to v0.38.5

# Or without 'v' prefix - both formats work
./claude-workbench --update-to 0.38.5
```

### Testing Methods

**Method 1: Downgrade and Re-update (Recommended)**

This tests the full update flow without releasing new versions:

```bash
# 1. Check current version
./target/release/claude-workbench --check-update

# 2. Downgrade to an older version
./target/release/claude-workbench --update-to v0.38.5

# 3. Start app - should detect newer version available
./target/release/claude-workbench

# 4. In Help screen (F12), press 'u' to trigger update
```

**Method 2: Fake Version (Simulated)**

Tests update detection without actual download:

```bash
# Simulates running an older version
./target/release/claude-workbench --fake-version 0.37.0

# Update check will find "newer" version, but binary isn't actually older
```

### TUI Update Triggers

- **Automatic**: Update check runs at startup (errors are silent)
- **Manual**: Press `u` in the Help screen (F12) to trigger check
- **Dialog**: If update available, shows version and release notes

### Log File

Update operations write detailed logs for debugging:

```bash
# View update log
cat /tmp/claude-workbench-update.log

# Watch log in real-time
tail -f /tmp/claude-workbench-update.log
```

### Troubleshooting

1. **"No releases found"**: Check that GitHub Release has assets for your platform
2. **Network errors**: Check internet connectivity and GitHub API accessibility
3. **Permission denied**: The binary must be writable for self-update to work
4. **Version mismatch**: Use `--check-update` to verify GitHub release versions

### GitHub Release Requirements

For updates to work, GitHub Releases must include:
- Tag format: `vX.Y.Z` (e.g., `v0.38.6`)
- Binary assets named: `claude-workbench-{target}.tar.gz`
- Supported targets:
  - `aarch64-apple-darwin` (macOS Apple Silicon)
  - `x86_64-apple-darwin` (macOS Intel)
  - `aarch64-unknown-linux-gnu` (Linux ARM64)
  - `x86_64-unknown-linux-gnu` (Linux x64)