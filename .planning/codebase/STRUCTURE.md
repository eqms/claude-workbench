# Codebase Structure

**Analysis Date:** 2026-05-11

## Directory Layout

```
workbench/
├── src/
│   ├── main.rs                  # Entry point; CLI dispatch; TUI bootstrap
│   ├── lib.rs                   # (absent — binary crate only)
│   ├── app/                     # Application state and event loop
│   │   ├── mod.rs               # App struct; run() event loop
│   │   ├── job_state.rs         # JobState<T> / PollOutcome<T> generics
│   │   ├── pty.rs               # PTY spawn/sync; quote_path_for_cd(); StartupOptions
│   │   ├── drawing.rs           # draw() — delegates to ui/ modules
│   │   ├── clipboard.rs         # poll_clipboard_outcome(); copy/paste dispatch
│   │   ├── file_ops.rs          # File open, browser preview, export trigger
│   │   ├── git_ops.rs           # Async git-remote check
│   │   ├── mouse.rs             # Mouse event routing; drag/selection state
│   │   ├── ssh_paste.rs         # Ctrl+V SSH image-paste interception
│   │   ├── update.rs            # Poll update jobs; trigger restart
│   │   └── keyboard/            # Key event routing (split by context)
│   │       ├── mod.rs           # handle_key_event() dispatcher
│   │       ├── global.rs        # Ctrl+Q, ?, F-keys, global shortcuts
│   │       ├── file_browser.rs  # j/k/l/h navigation; fuzzy finder
│   │       ├── preview.rs       # Scroll, edit mode, export chooser
│   │       ├── terminal.rs      # PTY key forwarding; Shift+PgUp scrollback
│   │       └── dialogs.rs       # Wizard, settings, permission dialog keys
│   ├── terminal.rs              # PseudoTerminal; Arc<Mutex<vt100::Parser>>; reader thread
│   ├── clipboard.rs             # Copy/paste backends; async worker; which(); is_executable()
│   ├── config.rs                # Config struct; YAML load/save; defaults
│   ├── types.rs                 # Shared enums/structs: PaneId, DragState, MouseSelection, …
│   ├── input.rs                 # Raw crossterm → PTY byte mapping (arrows, Fn keys, modifiers)
│   ├── session.rs               # Session state stub (load/save cwd)
│   ├── filter.rs                # File-browser filtering helpers
│   ├── syntax_registry.rs       # syntect SyntaxSet singleton
│   ├── app_detector.rs          # Detect running app (Claude, LazyGit, shell) in PTY
│   ├── git/
│   │   └── mod.rs               # Git status colors; branch/change-count queries
│   ├── browser/
│   │   ├── mod.rs               # Re-exports
│   │   ├── opener.rs            # open_file(); open_file_with_browser(); validate_program()
│   │   ├── pdf_export.rs        # default_preview_file() RAII; ExportFormat; date_now_dmy()
│   │   ├── markdown.rs          # Markdown → HTML conversion
│   │   ├── syntax.rs            # Syntax-highlight → HTML
│   │   ├── template.rs          # HTML page template (dark mode)
│   │   └── typst_pdf.rs         # Typst PDF rendering (feature = "pdf-export")
│   ├── ui/
│   │   ├── mod.rs               # Re-exports
│   │   ├── layout.rs            # compute_layout() → 6 Rects; configurable percentages
│   │   ├── file_browser.rs      # FileBrowserState; git-colored file list
│   │   ├── preview.rs           # PreviewState; syntax-highlighted content
│   │   ├── terminal_pane.rs     # vt100 screen cell rendering; character selection overlay
│   │   ├── footer.rs            # Status bar: shortcuts, date/time, version, flash messages
│   │   ├── help.rs              # F12 help overlay; dependency report
│   │   ├── fuzzy_finder.rs      # Ctrl+P file finder
│   │   ├── dialog.rs            # Generic modal dialog
│   │   ├── update_dialog.rs     # Update available dialog; button state
│   │   ├── about.rs             # About overlay
│   │   ├── settings.rs          # Settings menu state
│   │   ├── menu.rs              # Menu bar
│   │   ├── permission_mode.rs   # Claude permission mode selection dialog
│   │   ├── claude_startup.rs    # Startup prefix / session name dialog
│   │   ├── wizard_ui.rs         # First-run setup wizard rendering
│   │   ├── drag_ghost.rs        # Drag-and-drop visual ghost
│   │   └── syntax.rs            # SyntaxManager wrapper for ui layer
│   ├── setup/
│   │   ├── mod.rs               # DependencyReport; check() for clipboard helpers
│   │   ├── dependency_checker.rs # Binary presence checks (lazygit, xclip, …)
│   │   └── wizard.rs            # WizardState; step machine
│   └── update/
│       ├── mod.rs               # Re-exports; constants (REPO_OWNER, BIN_NAME)
│       ├── check.rs             # check_for_update_async/sync; semver max_by selection
│       ├── install.rs           # perform_update_async/sync; filter_restart_args(); restart_application()
│       ├── state.rs             # UpdateCheckResult; UpdateResult; UpdateState
│       ├── version.rs           # CURRENT_VERSION constant; version_newer()
│       ├── log.rs               # log_update() → /tmp/claude-workbench-update.log
│       └── release_notes.rs     # Platform-specific note filtering
├── tests/                       # Integration tests (if any)
├── Cargo.toml                   # Workspace manifest; feature flags
├── Cargo.lock                   # Locked dependency tree
├── config.yaml                  # Local config override (highest priority; not committed)
├── .planning/
│   └── codebase/                # GSD codebase mapping documents
└── .github/
    └── workflows/               # GitHub Actions: build, release binaries
```

