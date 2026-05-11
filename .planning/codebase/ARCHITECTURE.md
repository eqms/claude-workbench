<!-- refreshed: 2026-05-11 -->
# Architecture

**Analysis Date:** 2026-05-11

## System Overview

```text
┌─────────────────────────────────────────────────────────────────────┐
│                        main.rs — Entry Point                        │
│  CLI dispatch: --check-update / --update-to(debug) /                │
│  --clipboard-diag / --ssh-paste-diag → exit                         │
│  Normal path: tokio multi-thread runtime → async_main()             │
└───────────────────────────┬─────────────────────────────────────────┘
                            │ App::new() + App::run()
                            ▼
┌─────────────────────────────────────────────────────────────────────┐
│                     app/mod.rs — App struct                         │
│  All mutable state: terminals HashMap, pane visibility, JobState,   │
│  temp_preview_files Vec<NamedTempFile>, async job receivers, …       │
│                                                                     │
│  run() event loop (16 ms poll):                                     │
│    check_and_restart_exited_ptys()                                  │
│    poll_git_check() / poll_export_result()                          │
│    poll_update_check() / poll_update_result()                       │
│    poll_clipboard_outcome()                                         │
│    terminal.draw(|f| self.draw(f))                                  │
│    event::read() → handle_mouse_event / handle_key_event            │
└──────┬──────────────┬───────────────┬──────────────────────────────-┘
       │              │               │
       ▼              ▼               ▼
┌─────────────┐ ┌──────────┐ ┌─────────────────────────────────────┐
│ app/pty.rs  │ │ app/     │ │              ui/ modules             │
│ PTY spawn   │ │keyboard/ │ │  layout.rs  file_browser.rs          │
│ sync/cd     │ │ mouse.rs │ │  preview.rs terminal_pane.rs         │
│ quoting     │ │          │ │  footer.rs  help.rs  dialog.rs       │
└──────┬──────┘ └──────────┘ └─────────────────────────────────────┘
       │
       ▼
┌─────────────────────────────────────────────────────────────────────┐
│               terminal.rs — PseudoTerminal                         │
│  portable-pty master + vt100::Parser behind Arc<Mutex<>>           │
│  Background reader thread: PTY stdout → vt100 parser               │
│  Main thread: render from parser snapshot / write_input()          │
└──────────────────────┬──────────────────────────────────────────────┘
                       │
          ┌────────────┼────────────┐
          ▼            ▼            ▼
   PaneId::Claude  PaneId::LazyGit  PaneId::Terminal
   (claude CLI)   (lazygit)         (user shell)
```

## Component Responsibilities

| Component | Responsibility | File |
|-----------|----------------|------|
| `App` | All application state; event loop orchestration | `src/app/mod.rs` |
| `PseudoTerminal` | PTY lifecycle, vt100 parsing, scrollback | `src/terminal.rs` |
| `JobState<T>` | Generic async-job state machine (Idle/Running) | `src/app/job_state.rs` |
| PTY helpers | Claude command assembly; cd quoting; lazy-init | `src/app/pty.rs` |
| Layout engine | 6-pane Rect computation; configurable percentages | `src/ui/layout.rs` |
| Clipboard | Multi-backend copy/paste; async worker thread | `src/clipboard.rs` |
| Browser/opener | File→browser launch; `validate_program()` allow-list | `src/browser/opener.rs` |
| PDF/HTML export | Temp-file RAII preview; Typst PDF generation | `src/browser/pdf_export.rs` |
| Update/check | semver `max_by` release selection | `src/update/check.rs` |
| Update/install | Binary self-replace; `filter_restart_args()` | `src/update/install.rs` |
| Config | YAML load/save; all tuneable parameters | `src/config.rs` |
| Git ops | Background git-remote check | `src/app/git_ops.rs` |

## Pattern Overview

**Overall:** Single-threaded TUI event loop with background threads for blocking I/O.

