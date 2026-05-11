# Technology Stack

**Analysis Date:** 2026-05-11

## Languages

**Primary:**
- Rust 2021 edition — entire codebase (`src/`)

**Secondary:**
- YAML — configuration (`config.yaml`, `~/.config/claude-workbench/config.yaml`)

## Runtime

**Environment:**
- Native binary, no VM or interpreter
- Single process, multi-threaded (tokio multi-thread runtime + dedicated clipboard worker thread + per-PTY reader threads)

**Package Manager:**
- Cargo (Rust toolchain)
- Lockfile: `Cargo.lock` (present, committed)

## Frameworks

**TUI / Rendering:**
- `ratatui` 0.30.0 — terminal UI framework (widgets, layout, rendering)
- `crossterm` 0.28.1 — cross-platform terminal I/O, keyboard/mouse events, raw mode
  - Pinned to 0.28 — `tui-textarea` fork depends on `crossterm 0.28` event types

**Terminal Emulation:**
- `portable-pty` 0.8.1 — pseudo-terminal (PTY) creation and management for embedded shells
- `vt100` 0.16 — VT100/ANSI escape sequence parser with scrollback buffer (1000 lines default)

**Async Runtime:**
- `tokio` 1.44.0 (`rt-multi-thread` feature) — drives the main async event loop

**CLI Argument Parsing:**
- `clap` 4.5.37 (`derive`, `env` features) — `--check-update`, `--update-to`, `--clipboard-diag`, `--ssh-paste-diag`, `--fake-version` (debug only)

**Text Editing:**
- `tui-textarea` (git: `https://github.com/0xferrous/tui-textarea.git`, branch `update-ratatui`) — inline text editor widget, patched for ratatui 0.30 compatibility

**Markdown / Preview:**
- `tui-markdown` 0.3 — Markdown rendering in TUI panes
- `pulldown-cmark` 0.13 — Markdown parsing (CommonMark)

**Syntax Highlighting:**
- `syntect` 5.2 (`default-syntaxes`, `default-themes`, `regex-onig`) — syntax highlighting for file preview pane

**PDF Export (optional feature, enabled by default):**
- `typst` 0.14.2 — pure-Rust typesetting engine
- `typst-pdf` 0.14.2 — PDF output backend
- `typst-library` 0.14.2 — standard library for typst
- `typst-kit` 0.14.2 (`fonts` feature) — font loading
- `comemo` 0.4 — memoization required by `typst::World` trait
- `ecow` 0.2 — `EcoString`/`EcoVec` types used in typst API
- Feature flag: `pdf-export` (in `[features]`, on by default; disable with `--no-default-features`)

**Serialization:**
- `serde` 1.0.219 (`derive`) — config struct serialization
- `serde_yaml_ng` 0.9 — YAML config file parsing/writing

**Clipboard:**
- `arboard` 3.6 (`wayland-data-control` feature) — cross-platform clipboard (X11 + Wayland native)
- Subprocess fallback chain (no additional crate): `xclip` → `xsel` → `wl-copy`/`wl-paste` → OSC 52 escape sequence
- Strategy controlled by env var `CLAUDE_WORKBENCH_CLIPBOARD` (`osc52` | `arboard` | `subprocess`)

**Self-Update:**
- `self_update` 0.42 (features: `archive-tar`, `archive-zip`, `compression-flate2`, `compression-zip-deflate`, `rustls`, `signatures`) — downloads and installs GitHub Release assets

**Utilities:**
- `anyhow` 1.0.98 — error handling and propagation
- `dirs` 5.0 — XDG/platform-aware home/config directory resolution
- `shlex` 1.3 — shell-style argument splitting for PTY command construction
- `regex` 1.12 — pattern matching (git status parsing, file filtering)
- `libc` 0.2 — SIGTSTP suppression via `libc::signal()` (Unix only, one `unsafe` block in `src/main.rs`)
- `tempfile` 3 — temporary files for update staging

## Key Dependencies

**Critical (would break core functionality if removed):**
- `ratatui` 0.30.0 — entire UI layer
- `crossterm` 0.28.1 — terminal raw mode, events; version pinned, do not bump without updating `tui-textarea`
- `portable-pty` 0.8.1 — embedded PTY shells (Claude, LazyGit, Terminal panes)
- `vt100` 0.16 — PTY output rendering
- `tokio` 1.44.0 — async main loop

