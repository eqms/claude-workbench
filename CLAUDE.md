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
- Click and drag in Terminal/Preview panes for character-level text selection (auto-copies to clipboard on release)

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

## Recent Features (v0.41.0)

### Character-Level Mouse Selection
- Click and drag in Terminal or Preview panes to select text character-by-character
- Selection is constrained to pane boundaries (prevents overflow into adjacent panes)
- Selection automatically copies to system clipboard on mouse release
- Yellow highlight (`LightYellow` background with `Black` text) shows selected characters
- Works in both Terminal panes (Claude, LazyGit, Terminal) and Preview pane (ReadOnly mode)

### Edit Mode Clipboard Integration
- Block operations (Ctrl+F5 copy, Ctrl+F6 cut) now copy to system clipboard in addition to internal buffer
- Text can be pasted into external applications after copying in edit mode

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

<!-- GSD:project-start source:PROJECT.md -->
## Project

**claude-workbench**

A Rust-based TUI (Terminal User Interface) multiplexer that gives developers an integrated cockpit for Claude Code workflows: file browser with preview, three embedded PTY panes (Claude Code, LazyGit, system terminal), mouse + keyboard navigation, and scrollback. Built with Ratatui, Crossterm, and portable-pty. Currently at v0.89.0, distributed as a single binary via GitHub Releases with self-update.

**Core Value:** Stay in one terminal: file navigation, Claude Code, LazyGit, and a shell side-by-side, with the panes always pointing at the same working directory. If everything else fails, the three PTY panes must remain reliably interactive and synchronized.

### Constraints

- **Tech stack**: Rust 2021, Ratatui 0.30, Crossterm 0.28.1 (pinned), portable-pty, vt100, tokio multi-thread — locked unless an Active phase explicitly addresses migration
- **Platform**: Linux + macOS only (XRDP and Kitty are first-class targets due to known compatibility work)
- **Distribution**: Single binary via GitHub Releases (eqms/claude-workbench); GitLab (origin) and GitHub (upstream) must stay in sync
- **Compatibility**: Existing config.yaml format must be preserved or migrated transparently — users rely on persistent settings
- **Performance**: 16ms event-loop polling target; PTY reader threads must never block UI; clipboard work stays off the UI thread
<!-- GSD:project-end -->

<!-- GSD:stack-start source:codebase/STACK.md -->
## Technology Stack