**Key Characteristics:**
- One `App` struct owns all state; no global mutable state (clipboard worker uses `OnceLock<Mutex<>>`)
- Async jobs use `JobState<T>` (mpsc channel) polled each frame — never block the loop
- PTY output parsed off-main-thread; UI reads parser snapshot under short lock
- RAII governs all temp-file lifetime (`Vec<NamedTempFile>` on `App`)

## Layers

**Entry / CLI Layer:**
- Purpose: CLI argument dispatch; TUI initialisation; panic hook; signal masking
- Location: `src/main.rs`
- Contains: `Args` (clap), one-shot CLI modes, `async_main()`
- Depends on: all modules
- Used by: OS process spawner

**Application Layer:**
- Purpose: State ownership and event routing
- Location: `src/app/`
- Contains: `App`, `LayoutRects`, `SavedLayout`, sub-modules for keyboard/mouse/drawing/pty/clipboard/git_ops/update/ssh_paste
- Depends on: `terminal`, `ui`, `browser`, `clipboard`, `update`, `config`
- Used by: `main.rs`

**Terminal Layer:**
- Purpose: PTY process management and VT100 screen emulation
- Location: `src/terminal.rs`
- Contains: `PseudoTerminal`, `PtyCallbacks` (DSR/DA response)
- Depends on: `portable-pty`, `vt100`
- Used by: `app/pty.rs`, `ui/terminal_pane.rs`

**UI Layer:**
- Purpose: Ratatui widget rendering; no business logic
- Location: `src/ui/`
- Contains: `layout.rs`, `file_browser.rs`, `preview.rs`, `terminal_pane.rs`, `footer.rs`, `help.rs`, `fuzzy_finder.rs`, `dialog.rs`, `update_dialog.rs`, `wizard_ui.rs`, `settings.rs`, `about.rs`, `menu.rs`, `permission_mode.rs`, `claude_startup.rs`, `drag_ghost.rs`, `syntax.rs`
- Depends on: `types`, `app` state (read-only references during draw)
- Used by: `app/drawing.rs`

**Browser Layer:**
- Purpose: File preview in external browser; PDF/HTML export
- Location: `src/browser/`
- Contains: `opener.rs`, `pdf_export.rs`, `markdown.rs`, `syntax.rs`, `template.rs`, `typst_pdf.rs`
- Depends on: `tempfile`, optionally `typst` (feature-gated)
- Used by: `app/file_ops.rs`, `app/keyboard/`

**Clipboard Layer:**
- Purpose: Multi-backend copy/paste with async worker; SSH/XRDP fallbacks
- Location: `src/clipboard.rs`
- Contains: `copy_to_clipboard()`, `paste_from_clipboard()`, `ClipboardStrategy`, worker thread, `which()`, `is_executable()`
- Depends on: `arboard`, subprocess helpers
- Used by: `app/clipboard.rs`, `main.rs` (diag)

**Update Layer:**
- Purpose: GitHub release check; binary self-replace; restart
- Location: `src/update/`
- Contains: `check.rs`, `install.rs`, `state.rs`, `version.rs`, `log.rs`, `release_notes.rs`, `mod.rs`
- Depends on: `self_update`, `semver`
- Used by: `app/update.rs`, `main.rs`

## JobState<T> — Generic Async Job

`src/app/job_state.rs` defines the canonical pattern for all background work:

```rust
pub enum JobState<T> {
    Idle,
    Running(Receiver<T>),
}

pub enum PollOutcome<T> { Pending, Ready(T), Disconnected }

impl<T> JobState<T> {
    pub fn poll(&mut self) -> PollOutcome<T> { … } // resets to Idle on terminal outcome
}
```

Active `JobState` fields on `App`:
- `git_check_job: JobState<GitRemoteCheckResult>` — remote-ahead detection
- `update_check_job: JobState<UpdateCheckResult>` — GitHub release poll
- `update_job: JobState<UpdateResult>` — binary download + replace
- `export_job: JobState<Result<PathBuf, String>>` — async PDF/MD export