## Directory Purposes

**`src/app/`:**
- Purpose: All application state and event routing
- Contains: `App` struct, event loop, per-concern sub-modules
- Key files: `mod.rs` (App + run loop), `job_state.rs` (async pattern), `pty.rs` (PTY helpers)

**`src/ui/`:**
- Purpose: Pure rendering — no business logic, no state mutation
- Contains: One file per UI component; all take `&App` or sub-state references
- Key files: `layout.rs` (geometry), `terminal_pane.rs` (vt100 cell rendering), `footer.rs`

**`src/browser/`:**
- Purpose: File-to-browser pipeline and document export
- Contains: opener, markdown converter, syntax highlighter, PDF exporter, HTML template
- Key files: `opener.rs` (launch with `validate_program()`), `pdf_export.rs` (RAII temp files)

**`src/update/`:**
- Purpose: Self-update lifecycle
- Contains: GitHub release check, binary replacement, restart, version comparison
- Key files: `check.rs` (semver max_by), `install.rs` (filter_restart_args + exec)

**`src/setup/`:**
- Purpose: First-run wizard and runtime dependency detection
- Contains: `DependencyReport` (clipboard helper availability), wizard state machine

## Key File Locations

**Entry Points:**
- `src/main.rs`: binary entry; CLI arg parse; tokio runtime; `App::new` + `App::run`

**Core State:**
- `src/app/mod.rs`: `App` struct definition (all fields); `run()` event loop
- `src/app/job_state.rs`: `JobState<T>` / `PollOutcome<T>` — use for every async job

**PTY:**
- `src/terminal.rs`: `PseudoTerminal` — create, resize, read, write, exited flag
- `src/app/pty.rs`: Claude command builder; `quote_path_for_cd()`; `sync_terminals()`; `ensure_pty_for_pane()`

**Clipboard:**
- `src/clipboard.rs`: `copy_to_clipboard()` (async), `copy_to_clipboard_sync()`, `paste_from_clipboard()`, `which()`, `is_executable()`, `is_ssh_session()`

**Browser/Preview:**
- `src/browser/opener.rs`: `open_file_with_browser()`, `open_file_with_editor()`, `validate_program()`
- `src/browser/pdf_export.rs`: `default_preview_file()` → `NamedTempFile`; `export_markdown()`