## Languages
- Rust 2021 edition — entire codebase (`src/`)
- YAML — configuration (`config.yaml`, `~/.config/claude-workbench/config.yaml`)
## Runtime
- Native binary, no VM or interpreter
- Single process, multi-threaded (tokio multi-thread runtime + dedicated clipboard worker thread + per-PTY reader threads)
- Cargo (Rust toolchain)
- Lockfile: `Cargo.lock` (present, committed)
## Frameworks
- `ratatui` 0.30.0 — terminal UI framework (widgets, layout, rendering)
- `crossterm` 0.28.1 — cross-platform terminal I/O, keyboard/mouse events, raw mode
- `portable-pty` 0.8.1 — pseudo-terminal (PTY) creation and management for embedded shells
- `vt100` 0.16 — VT100/ANSI escape sequence parser with scrollback buffer (1000 lines default)
- `tokio` 1.44.0 (`rt-multi-thread` feature) — drives the main async event loop
- `clap` 4.5.37 (`derive`, `env` features) — `--check-update`, `--update-to`, `--clipboard-diag`, `--ssh-paste-diag`, `--fake-version` (debug only)
- `tui-textarea` (git: `https://github.com/0xferrous/tui-textarea.git`, branch `update-ratatui`) — inline text editor widget, patched for ratatui 0.30 compatibility
- `tui-markdown` 0.3 — Markdown rendering in TUI panes
- `pulldown-cmark` 0.13 — Markdown parsing (CommonMark)
- `syntect` 5.2 (`default-syntaxes`, `default-themes`, `regex-onig`) — syntax highlighting for file preview pane
- `typst` 0.14.2 — pure-Rust typesetting engine
- `typst-pdf` 0.14.2 — PDF output backend
- `typst-library` 0.14.2 — standard library for typst
- `typst-kit` 0.14.2 (`fonts` feature) — font loading
- `comemo` 0.4 — memoization required by `typst::World` trait
- `ecow` 0.2 — `EcoString`/`EcoVec` types used in typst API
- Feature flag: `pdf-export` (in `[features]`, on by default; disable with `--no-default-features`)
- `serde` 1.0.219 (`derive`) — config struct serialization
- `serde_yaml_ng` 0.9 — YAML config file parsing/writing
- `arboard` 3.6 (`wayland-data-control` feature) — cross-platform clipboard (X11 + Wayland native)
- Subprocess fallback chain (no additional crate): `xclip` → `xsel` → `wl-copy`/`wl-paste` → OSC 52 escape sequence
- Strategy controlled by env var `CLAUDE_WORKBENCH_CLIPBOARD` (`osc52` | `arboard` | `subprocess`)
- `self_update` 0.42 (features: `archive-tar`, `archive-zip`, `compression-flate2`, `compression-zip-deflate`, `rustls`, `signatures`) — downloads and installs GitHub Release assets
- `anyhow` 1.0.98 — error handling and propagation
- `dirs` 5.0 — XDG/platform-aware home/config directory resolution
- `shlex` 1.3 — shell-style argument splitting for PTY command construction
- `regex` 1.12 — pattern matching (git status parsing, file filtering)
- `libc` 0.2 — SIGTSTP suppression via `libc::signal()` (Unix only, one `unsafe` block in `src/main.rs`)
- `tempfile` 3 — temporary files for update staging
## Key Dependencies
- `ratatui` 0.30.0 — entire UI layer
- `crossterm` 0.28.1 — terminal raw mode, events; version pinned, do not bump without updating `tui-textarea`
- `portable-pty` 0.8.1 — embedded PTY shells (Claude, LazyGit, Terminal panes)
- `vt100` 0.16 — PTY output rendering
- `tokio` 1.44.0 — async main loop
- `crossterm` — must stay at 0.28.x; `tui-textarea` fork branch `update-ratatui` imports `crossterm 0.28` event types. Bumping to 0.29 breaks `editor.input(Event::Key(...))` call sites.
- `tui-textarea` — sourced from git fork, not crates.io. `Cargo.lock` must be committed.
- `self_update` 0.42 — GitHub Releases API, binary download, extraction, atomic self-replace
- `arboard` 3.6 — clipboard; `wayland-data-control` feature needed for Wayland sessions
## Configuration
- `Cargo.toml` — single source of truth for dependencies
- Features: `default = ["pdf-export"]`; disable Typst PDF with `cargo build --no-default-features`
## Platform Requirements
- Rust stable toolchain (2021 edition)
- `cargo build` / `cargo run`
- `cargo clippy`, `cargo fmt`, `cargo test`
- X11 session: `xclip` or `xsel` recommended (clipboard over XRDP)
- Wayland session: `wl-clipboard` package (`wl-copy`, `wl-paste`)
- `lazygit` binary on `$PATH` (LazyGit pane)
- `claude` CLI on `$PATH` (Claude pane, if `pty.claude_command` not set)
- `arboard` uses native pasteboard — no external clipboard tools needed
- `open` command used for browser/file opening (`src/browser/opener.rs`)
- GitHub Releases: `claude-workbench-{target}.tar.gz`
- Supported targets: `aarch64-apple-darwin`, `x86_64-apple-darwin`, `aarch64-unknown-linux-gnu`, `x86_64-unknown-linux-gnu`
- Self-update: binary downloads and replaces itself at runtime
<!-- GSD:stack-end -->

<!-- GSD:conventions-start source:CONVENTIONS.md -->
## Conventions

