# Codebase Structure

**Analysis Date:** 2026-05-11

## Directory Layout

```
workbench/
├── src/
│   ├── main.rs                  # Binary entry point, CLI args, tokio bootstrap
│   ├── app/                     # App struct + all event handlers
│   │   ├── mod.rs               # App struct definition, App::new(), App::run()
│   │   ├── job_state.rs         # JobState<T> generic for async mpsc jobs
│   │   ├── keyboard/            # Key event dispatch (split by context)
│   │   │   ├── mod.rs           # handle_key_event() priority chain, pane focus keys
│   │   │   ├── dialogs.rs       # Overlay key handlers (fuzzy, update, wizard, settings, etc.)
│   │   │   ├── global.rs        # Pane-independent shortcuts (F7–F12, Ctrl+P/O/X/E, F8)
│   │   │   ├── file_browser.rs  # File browser key handler (j/k/l/h/enter/backspace)
│   │   │   ├── preview.rs       # Preview pane (search, tui-textarea edit mode, read-only)
│   │   │   └── terminal.rs      # Terminal pane pass-through, scrollback (Shift+PgUp/Dn)
│   │   ├── mouse.rs             # Mouse event dispatch, hit-test, drag/select state
│   │   ├── drawing.rs           # Full-frame render: calls compute_layout + all ui widgets
│   │   ├── file_ops.rs          # File browser operations (open, rename, delete, copy)
│   │   ├── git_ops.rs           # Git remote check job, branch/remote state mutations
│   │   ├── clipboard.rs         # App-level clipboard glue: poll_clipboard_outcome()
│   │   ├── pty.rs               # PTY spawn helpers, check_and_restart_exited_ptys(), lazy-init
│   │   ├── ssh_paste.rs         # Ctrl+V image-paste over SSH via cc-clip
│   │   └── update.rs            # Self-update job management, poll_update_check/result
│   ├── terminal.rs              # PseudoTerminal: portable-pty + vt100 + background reader thread
│   ├── clipboard.rs             # 5-stage fallback clipboard (arboard/xclip/xsel/wl-copy/OSC52)
│   ├── config.rs                # Config struct (serde YAML), load_config(), save_config()
│   ├── types.rs                 # Shared enums/structs: PaneId, ClaudePermissionMode, DragState, etc.
│   ├── input.rs                 # key_event_to_bytes(): crossterm KeyEvent → PTY byte sequences
│   ├── filter.rs                # Fuzzy file name filtering for Ctrl+P finder
│   ├── session.rs               # Session persistence (load_session / save_session)
│   ├── syntax_registry.rs       # syntect SyntaxSet/ThemeSet singleton loader
│   ├── app_detector.rs          # Detect running apps (e.g. Claude CLI availability check)
│   ├── ui/                      # Stateless render widgets (ratatui)
│   │   ├── mod.rs               # Re-exports, shared render helpers
│   │   ├── layout.rs            # compute_layout(): maps Rect → 6 pane Rects (configurable %)
│   │   ├── terminal_pane.rs     # Renders vt100::Screen cells into ratatui buffer
│   │   ├── file_browser.rs      # File list with git-status colors, status bar
│   │   ├── preview.rs           # File preview: syntax-highlighted code, markdown, edit mode
│   │   ├── footer.rs            # Status bar: shortcuts, date/time, version, flash messages
│   │   ├── help.rs              # F12 help overlay: shortcuts + dependency status
│   │   ├── about.rs             # About dialog (license, version)
│   │   ├── settings.rs          # Settings menu (toggle panes, theme, etc.)
│   │   ├── fuzzy_finder.rs      # Ctrl+P fuzzy file finder overlay
│   │   ├── menu.rs              # Menu bar
│   │   ├── dialog.rs            # Generic modal dialog (yes/no confirm)
│   │   ├── update_dialog.rs     # Update available dialog with release notes
│   │   ├── claude_startup.rs    # Claude startup prefix chooser dialog
│   │   ├── permission_mode.rs   # Claude permission mode selection dialog
│   │   ├── wizard_ui.rs         # First-run setup wizard render
│   │   ├── drag_ghost.rs        # Drag-and-drop visual ghost overlay
│   │   └── syntax.rs            # SyntaxManager wrapper used by preview rendering
│   ├── browser/                 # External file/browser opening
│   │   ├── mod.rs               # Re-exports
│   │   ├── opener.rs            # Platform open(): open/xdg-open/start dispatch
│   │   ├── markdown.rs          # Markdown → styled HTML (for browser preview)
│   │   ├── template.rs          # HTML template for markdown browser preview
│   │   ├── syntax.rs            # Syntax-highlighted HTML export
│   │   ├── pdf_export.rs        # Typst-based PDF export orchestration
│   │   └── typst_pdf.rs         # Low-level typst World impl for PDF rendering
│   ├── git/
│   │   └── mod.rs               # Git status queries (file colors, remote-ahead count)
│   ├── update/                  # Self-update subsystem
│   │   ├── mod.rs               # Re-exports, CURRENT_VERSION const, restart_application()
│   │   ├── check.rs             # check_for_update_with_version(): GitHub Releases API
│   │   ├── install.rs           # Binary download, verify, atomic replace
│   │   ├── log.rs               # Update log file path + append helpers
│   │   ├── release_notes.rs     # Parse release notes from GitHub response
│   │   ├── state.rs             # UpdateState struct (show_dialog, available_version, etc.)
│   │   └── version.rs           # Version comparison utilities
│   └── setup/                   # First-run setup
│       ├── mod.rs               # DependencyReport: checks for lazygit, xclip, etc.
│       ├── dependency_checker.rs # Binary presence checks, platform detection
│       └── wizard.rs            # WizardState: step-by-step first-run wizard logic
├── Cargo.toml                   # Package manifest, feature flags (pdf-export default)
├── Cargo.lock                   # Lockfile (committed)
├── clippy.toml                  # Clippy lint configuration
├── rustfmt.toml                 # rustfmt formatting configuration
├── config.yaml                  # Optional local config override (highest priority)
└── CLAUDE.md                    # Architecture docs for AI assistants
```