**Update:**
- `src/update/check.rs`: `check_for_update_async()`, semver `max_by` release selection
- `src/update/install.rs`: `perform_update_async()`, `filter_restart_args()`, `restart_application()`

**Configuration:**
- `src/config.rs`: `Config` struct; `load_config()` / `save_config()`
- `config.yaml` (repo root): local override; `~/.config/claude-workbench/config.yaml`: user config

## Naming Conventions

**Files:**
- `snake_case.rs` for all Rust source files
- `mod.rs` for module roots with sub-files
- Test helpers inlined in `#[cfg(test)]` blocks within the same file

**Directories:**
- `snake_case/` matching the module name

**Types:**
- `PascalCase` structs and enums (`App`, `JobState`, `PollOutcome`, `ClipboardOutcome`)
- `snake_case` functions and methods (`copy_to_clipboard`, `quote_path_for_cd`)
- Constants: `SCREAMING_SNAKE_CASE` (`CURRENT_VERSION`, `REPO_OWNER`, `SUBPROCESS_TIMEOUT`)

## Where to Add New Code

**New async background job:**
1. Add `your_job: JobState<YourResult>` field to `App` in `src/app/mod.rs`
2. Spawn with `JobState::running(rx)` from a `thread::spawn` that sends on `tx`
3. Add `poll_your_job()` method (pattern: `src/app/git_ops.rs` or `src/app/update.rs`)
4. Call `self.poll_your_job()` in the `run()` loop in `src/app/mod.rs`

**New pane-level keyboard handler:**
- Add to the appropriate file in `src/app/keyboard/` (one file per context)
- Dispatch via `handle_key_event()` in `src/app/keyboard/mod.rs`

**New UI overlay/dialog:**
- State struct in `src/ui/<name>.rs`; rendering function takes `&App` or sub-state
- Add state field to `App`; call render from `src/app/drawing.rs`

**New browser/external-process launch:**
- Always call `validate_program(program)?` before `Command::new(program)` — see `src/browser/opener.rs:85`
- Never concatenate user-supplied strings into a shell command

**New temp file for browser preview:**
- Use `default_preview_file(source, project_name)?` from `src/browser/pdf_export.rs`
- Store the returned `NamedTempFile` in `App::temp_preview_files`
- Never construct a predictable path; never call `cleanup_temp_files()`

**New one-shot CLI flag (--foo that exits before TUI):**
- Add `#[arg(long)] foo: bool` to `Args` in `src/main.rs`
- Add dispatch branch in `main()` before `async_main()`
- **Mandatory:** add `"--foo"` to the match in `filter_restart_args()` in `src/update/install.rs:189` to prevent infinite restart loop after self-update

**New configuration option:**
- Add field to appropriate sub-struct in `src/config.rs`
- Provide a `Default` impl value
- Config is loaded from YAML; no code-gen needed

## New Helper Functions Added in v0.89.0

| Helper | Location | Purpose |
|--------|----------|---------|
| `quote_path_for_cd(path_str)` | `src/app/pty.rs:435` | Shell-safe `cd <path>\r` string; returns `None` on NUL (log+skip, never unescaped fallback) |
| `is_executable(path)` | `src/clipboard.rs:124` (`#[cfg(unix)]`) | Check execute bits before `which()` returns a candidate |
| `validate_program(prog)` | `src/browser/opener.rs:85` | Allow-list check before any `Command::new()` from config strings |
| `filter_restart_args(args)` | `src/update/install.rs:180` | Strip one-shot flags from restart argv to prevent infinite loops |

## Special Directories

**`.planning/`:**
- Purpose: GSD planning documents (codebase maps, phase plans)
- Generated: No (human/agent authored)
- Committed: Yes

**`.github/workflows/`:**
- Purpose: CI — build matrix (4 targets), release binary upload
- Generated: No
- Committed: Yes

**`target/`:**
- Purpose: Cargo build artifacts
- Generated: Yes
- Committed: No

---

*Structure analysis: 2026-05-11*
