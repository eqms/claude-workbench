# External Integrations

**Analysis Date:** 2026-05-11

## APIs & External Services

**GitHub Releases (self-update):**
- Purpose: Check for newer versions, download and install updated binary
- Crate: `self_update` 0.42 (with `rustls` — no system OpenSSL dependency)
- Repo: `github.com/eqms/claude-workbench` (`REPO_OWNER = "eqms"`, `REPO_NAME = "claude-workbench"`)
- Implementation: `src/update/` (submodules: `check.rs`, `install.rs`, `log.rs`, `release_notes.rs`, `state.rs`, `version.rs`)
- Entry points: `check_for_update_async()`, `perform_update_sync()`, `perform_update_to_version_sync()`
- Triggers: async background check at startup; manual via `u` key in Help screen (F12); CLI `--check-update`
- Asset naming: `claude-workbench-{target}.tar.gz` (four targets; see STACK.md)
- Log file: `/tmp/claude-workbench-update.log`
- Auth: none (public repo, unauthenticated API)
- Network calls are blocking-sync in CLI modes; async via tokio in TUI mode

**cc-clip (SSH image paste, optional):**
- Purpose: Relay Mac/Windows clipboard images over SSH reverse tunnel
- Project: `https://github.com/ShunmeiCho/cc-clip`
- Protocol: TCP on `127.0.0.1:9998` (user sets up `ssh -R 9998:localhost:9998`)
- Detection: `clipboard::which("cc-clip")` + `TcpStream::connect_timeout` (500 ms) in `src/main.rs:run_ssh_paste_diag_cli()`
- Config: `ssh.image_paste_helper` in `config.yaml` (path override; `null` = `$PATH` lookup)
- Implementation: integrated in `src/clipboard.rs` and `src/config.rs` (`SshConfig`)
- No crate dependency — pure TCP socket check + subprocess invocation

## Data Storage

**Databases:** None

**File Storage:**
- Config: `./config.yaml` (project-local) or `~/.config/claude-workbench/config.yaml` (XDG)
- Session state: `src/session/mod.rs` — currently stubbed (returns default, not persisted)
- Update log: `/tmp/claude-workbench-update.log` (written by `src/update/log.rs`)
- Temp files: staging area during self-update (`tempfile` crate)

**Caching:** None

## Clipboard Integration

**Primary library:** `arboard` 3.6 (`wayland-data-control` feature) — `src/clipboard.rs`

**Copy fallback chain (in order):**
1. `arboard` — native X11/Wayland/macOS pasteboard
2. `xclip -selection clipboard -i` — subprocess, X11
3. `xsel --clipboard --input` — subprocess, X11
4. `wl-copy` — subprocess, Wayland
5. OSC 52 escape sequence — terminal-emulator based, works over SSH (write-only, no read path)

**Paste fallback chain:**
1. `arboard`
2. `xclip -selection clipboard -o`
3. `xsel --clipboard --output`
4. `wl-paste --no-newline`

**Strategy selection logic (`src/clipboard.rs:detect_strategy()`):**
- Non-Linux: `ArboardFirst`
- Linux with `XRDP_SESSION` set OR `XDG_SESSION_TYPE=x11`, AND `xclip`/`xsel` present: `SubprocessFirst`
- Otherwise: `ArboardFirst`
- Override: env var `CLAUDE_WORKBENCH_CLIPBOARD=osc52|arboard|subprocess`

**Async design:** Copy is dispatched to a background worker thread (`clipboard-worker`) via `std::sync::mpsc`. The main loop returns `ClipboardOutcome::Submitted` immediately and polls `take_pending_outcome()` once per frame. Paste stays synchronous (callers need text immediately).

**Subprocess timeout:** 500 ms per helper (`SUBPROCESS_TIMEOUT`), polled every 20 ms. Process is killed and reaped on timeout to prevent zombies.

**Diagnostic CLI:** `--clipboard-diag` flag runs `ClipboardDiag::collect()` + copy/paste roundtrip test without starting TUI.

## Terminal Emulator Interactions

**PTY spawning:** `portable-pty` 0.8.1 (`src/terminal.rs`)
- Three PTY processes: Claude pane, LazyGit pane, User Terminal pane
- All inherit full parent environment (`App::new` in `src/app.rs`)
- Start in file browser's current working directory
- `fish_features=no-query-term` set to suppress Fish shell DA query

**VT100 parsing:** `vt100` 0.16
- Each PTY has a dedicated background reader thread updating `Arc<Mutex<vt100::Parser>>`
- Parser initialized with 1000-line scrollback buffer
- Main UI thread locks parser only during rendering

**Bracketed paste:** enabled via `crossterm::event::EnableBracketedPaste` at startup (`src/main.rs:async_main`)

**OSC 52:** Raw stdout write (`\x1b]52;c;{base64}\x07` + ST terminator) — bypasses crossterm buffering. Hand-rolled base64 encoder in `src/clipboard.rs:base64_encode()` (no external crate).