## Directory Purposes

**`src/app/`:**
- Purpose: All application behavior. `App` struct owns everything; submodules add `impl App` blocks.
- Contains: State struct, event loop, all input handlers, async job management
- Key files: `mod.rs` (struct + run loop), `job_state.rs` (async pattern), `keyboard/mod.rs` (dispatch)

**`src/ui/`:**
- Purpose: Pure rendering — no state mutations, only reads from `App` (or sub-state passed by ref)
- Contains: One file per visual component; `layout.rs` is the single source of truth for pane geometry
- Key files: `layout.rs`, `terminal_pane.rs`, `footer.rs`

**`src/browser/`:**
- Purpose: Everything that involves opening files in the system browser or converting to HTML/PDF
- Contains: Platform open dispatch, markdown→HTML, syntax→HTML, Typst PDF generation
- Key files: `opener.rs`, `pdf_export.rs`

**`src/update/`:**
- Purpose: Self-update from GitHub Releases. Fully contained subsystem.
- Contains: API check, download/install, version compare, update log, dialog state
- Key files: `check.rs`, `install.rs`, `state.rs`

**`src/setup/`:**
- Purpose: First-run experience — dependency detection and setup wizard
- Contains: Binary checks (lazygit, clipboard helpers), wizard step state
- Key files: `dependency_checker.rs`, `wizard.rs`

**`src/git/`:**
- Purpose: Git status queries used by file browser coloring and remote-ahead detection
- Key files: `mod.rs`

## Key File Locations

**Entry Points:**
- `src/main.rs`: Binary entry, CLI arg parsing (clap), tokio runtime, terminal init/restore
- `src/app/mod.rs`: `App::new()` spawns PTYs; `App::run()` is the event loop

**Configuration:**
- `src/config.rs`: `Config` struct with all sub-configs; `load_config()` / `save_config()`
- `config.yaml` (project root): Local override, highest priority
- `~/.config/claude-workbench/config.yaml`: User-level config

**Core State Types:**
- `src/types.rs`: `PaneId`, `ClaudePermissionMode`, `ClaudeModel`, `ClaudeEffort`, `DragState`, `MouseSelection`, `TerminalSelection`, `ResizeState`, `BorderAreas`, `ScrollbarAreas`, `ExportChooserState`

