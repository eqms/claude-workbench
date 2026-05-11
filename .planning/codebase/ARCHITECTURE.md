<!-- refreshed: 2026-05-11 -->
# Architecture

**Analysis Date:** 2026-05-11

## System Overview

```text
┌─────────────────────────────────────────────────────────────────────┐
│                          main.rs                                     │
│  CLI dispatch (--check-update / --clipboard-diag / --ssh-paste-diag)│
│  tokio multi-thread runtime → async_main()                          │
└──────────────────────────────┬──────────────────────────────────────┘
                               │ App::new() + App::run()
                               ▼
┌─────────────────────────────────────────────────────────────────────┐
│                     App (src/app/mod.rs)                             │
│  Monolithic state struct — owns all sub-state                       │
│  Event loop: draw → poll(16ms) → route to handler                   │
│                                                                      │
│  Sub-modules (all impl App):                                         │
│   keyboard/   mouse.rs   drawing.rs   file_ops.rs   git_ops.rs      │
│   clipboard.rs   pty.rs   ssh_paste.rs   update.rs                  │
└───────┬──────────────────────────────────────────────────┬──────────┘
        │                                                  │
        ▼                                                  ▼
┌───────────────────┐                         ┌───────────────────────┐
│  Input Layer      │                         │  Async Job Layer      │
│  src/app/keyboard/│                         │  src/app/job_state.rs │
│  src/app/mouse.rs │                         │  src/app/update.rs    │
│  src/input.rs     │                         │  src/app/git_ops.rs   │
└───────────────────┘                         │  JobState<T> generic  │
                                              └──────────┬────────────┘
                                                         │ mpsc Receiver<T>
        ┌────────────────────────────────────────────────┘
        ▼
┌───────────────────────────────────────────────────────────────────┐
│                    PTY Layer  (src/terminal.rs)                    │
│  PseudoTerminal — one instance per pane                           │
│  ┌─────────────────────────────────────────────────────────────┐ │
│  │  Background reader thread                                   │ │
│  │  reader.read() → vt100::Parser::process() → callbacks      │ │
│  │  shared via Arc<Mutex<vt100::Parser<PtyCallbacks>>>         │ │
│  └─────────────────────────────────────────────────────────────┘ │
│  write_input() → Arc<Mutex<Box<dyn Write + Send>>>                │
│  exited: Arc<AtomicBool>                                          │
└───────────────────────────────┬───────────────────────────────────┘
                                │ lock parser during draw
                                ▼
┌───────────────────────────────────────────────────────────────────┐
│                    UI / Render Layer  (src/ui/)                    │
│  drawing.rs → frame.render_widget() per pane                      │
│  layout.rs  → compute_layout() returns 6 Rect structs             │
│  terminal_pane.rs reads vt100::Screen cells                       │
│  file_browser.rs / preview.rs / footer.rs / help.rs / ...        │
└───────────────────────────────────────────────────────────────────┘
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

**Overall:** Single-threaded event loop with shared-state PTY threads.

**Key Characteristics:**
- `App` is a single monolithic struct (≈165 fields). All handler modules implement `App` via `impl App` blocks, not separate structs.
- PTY output is produced on N background threads (one per pane); main thread consumes via `Arc<Mutex<vt100::Parser>>` during `draw()`.
- Async jobs (update check, git remote check, PDF export) use `std::sync::mpsc` channels wrapped in `JobState<T>`, polled each event-loop iteration with `try_recv()`. No blocking inside the loop.
- 16ms poll timeout gives ~60fps UI refresh rate.

## Layers

**CLI Dispatch Layer:**
- Purpose: Parse args, handle non-TUI modes, bootstrap tokio runtime
- Location: `src/main.rs`
- Contains: `Args` (clap), `run_update_check_cli`, `run_clipboard_diag_cli`, `run_ssh_paste_diag_cli`, `async_main`
- Depends on: `update`, `clipboard`, `session`, `config`, `app`

**App State Layer:**
- Purpose: Own all application state, run the event loop
- Location: `src/app/mod.rs`
- Contains: `App` struct, `App::new()`, `App::run()`
- Depends on: all other layers
- Used by: `main.rs` only

**Input Routing Layer:**
- Purpose: Translate crossterm events into App mutations
- Location: `src/app/keyboard/` (5 submodules), `src/app/mouse.rs`
- Contains:
  - `keyboard/mod.rs` — priority-ordered dispatch (dialogs first, then global, then pane-specific)
  - `keyboard/global.rs` — F7/F8/F9/F10/F11/F12, Ctrl+P/O/X/E
  - `keyboard/dialogs.rs` — fuzzy finder, update dialog, wizard, settings, permission mode, claude startup, about, help, menu, export chooser
  - `keyboard/file_browser.rs` — j/k/l/h navigation, enter, backspace
  - `keyboard/preview.rs` — search, edit mode (tui-textarea), read-only scroll
  - `keyboard/terminal.rs` — pass-through to PTY, scrollback (Shift+PageUp/Down)
- Depends on: `terminal.rs`, `types.rs`, `clipboard.rs`

**PTY Threading Layer:**
- Purpose: Manage subprocess lifecycle and terminal emulation
- Location: `src/terminal.rs`
- Contains: `PseudoTerminal`, `PtyCallbacks` (DSR/CPR/DA response handler)
- Shared state: `Arc<Mutex<vt100::Parser<PtyCallbacks>>>` read by UI, written by background thread
- Writer: `Arc<Mutex<Box<dyn Write + Send>>>` — locked only during `write_input()`
- Exit detection: `Arc<AtomicBool>` set by reader thread on EOF
- Depends on: `portable-pty`, `vt100`

**Async Job Layer:**
- Purpose: Non-blocking background work that delivers a single result
- Location: `src/app/job_state.rs`, used in `src/app/update.rs`, `src/app/git_ops.rs`, `src/app/clipboard.rs`
- Contains: `JobState<T>` enum (`Idle` | `Running(Receiver<T>)`), `PollOutcome<T>` enum
- Pattern: spawn `std::thread::spawn` → send on `mpsc::Sender<T>` → `App::run()` calls `poll()` each loop
- Active jobs on `App`: `git_check_job`, `update_check_job`, `update_job`, `export_job`

**UI Render Layer:**
- Purpose: Stateless frame rendering from App state
- Location: `src/ui/`
- Contains: one file per widget/pane (see STRUCTURE.md)
- Depends on: `ratatui`, `vt100` (reads parser screen), `syntect` (syntax highlighting)
- Called by: `src/app/drawing.rs` once per loop iteration

**Supporting Modules:**
- `src/clipboard.rs` — 5-stage fallback: arboard → xclip → xsel → wl-copy → OSC 52; async worker thread for copy, sync path for diagnostics
- `src/config.rs` — YAML via `serde_yaml_ng`, search paths: `./config.yaml` → `~/.config/claude-workbench/config.yaml`
- `src/git/mod.rs` — git status queries for file browser coloring and remote-ahead detection
- `src/update/` — GitHub Releases API via `self_update` crate, self-replace binary on disk
- `src/session.rs` — session persistence (currently returns defaults)
- `src/filter.rs` — file name filtering for fuzzy finder
- `src/syntax_registry.rs` — syntect `SyntaxSet` singleton

## Data Flow

### Primary Event Loop Iteration

1. `App::run()` calls `check_and_restart_exited_ptys()` — restart any PTY whose `exited` flag is set (`src/app/pty.rs`)
2. Auto-refresh file browser and preview if `auto_refresh_ms` elapsed
3. Poll all `JobState` receivers: `poll_git_check()`, `poll_export_result()`, `poll_update_check()`, `poll_update_result()`, `poll_clipboard_outcome()` (`src/app/update.rs`, `src/app/git_ops.rs`, `src/app/clipboard.rs`)
4. `terminal.draw(|frame| self.draw(frame))` — calls `src/app/drawing.rs` which calls `ui::layout::compute_layout()` then renders each pane
5. `event::poll(16ms)` — returns when crossterm has an event or timeout
6. Route event: `Mouse` → `handle_mouse_event()`, `Key` → `handle_key_event()`, `Paste` → `handle_paste_event()`

### PTY Output Path

1. Background thread in `PseudoTerminal::new()` calls `reader.read(&mut buffer)` in a loop
2. Acquires `Arc<Mutex<vt100::Parser>>`, calls `parser.process(&buffer[..n])`
3. `PtyCallbacks::unhandled_csi()` intercepts DSR/DA queries, queues responses
4. Responses written back to PTY via `Arc<Mutex<Box<dyn Write>>>` before releasing lock
5. Main thread acquires same mutex during `draw()` to read `parser.screen()` cells

### PTY Input Path

1. Key event routed to `keyboard/terminal.rs` → `handle_terminal_pane_key()`
2. Key translated to byte sequence via `src/input.rs` (`key_event_to_bytes()`)
3. `PseudoTerminal::write_input(bytes)` — resets scrollback to 0, then writes to `Arc<Mutex<writer>>`

### Async Job Pattern (e.g. update check)

1. User presses `u` in help screen → `app.start_update_check()` (`src/app/update.rs`)
2. `std::thread::spawn` runs blocking HTTP check, sends result on `mpsc::Sender<UpdateCheckResult>`
3. `app.update_check_job` transitions to `JobState::Running(receiver)`
4. Each loop iteration: `poll_update_check()` calls `update_check_job.poll()` → `try_recv()`
5. On `PollOutcome::Ready(result)`: update `update_state`, show dialog if update available

### PTY Resize

1. `draw()` calls `compute_layout()` to get current `Rect` for each terminal pane
2. For each visible terminal pane: `pty.resize(rect.height - 2, rect.width - 2)` (border accounting)
3. `PseudoTerminal::resize()` checks current size first — no-op if unchanged
4. On size change: `master.resize(PtySize)` + `parser.screen_mut().set_size()`

### Directory Sync

1. File browser navigates to new directory
2. `App::sync_terminals()` sends `cd "{path}"\r` bytes to Terminal pane PTY
3. LazyGit is restarted in new directory (killed and respawned) when F5 shows it while hidden

## Key Abstractions

**`PseudoTerminal` (`src/terminal.rs`):**
- Purpose: Wraps portable-pty + vt100 parser into a single owned handle
- Fields: `parser: Arc<Mutex<vt100::Parser<PtyCallbacks>>>`, `writer: Arc<Mutex<Box<dyn Write+Send>>>`, `master: Box<dyn MasterPty+Send>`, `exited: Arc<AtomicBool>`
- Pattern: background thread shares `Arc` clones; main thread accesses via `lock_or_recover()` (poison-safe)
- Instances: up to 3, keyed by `PaneId` in `App::terminals: HashMap<PaneId, PseudoTerminal>`

**`JobState<T>` (`src/app/job_state.rs`):**
- Purpose: Explicit lifecycle for single-shot async jobs replacing `Option<Receiver<T>>`
- States: `Idle` (no job) | `Running(Receiver<T>)` (in flight)
- `poll()` returns `PollOutcome::{Pending, Ready(T), Disconnected}` and auto-resets to `Idle`
- Used for: `git_check_job: JobState<GitRemoteCheckResult>`, `update_check_job: JobState<UpdateCheckResult>`, `update_job: JobState<UpdateResult>`, `export_job: JobState<Result<PathBuf, String>>`

**`LayoutRects` (`src/app/mod.rs`):**
- Purpose: Bundle of 6 `Rect` values recomputed per mouse event to hit-test pane clicks
- Fields: `files`, `preview`, `claude`, `lazygit`, `terminal`, `footer`

**`ClipboardOutcome` (`src/clipboard.rs`):**
- Purpose: Enumerate which fallback stage succeeded or why all failed
- Values: `Arboard | Xclip | Xsel | WlCopy | Osc52 | Failed(String) | Submitted`
- `Submitted` is returned immediately when copy is queued to async worker; real outcome arrives later via `take_pending_outcome()`

## Entry Points

**TUI application:**
- Location: `src/main.rs` → `async_main()` → `App::new()` + `App::run()`
- Triggers: normal `cargo run` / binary invocation without special flags

**CLI diagnostic modes (all exit without TUI):**
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

**What happens:** `App` in `src/app/mod.rs` accumulates ≈165 fields. All handler submodules operate as `impl App` blocks accessing the full struct directly.
**Why it's wrong:** Adding any new feature requires touching the `App` struct definition. Testing individual handlers requires constructing the full `App`. Compile times grow with the struct.
**Do this instead:** Extract cohesive sub-states (e.g. `ClipboardState`, `UpdateState`) as owned sub-structs and pass them by `&mut` to handlers. `UpdateState` already exists (`src/update/state.rs`) — this pattern should be extended.

### `lock_or_recover` poison suppression

**What happens:** `src/terminal.rs`'s `lock_or_recover()` silently recovers from poisoned mutexes, returning the inner data.
**Why it's wrong:** A panicking PTY reader thread will leave the parser in an unknown state; the main thread continues rendering corrupt screen data with no visible error.
**Do this instead:** Log the poison event and treat the pane as errored (set `exited = true`, show error overlay). At minimum surface a debug-build assertion.

## Error Handling

**Strategy:** `anyhow::Result` at the `App::run()` boundary; PTY errors stored as `Option<String>` on `App` (e.g. `claude_error`, `lazygit_error`, `terminal_error`) and rendered as red-border overlays inside the pane.

**Patterns:**
- PTY spawn failure: stored in `App::claude_error` / `lazygit_error` / `terminal_error`, shown as pane overlay
- PTY exit: `Arc<AtomicBool>` set by reader thread; `check_and_restart_exited_ptys()` respawns
- Clipboard failure: `ClipboardOutcome::Failed(msg)` triggers `clipboard_error_flash` footer banner (3s)
- Async job disconnect: `PollOutcome::Disconnected` resets job to `Idle`, UI silently returns to previous state

## Cross-Cutting Concerns

**Logging:** No structured logging framework. Startup progress written to stderr before ratatui enters alternate screen. Update operations write to `/tmp/claude-workbench-update.log`.
**Validation:** Config loaded at startup via serde; unknown fields silently ignored.
**Authentication:** Not applicable. Claude CLI handles its own auth.

---

*Architecture analysis: 2026-05-11*