## External Programs Launched as Subprocesses

**lazygit:**
- Launched as PTY process in LazyGit pane
- Command: configurable via `pty.lazygit_command` (default: `["lazygit"]`)
- Must be on `$PATH` or configured with full path

**claude (Anthropic CLI):**
- Launched as PTY process in Claude pane
- Command: configurable via `pty.claude_command` (default: `[]` = user's login shell, user starts claude manually)
- Supports flags: `--model`, `--effort`, `--permission-mode`, `--name`, `--worktree`, `remote-control`
- Startup prefix dialog (`src/ui/claude_startup.rs`) prepends custom text before command

**System shell (User Terminal pane):**
- Command: `terminal.shell_path` + `terminal.shell_args` from config
- Default: `$SHELL` (Unix) / `%COMSPEC%` → PowerShell → cmd.exe (Windows)
- PTY sync: `cd "{path}"\r` sent when file browser changes directory

**xclip / xsel / wl-copy / wl-paste:**
- Launched on-demand as clipboard subprocesses (see Clipboard Integration above)
- Located via `$PATH` search (`clipboard::which()`)

**cc-clip:**
- Optional SSH image paste helper
- Located via `ssh.image_paste_helper` config or `$PATH` search

## File System Access

**File browser (`src/ui/file_browser.rs`):**
- Reads directory entries (`std::fs::read_dir`)
- Shows git status colors via `src/git.rs` (runs `git status --porcelain` subprocess)
- Auto-refresh every `file_browser.auto_refresh_ms` ms (default: 2000)

**File preview (`src/ui/preview.rs`):**
- Reads file contents for syntax-highlighted preview (syntect)
- Markdown rendered via `tui-markdown`
- HTML/Markdown/PDF/images opened in system browser via `src/browser/opener.rs`

**Browser/file opening (`src/browser/opener.rs`):**
- macOS: `open {path}`
- Linux: `xdg-open {path}`
- Windows: `start {path}`
- Command override: `ui.browser` config field

**Markdown to HTML (`src/browser/markdown.rs`):**
- Converts Markdown to styled HTML (`pulldown-cmark`)
- Writes to temp file, opens in browser

**PDF export (`src/ui/` + typst pipeline):**
- Markdown → Typst markup → PDF via `typst`, `typst-pdf`, `typst-library`, `typst-kit`
- Output directory: `ui.export_dir` config (default: `~/Downloads`)
- Feature-gated: `pdf-export` (on by default)

## Authentication & Identity

**Auth Provider:** None (no user accounts, no auth flows)

**Config file permissions:** 0600 (Unix owner read/write only) set by `save_config()` in `src/config.rs`

## Monitoring & Observability

**Error Tracking:** None

**Logs:**
- Startup progress lines written to `stderr` before ratatui takes over `stdout`
- Update operations: `/tmp/claude-workbench-update.log` (`src/update/log.rs`)
- No structured logging framework (no `tracing`, no `log` crate)

**Panic handling:**
- Custom panic hook in `src/main.rs:async_main()` calls `restore_terminal()` before printing panic info
- Ensures terminal raw mode is restored on crash

## CI/CD & Deployment

**Primary repo:** `git@gitlab.ownerp.io:ki/workbench.git` (remote `origin`)
**Open-source mirror:** `git@github.com:eqms/claude-workbench.git` (remote `upstream`)
- Both remotes kept in sync on every commit (dual-push strategy)

**GitHub Actions:**
- Builds release binaries for all four targets
- Publishes as GitHub Release assets (`claude-workbench-{target}.tar.gz`)
- Tag format: `vX.Y.Z`

**Binary self-update** pulls from GitHub Releases (see GitHub Releases section above).

## Webhooks & Callbacks

**Incoming:** None

**Outgoing:** None (update check is a pull, not a push)

## Environment Variables

**Runtime behavior:**
- `CLAUDE_WORKBENCH_CLIPBOARD` — override clipboard strategy (`osc52` | `arboard` | `subprocess`)
- `SHELL` — default shell for User Terminal pane (Unix)
- `COMSPEC` — default shell for User Terminal pane (Windows)
- `XDG_CONFIG_HOME` — override config directory root
- `XDG_SESSION_TYPE` — used for X11/Wayland clipboard strategy detection
- `XRDP_SESSION` — triggers `SubprocessFirst` clipboard strategy (XRDP environment)
- `SSH_TTY` / `SSH_CONNECTION` — SSH session detection (for cc-clip hint, OSC 52 fallback)
- `WAYLAND_DISPLAY` / `DISPLAY` — display server detection
- `fish_features=no-query-term` — set on child PTY environment to suppress Fish DA query
- `WORKBENCH_FAKE_VERSION` — fake version for update testing (debug builds only, mapped to `--fake-version`)

---

*Integration audit: 2026-05-11*