All are polled unconditionally each frame. Callers do not hold receivers directly; `JobState` encapsulates the `Receiver<T>` and exposes only the outcome enum.

## Data Flow

### Primary Event Loop

1. `main.rs` calls `App::run(terminal)` (`src/app/mod.rs:427`)
2. Each iteration: poll background jobs → `terminal.draw()` → `event::poll(16ms)`
3. `Event::Key(k)` → `handle_key_event(k)` (`src/app/keyboard/mod.rs`)
4. `Event::Mouse(m)` → `handle_mouse_event(m, rects)` (`src/app/mouse.rs`)
5. `Event::Paste(text)` → `handle_paste_event(text)` (bracketed paste)
6. Key events for terminal panes → `pty.write_input(bytes)` (`src/terminal.rs`)

### PTY Threading Model

```
Main thread                      Reader thread (per PTY)
    │                                    │
    │  PseudoTerminal::new()             │
    │  ──────────────────────────────►  spawned
    │                                    │  loop { pty_reader.read() }
    │                                    │    → parser.lock().process()
    │                                    │  EOF → exited.store(true)
    │
    │  draw(): parser.lock().screen()   ← short read lock
    │  write_input(): writer.lock()     ← short write lock
```

The `Arc<Mutex<vt100::Parser<PtyCallbacks>>>` is shared between the reader thread (writer) and main thread (reader during draw). Lock contention is bounded: the reader thread holds it only for the duration of `process()` per read chunk; the main thread holds it only for `screen()` snapshot during draw.

### vt100 Parser and Scrollback

- Parser initialized with 1000-line scrollback: `vt100::Parser::new(rows, cols, 1000)`
- `PtyCallbacks` implements `vt100::Callbacks` to intercept DSR/DA queries and send PTY responses back synchronously
- `write_input()` resets scroll position to 0 (bottom) so typed input is always visible
- `Shift+PageUp/Down` and `Shift+Up/Down` adjust `App`'s scroll offset; render reads `parser.screen()` at that offset

### Directory Sync Pattern

`sync_terminals()` (`src/app/pty.rs:169`) sends `cd <quoted-path>\r` to the Terminal pane only when the file-browser directory changes. Claude pane keeps its initial working directory permanently.

Path quoting uses `quote_path_for_cd()` (`src/app/pty.rs:435`), a module-private helper that wraps `shlex::try_quote`. Returns `None` only for NUL-containing paths (unreachable on real filesystems). Callers receiving `None` log via `log_update()` and skip — never fall back to unescaped output.

### Browser Preview / Temp-File Lifetime (RAII)

`default_preview_file()` (`src/browser/pdf_export.rs:114`) creates a `tempfile::NamedTempFile` via `Builder::new().prefix(…).suffix(".html").tempfile_in(temp_dir())`. The kernel guarantees exclusive O_EXCL creation — no predictable path, no symlink-attack vector.

The returned `NamedTempFile` is pushed into `App::temp_preview_files: Vec<tempfile::NamedTempFile>` (`src/app/mod.rs:161`). Files are deleted automatically when:
- A new preview replaces them (old entry removed from `Vec`)
- `App` drops at process exit

There is no `cleanup_temp_files()` function. Deletion is purely RAII.

`markdown_to_html` and `text_to_html` (callers in `app/file_ops.rs`) similarly return `NamedTempFile` values that the caller stores in `temp_preview_files`.

### Update Path — filter_restart_args Invariant

`restart_application()` (`src/update/install.rs:209`) re-execs the binary. Before forwarding `std::env::args()` to the new process, it calls `filter_restart_args()` (`src/update/install.rs:180`) which strips one-shot flags:

- `--update-to` (+ its value argument)
- `--check-update`
- `--clipboard-diag`
- `--ssh-paste-diag`

**Invariant:** any flag that causes early-exit without starting the TUI must be listed here. Omitting a flag would cause an infinite restart loop (issue tag: IN-02).

`--update-to` is `#[cfg(debug_assertions)]` in `Args` (`src/main.rs:52`) — release binaries cannot trigger intentional downgrade.

Release selection in `check.rs` uses `semver::Version` `max_by` over all fetched releases rather than trusting `releases[0]` (creation-order). Unparseable tags (nightly, pre-release) are silently skipped.

## Architectural Constraints

- **Threading:** Single main thread owns all `App` state. Background threads communicate via `mpsc` channels only (never share `App` references). Clipboard worker is a separate long-lived thread (`OnceLock<Sender>`).
- **Global state:** `src/clipboard.rs` uses three `OnceLock` singletons: `STRATEGY`, `IS_SSH`, `WORKER_TX`/`OUTCOME_SLOT`. No other module-level mutable state.
- **Circular imports:** None detected. `app/` imports `terminal`, `ui`, `browser`, `clipboard`, `update`. `ui/` imports `types` only. `browser/` does not import `app/`.
- **Unsafe blocks:** Confined to `libc::localtime_r` in `src/browser/pdf_export.rs:89` (date formatting) and `libc::signal(SIGTSTP, SIG_IGN)` in `src/main.rs:359`. No unsafe PTY or parser code.
- **Feature flags:** `pdf-export` gates Typst PDF rendering. `src/browser/pdf_export.rs` compiles in all configurations; only the `ExportFormat::Pdf` branch is gated.

## Anti-Patterns

### Spawning commands with unvalidated program strings

**What happens:** Old code passed `browser`/`editor` config strings directly to `Command::new()`.
**Why it's wrong:** Shell metacharacters in a config value could execute unintended commands.
**Do this instead:** Call `validate_program(program)` (allow-list: alphanumeric + `_-./ +`) before any `Command::new()`. See `src/browser/opener.rs:85`.

### Predictable temp-file paths

**What happens:** Former `default_preview_filename()` returned a fixed path like `/tmp/{project}-preview.html`.
**Why it's wrong:** Symlink attack (CR-03 / SEC-04): attacker pre-creates symlink at that path.
**Do this instead:** Use `tempfile::Builder` with `O_EXCL` — `default_preview_file()` in `src/browser/pdf_export.rs:114`.

### Selecting GitHub releases by creation order

**What happens:** Former code used `releases[0]`.
**Why it's wrong:** A backdated patch release could suppress legitimate updates.
**Do this instead:** `max_by(semver::Version)` across all fetched releases. See `src/update/check.rs:43`.

## Error Handling

**Strategy:** `anyhow::Result` throughout the public API surface. PTY spawn errors stored as `Option<String>` on `App` and rendered as red-bordered pane messages. Background job failures surfaced via `PollOutcome::Disconnected` or `JobState`-wrapped `Result`.

**Patterns:**
- PTY errors: stored in `claude_error` / `lazygit_error` / `terminal_error` on `App`; displayed in `terminal_pane.rs`
- Clipboard errors: `ClipboardOutcome::Failed(reason)` written to `OUTCOME_SLOT`; polled by `poll_clipboard_outcome()` → `clipboard_error_flash` footer message
- Export errors: `export_job: JobState<Result<PathBuf, String>>` — `Err(String)` shown in dialog

## Cross-Cutting Concerns

**Logging:** `update::log_update()` writes to `/tmp/claude-workbench-update.log`. All other operational logging is absent (TUI renders own diagnostic output).
**Validation:** Program names validated via `validate_program()` allow-list before `Command::new()`. Path quoting via `shlex::try_quote` before PTY injection.
**Authentication:** None (no user accounts). Claude CLI handles its own auth.

---

*Architecture analysis: 2026-05-11*