## Naming Patterns
- `snake_case` for all module files: `job_state.rs`, `file_browser.rs`, `terminal_pane.rs`
- Descriptive compound names preferred over abbreviations: `dependency_checker.rs`, `release_notes.rs`
- Module groups use `mod.rs` as entry point: `src/app/mod.rs`, `src/git/mod.rs`, `src/update/mod.rs`
- `CamelCase` universally: `PseudoTerminal`, `ClipboardOutcome`, `GitFileStatus`, `JobState<T>`
- Enum variants: `CamelCase` — `ClipboardOutcome::Arboard`, `PollOutcome::Ready(T)`, `GitFileStatus::Untracked`
- Generic parameters: single uppercase letter `T`
- `snake_case` universally: `filter_lines()`, `find_repo_root()`, `detect_strategy()`
- Boolean predicates prefixed `is_` or `has_`: `is_running()`, `is_yolo()`, `has_meaningful_selection()`
- Constructors use `new()` or descriptive names: `UpdateState::new()`, `JobState::running(rx)`
- Helper functions that return static `&str`: `name()`, `label()`, `symbol()`
- `snake_case` universally
- `_` prefix for intentionally unused: `_path` in `set_restrictive_permissions(_path: &Path)`
- Descriptive over short: `consecutive_blanks`, `in_traceback`, not `n` or `flag`
- `SCREAMING_SNAKE_CASE`: `SUBPROCESS_TIMEOUT`, `WAIT_POLL_INTERVAL`, `STRATEGY_ENV`, `CURRENT_VERSION`
- `pub(crate)` constants for cross-module sharing: `REPO_OWNER`, `REPO_NAME`, `BIN_NAME` in `src/update/mod.rs`
- `snake_case` directory/file names
- Public API re-exported at module root via `pub use`: `src/update/mod.rs` re-exports all submodule items
## Code Style
- Tool: `rustfmt` with `rustfmt.toml`
- `edition = "2021"`
- `reorder_imports = true`
- `reorder_modules = true`
- `newline_style = "Unix"`
- No explicit line-length override — Rust edition 2021 default (100 chars)
- Tool: `clippy` with `clippy.toml`
- `cognitive-complexity-threshold = 30` (relaxed from default 25)
- `too-many-arguments-threshold = 8` (relaxed from default 7)
- `type-complexity-threshold = 300` (relaxed from default 250)
- `#[allow(clippy::type_complexity)]` used sparingly for `MouseSelection::finish()` return type
## Import Organization
#[cfg(unix)]
## Error Handling
## Logging
- `println!` used only in CLI diagnostic modes (`--check-update`, `--clipboard-diag`) in `src/main.rs`
- TUI operation: no stdout/stderr logging during normal operation (would corrupt terminal rendering)
- Update operations write to `/tmp/claude-workbench-update.log` via `src/update/log.rs`
- Errors that cannot be surfaced to the user are silently dropped (by design — TUI constraint)
## Comments
## Function Design
- Fallible operations: `Result<T>` (anyhow) or `Option<T>`
- Infallible state mutation: `()` (methods on state structs)
- Boolean queries: `bool`
## Module Design
- Public API declared with `pub` at item level
- Crate-internal sharing uses `pub(crate)`
- Re-export at module root via `pub use submodule::Item` for clean external API
## Concurrency Patterns
<!-- GSD:conventions-end -->

<!-- GSD:architecture-start source:ARCHITECTURE.md -->
## Architecture