**PTY / Terminal:**
- `src/terminal.rs`: `PseudoTerminal` struct — the only place portable-pty is used
- `src/app/pty.rs`: Spawn helpers, lazy-init, auto-restart logic
- `src/input.rs`: Key → byte sequence translation for PTY input

**Async Jobs:**
- `src/app/job_state.rs`: `JobState<T>` — use this for every new async job

**Clipboard:**
- `src/clipboard.rs`: All clipboard I/O. `copy_to_clipboard()` (async), `copy_to_clipboard_sync()` (diag), `paste_from_clipboard()`, `ClipboardDiag::collect()`

**Layout:**
- `src/ui/layout.rs`: `compute_layout()` — single function, returns `(Rect, Rect, Rect, Rect, Rect, Rect)`

**Testing:**
- No dedicated `tests/` directory. Tests are inline `#[cfg(test)]` modules within source files.

## Naming Conventions

**Files:**
- `snake_case.rs` throughout
- Handler groupings use the pane name: `file_browser.rs`, `terminal.rs`, `preview.rs`
- Dialog/overlay UIs suffixed `_dialog.rs` or `_ui.rs`

**Directories:**
- `snake_case/` with `mod.rs` entry point
- Feature subsystems get their own directory: `update/`, `setup/`, `browser/`, `git/`

**Types:**
- Structs: `PascalCase` — `PseudoTerminal`, `JobState`, `LayoutRects`
- Enums: `PascalCase` with `PascalCase` variants — `PaneId::Claude`, `PollOutcome::Ready`
- Config structs: `FooConfig` pattern — `UiConfig`, `PtyConfig`, `ClaudeConfig`, `LayoutConfig`

**Functions:**
- `snake_case`; handler methods prefixed `handle_`: `handle_key_event`, `handle_mouse_event`
- Poll methods prefixed `poll_`: `poll_git_check`, `poll_update_check`, `poll_clipboard_outcome`
- Async job starters prefixed `start_`: `start_update_check`

## Where to Add New Code

**New pane or UI overlay:**
- Render widget: `src/ui/<name>.rs`
- State struct: inline in `src/ui/<name>.rs` or in `src/types.rs` if shared
- Add field to `App` in `src/app/mod.rs`
- Wire keyboard: add handler method in `src/app/keyboard/dialogs.rs` (if modal) or appropriate submodule
- Register in render: `src/app/drawing.rs`

**New async background job:**
- Add `JobState<MyResult>` field to `App` in `src/app/mod.rs`
- Add `start_foo()` and `poll_foo()` methods via new or existing `impl App` block
- Call `poll_foo()` in `App::run()` loop (before `terminal.draw`)
- Use `std::sync::mpsc::channel()` + `std::thread::spawn`; see `src/app/update.rs` for reference

**New PTY pane:**
- Add variant to `PaneId` in `src/types.rs`
- Spawn in `App::new()` in `src/app/mod.rs`
- Add error field `foo_error: Option<String>` to `App`
- Add Rect to `LayoutRects` and `compute_layout()` return
- Add render call in `src/app/drawing.rs`
- Handle focus switch in `src/app/keyboard/mod.rs`

**New config option:**
- Add field to appropriate `FooConfig` struct in `src/config.rs`
- Add `#[serde(default)]` and a `default_foo()` fn
- Access via `self.config.section.field` in any `impl App` block

**New keyboard shortcut:**
- Global (any pane): `src/app/keyboard/global.rs`
- Pane-specific: matching submodule in `src/app/keyboard/`
- Dialog/overlay: `src/app/keyboard/dialogs.rs`

**Shared helper utilities:**
- Type definitions: `src/types.rs`
- Pure functions with no App dependency: new module at `src/` top level

## Special Directories

**`.planning/`:**
- Purpose: AI planning documents (this file)
- Generated: No
- Committed: Yes (planning artifacts)

**`target/`:**
- Purpose: Cargo build output
- Generated: Yes
- Committed: No

---

*Structure analysis: 2026-05-11*