**Version-Sensitive:**
- `crossterm` — must stay at 0.28.x; `tui-textarea` fork branch `update-ratatui` imports `crossterm 0.28` event types. Bumping to 0.29 breaks `editor.input(Event::Key(...))` call sites.
- `tui-textarea` — sourced from git fork, not crates.io. `Cargo.lock` must be committed.

**Infrastructure:**
- `self_update` 0.42 — GitHub Releases API, binary download, extraction, atomic self-replace
- `arboard` 3.6 — clipboard; `wayland-data-control` feature needed for Wayland sessions

## Configuration

**Format:** YAML, parsed/written via `serde_yaml_ng`

**Load priority (first match wins):**
1. `./config.yaml` — project-local override (CWD)
2. `~/.config/claude-workbench/config.yaml` — XDG user config (`$XDG_CONFIG_HOME/claude-workbench/config.yaml` if `XDG_CONFIG_HOME` set)
3. Compiled-in defaults (via `Config::default()`)

**Config sections and key fields (`src/config.rs`):**
```yaml
terminal:
  shell_path: "/bin/bash"     # auto-detected from $SHELL / $COMSPEC
  shell_args: []

ui:
  theme: "default"
  show_file_browser: true
  show_terminal: false
  show_lazygit: false
  show_preview: true
  browser: ""                 # empty = system default (open/xdg-open)
  external_editor: ""
  export_dir: ""              # empty = ~/Downloads

layout:
  claude_height_percent: 40
  file_browser_width_percent: 20
  preview_width_percent: 50
  right_panel_width_percent: 30

file_browser:
  show_hidden: true
  show_file_info: true
  date_format: "%d.%m.%Y %H:%M:%S"
  auto_refresh_ms: 2000

pty:
  claude_command: []          # empty = use terminal shell; user starts claude manually
  lazygit_command: ["lazygit"]
  scrollback_lines: 1000
  auto_restart: true
  copy_lines_count: 50        # lines copied by F9

claude:
  startup_prefixes: []
  default_permission_mode: null
  show_permission_dialog: true
  remote_control: false
  default_model: Unset        # ClaudeModel enum
  default_effort: Unset       # ClaudeEffort enum
  default_session_name: ""
  default_worktree: ""

ssh:
  enabled: true
  image_paste_helper: null    # path to cc-clip binary; null = $PATH lookup
  notification_dismissed: false

document:
  company: { name, footer_text, author, website }
  fonts: { body, code }
  colors: { accent, table_header_bg, ... }   # 13 color fields
  sizes: { title, h1, h2, h3, body, table, code, footer, header, line_height, ... }
  pdf: { page_size: "A4", margin: "2.5cm" }
```

**Saved with 0600 permissions** on Unix (owner read/write only).

**Build configuration:**
- `Cargo.toml` — single source of truth for dependencies
- Features: `default = ["pdf-export"]`; disable Typst PDF with `cargo build --no-default-features`

## Platform Requirements

**Development:**
- Rust stable toolchain (2021 edition)
- `cargo build` / `cargo run`
- `cargo clippy`, `cargo fmt`, `cargo test`

**Runtime (Linux):**
- X11 session: `xclip` or `xsel` recommended (clipboard over XRDP)
- Wayland session: `wl-clipboard` package (`wl-copy`, `wl-paste`)
- `lazygit` binary on `$PATH` (LazyGit pane)
- `claude` CLI on `$PATH` (Claude pane, if `pty.claude_command` not set)

**Runtime (macOS):**
- `arboard` uses native pasteboard — no external clipboard tools needed
- `open` command used for browser/file opening (`src/browser/opener.rs`)

**Production (binary distribution):**
- GitHub Releases: `claude-workbench-{target}.tar.gz`
- Supported targets: `aarch64-apple-darwin`, `x86_64-apple-darwin`, `aarch64-unknown-linux-gnu`, `x86_64-unknown-linux-gnu`
- Self-update: binary downloads and replaces itself at runtime

---

*Stack analysis: 2026-05-11*