## System Overview
```text
```
## Component Responsibilities
| Component | Responsibility | File |
|-----------|----------------|------|
| `App` | Master state, event loop, sub-module dispatch | `src/app/mod.rs` |
| `PseudoTerminal` | PTY lifecycle, background reader thread, vt100 parse | `src/terminal.rs` |
| `JobState<T>` | Typed async job lifecycle (Idle/Running/poll) | `src/app/job_state.rs` |
| `compute_layout` | Maps terminal Rect into 6 pane Rects | `src/ui/layout.rs` |
| `keyboard/` | Key event dispatch, split by context | `src/app/keyboard/mod.rs` + submodules |
| `mouse.rs` | Mouse event dispatch, hit-test, drag/select | `src/app/mouse.rs` |
| `drawing.rs` | Full-frame render orchestration | `src/app/drawing.rs` |
| `clipboard.rs` | 5-stage fallback chain, async worker thread | `src/clipboard.rs` |
| `config.rs` | YAML config load/save, struct definitions | `src/config.rs` |
| `update/` | GitHub release check, binary self-replace | `src/update/` |
## Pattern Overview
- `App` is a single monolithic struct (≈165 fields). All handler modules implement `App` via `impl App` blocks, not separate structs.
- PTY output is produced on N background threads (one per pane); main thread consumes via `Arc<Mutex<vt100::Parser>>` during `draw()`.
- Async jobs (update check, git remote check, PDF export) use `std::sync::mpsc` channels wrapped in `JobState<T>`, polled each event-loop iteration with `try_recv()`. No blocking inside the loop.
- 16ms poll timeout gives ~60fps UI refresh rate.
## Layers
- Purpose: Parse args, handle non-TUI modes, bootstrap tokio runtime
- Location: `src/main.rs`
- Contains: `Args` (clap), `run_update_check_cli`, `run_clipboard_diag_cli`, `run_ssh_paste_diag_cli`, `async_main`
- Depends on: `update`, `clipboard`, `session`, `config`, `app`
- Purpose: Own all application state, run the event loop
- Location: `src/app/mod.rs`
- Contains: `App` struct, `App::new()`, `App::run()`
- Depends on: all other layers
- Used by: `main.rs` only
- Purpose: Translate crossterm events into App mutations
- Location: `src/app/keyboard/` (5 submodules), `src/app/mouse.rs`
- Contains:
- Depends on: `terminal.rs`, `types.rs`, `clipboard.rs`
- Purpose: Manage subprocess lifecycle and terminal emulation
- Location: `src/terminal.rs`
- Contains: `PseudoTerminal`, `PtyCallbacks` (DSR/CPR/DA response handler)
- Shared state: `Arc<Mutex<vt100::Parser<PtyCallbacks>>>` read by UI, written by background thread
- Writer: `Arc<Mutex<Box<dyn Write + Send>>>` — locked only during `write_input()`
- Exit detection: `Arc<AtomicBool>` set by reader thread on EOF
- Depends on: `portable-pty`, `vt100`
- Purpose: Non-blocking background work that delivers a single result
- Location: `src/app/job_state.rs`, used in `src/app/update.rs`, `src/app/git_ops.rs`, `src/app/clipboard.rs`
- Contains: `JobState<T>` enum (`Idle` | `Running(Receiver<T>)`), `PollOutcome<T>` enum
- Pattern: spawn `std::thread::spawn` → send on `mpsc::Sender<T>` → `App::run()` calls `poll()` each loop
- Active jobs on `App`: `git_check_job`, `update_check_job`, `update_job`, `export_job`
- Purpose: Stateless frame rendering from App state
- Location: `src/ui/`
- Contains: one file per widget/pane (see STRUCTURE.md)
- Depends on: `ratatui`, `vt100` (reads parser screen), `syntect` (syntax highlighting)
- Called by: `src/app/drawing.rs` once per loop iteration
- `src/clipboard.rs` — 5-stage fallback: arboard → xclip → xsel → wl-copy → OSC 52; async worker thread for copy, sync path for diagnostics
- `src/config.rs` — YAML via `serde_yaml_ng`, search paths: `./config.yaml` → `~/.config/claude-workbench/config.yaml`
- `src/git/mod.rs` — git status queries for file browser coloring and remote-ahead detection
- `src/update/` — GitHub Releases API via `self_update` crate, self-replace binary on disk
- `src/session.rs` — session persistence (currently returns defaults)
- `src/filter.rs` — file name filtering for fuzzy finder
- `src/syntax_registry.rs` — syntect `SyntaxSet` singleton
## Data Flow
### Primary Event Loop Iteration
### PTY Output Path
### PTY Input Path
### Async Job Pattern (e.g. update check)
### PTY Resize
### Directory Sync
## Key Abstractions
- Purpose: Wraps portable-pty + vt100 parser into a single owned handle
- Fields: `parser: Arc<Mutex<vt100::Parser<PtyCallbacks>>>`, `writer: Arc<Mutex<Box<dyn Write+Send>>>`, `master: Box<dyn MasterPty+Send>`, `exited: Arc<AtomicBool>`
- Pattern: background thread shares `Arc` clones; main thread accesses via `lock_or_recover()` (poison-safe)
- Instances: up to 3, keyed by `PaneId` in `App::terminals: HashMap<PaneId, PseudoTerminal>`
- Purpose: Explicit lifecycle for single-shot async jobs replacing `Option<Receiver<T>>`
- States: `Idle` (no job) | `Running(Receiver<T>)` (in flight)
- `poll()` returns `PollOutcome::{Pending, Ready(T), Disconnected}` and auto-resets to `Idle`
- Used for: `git_check_job: JobState<GitRemoteCheckResult>`, `update_check_job: JobState<UpdateCheckResult>`, `update_job: JobState<UpdateResult>`, `export_job: JobState<Result<PathBuf, String>>`
- Purpose: Bundle of 6 `Rect` values recomputed per mouse event to hit-test pane clicks
- Fields: `files`, `preview`, `claude`, `lazygit`, `terminal`, `footer`
- Purpose: Enumerate which fallback stage succeeded or why all failed
- Values: `Arboard | Xclip | Xsel | WlCopy | Osc52 | Failed(String) | Submitted`
- `Submitted` is returned immediately when copy is queued to async worker; real outcome arrives later via `take_pending_outcome()`
## Entry Points
- Location: `src/main.rs` → `async_main()` → `App::new()` + `App::run()`
- Triggers: normal `cargo run` / binary invocation without special flags
- `--check-update` → `run_update_check_cli()`
- `--update-to <version>` → `run_update_to_version_cli()`
- `--clipboard-diag` → `run_clipboard_diag_cli()`
- `--ssh-paste-diag` → `run_ssh_paste_diag_cli()`
## Architectural Constraints
- **Threading:** Single UI thread. PTY reader threads (one per pane) share state only via `Arc<Mutex>` and `Arc<AtomicBool>`. Async jobs run on `std::thread` (not tokio tasks) and communicate via `mpsc`. Tokio runtime is present but used only for the outer `block_on`; internal async is avoided in the event loop.
- **Global state:** `OnceLock<Mutex<Sender<ClipboardJob>>>` in `src/clipboard.rs` holds the async clipboard worker sender. `SyntaxSet` in `src/syntax_registry.rs` is effectively a module-level singleton. No other global mutable state.
- **Circular imports:** None observed. `app/` depends on `terminal`, `ui`, `clipboard`, `config`, `types`, `git`, `update`, `setup`. `ui/` depends only on `types` and `config`.
- **Paste handling:** Claude pane receives raw bytes (no bracketed-paste wrapping); LazyGit and Terminal panes receive `\x1b[200~{text}\x1b[201~`. This asymmetry is by design — Claude CLI does not support bracketed paste.
- **PTY auto-restart:** When `PseudoTerminal::exited` is true, `check_and_restart_exited_ptys()` respawns using the same command. Configurable via `config.pty.auto_restart`.
## Anti-Patterns
### Monolithic `App` struct
### `lock_or_recover` poison suppression
## Error Handling
- PTY spawn failure: stored in `App::claude_error` / `lazygit_error` / `terminal_error`, shown as pane overlay
- PTY exit: `Arc<AtomicBool>` set by reader thread; `check_and_restart_exited_ptys()` respawns
- Clipboard failure: `ClipboardOutcome::Failed(msg)` triggers `clipboard_error_flash` footer banner (3s)
- Async job disconnect: `PollOutcome::Disconnected` resets job to `Idle`, UI silently returns to previous state
## Cross-Cutting Concerns
<!-- GSD:architecture-end -->

<!-- GSD:skills-start source:skills/ -->
## Project Skills

No project skills found. Add skills to any of: `.claude/skills/`, `.agents/skills/`, `.cursor/skills/`, `.github/skills/`, or `.codex/skills/` with a `SKILL.md` index file.
<!-- GSD:skills-end -->

<!-- GSD:workflow-start source:GSD defaults -->
## GSD Workflow Enforcement

Before using Edit, Write, or other file-changing tools, start work through a GSD command so planning artifacts and execution context stay in sync.

Use these entry points:
- `/gsd-quick` for small fixes, doc updates, and ad-hoc tasks
- `/gsd-debug` for investigation and bug fixing
- `/gsd-execute-phase` for planned phase work

Do not make direct repo edits outside a GSD workflow unless the user explicitly asks to bypass it.
<!-- GSD:workflow-end -->

<!-- GSD:profile-start -->
## Developer Profile

> Profile not yet configured. Run `/gsd-profile-user` to generate your developer profile.
> This section is managed by `generate-claude-profile` -- do not edit manually.
<!-- GSD:profile-end -->
