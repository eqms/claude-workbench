# Technology Stack

**Analysis Date:** 2026-05-11

## Languages

**Primary:**
- Rust (edition 2021) — entire codebase (`src/`)

**Secondary:**
- YAML — configuration files (`config.yaml`, `~/.config/claude-workbench/config.yaml`)

## Runtime

**Environment:**
- Native binary (no runtime VM). Compiled to platform-native executable.
- Targets: `aarch64-apple-darwin`, `x86_64-apple-darwin`, `aarch64-unknown-linux-gnu`, `x86_64-unknown-linux-gnu`

**Package Manager:**
- Cargo (rustup toolchain)
- Lockfile: `Cargo.lock` — present and committed (binary crate)

## Frameworks

**Core:**
- `ratatui` 0.30.0 — TUI rendering framework (widgets, layout, drawing)
- `crossterm` 0.28.1 — terminal backend for ratatui; **pinned at 0.28**: `tui-textarea` fork targets crossterm 0.28 event types. Upgrading to 0.29 breaks `editor.input(Event::Key(...))` call sites.
- `tokio` 1.44.0 (features: `rt-multi-thread`) — async runtime; used for update checks and background tasks

**Build/Dev:**
- `cargo build --release` — standard release build
- Feature flag `pdf-export` enabled by default; disable with `--no-default-features`

## Key Dependencies

**UI & Terminal:**
- `ratatui` 0.30.0 — widget system, layout engine (`src/ui/`)
- `crossterm` 0.28.1 — keyboard/mouse events, raw mode, terminal control
- `tui-textarea` (git: `https://github.com/0xferrous/tui-textarea.git`, branch `update-ratatui`) — editor widget; fork pinned to crossterm 0.28
- `portable-pty` 0.8.1 — PTY creation and management (`src/terminal.rs`)
- `vt100` 0.16.2 — VT100 terminal emulator / screen buffer parser; 1000-line scrollback

**Input Safety:**
- `shlex` 1.3.0 — shell-quoting-aware command splitting for browser/editor config values (`src/browser/opener.rs`). Replaces former hand-rolled `split_command`. Used with `validate_program` allow-list check.

**Clipboard:**
- `arboard` 3.6.1 (features: `wayland-data-control`) — cross-platform clipboard (X11 + Wayland primary path)

**Syntax & Markup:**
- `syntect` 5.2 (features: `default-syntaxes`, `default-themes`, `regex-onig`) — syntax highlighting (`src/ui/syntax.rs`)
- `tui-markdown` 0.3 — Markdown rendering in TUI panes
- `pulldown-cmark` 0.13 — Markdown → HTML for browser preview

**PDF Export (feature-gated, default ON):**
- `typst` 0.14.2 — pure-Rust document typesetting engine
- `typst-pdf` 0.14.2 — PDF output backend
- `typst-library` 0.14.2 — Typst standard library
- `typst-kit` 0.14.2 (features: `fonts`) — font loading utilities
- `comemo` 0.4 — memoization required by `typst::World` trait
- `ecow` 0.2 — `EcoString`/`EcoVec` used in Typst API

**Serialization:**
- `serde` 1.0.219 (features: `derive`) — serialization framework
- `serde_yaml_ng` 0.9 — YAML config parsing

**CLI:**
- `clap` 4.5.37 (features: `derive`, `env`) — CLI argument parsing (`--check-update`, `--fake-version`, `--update-to`, `--config`)

**Temp Files (direct dependency, promoted from transitive):**
- `tempfile` 3.24.0 — secure temp file creation via `tempfile::Builder` with `O_EXCL`. Used in `src/browser/pdf_export.rs` (`default_preview_file`). Replaces former predictable `/tmp/{name}.html` path (SEC-04/CR-03 fix).

**Version Parsing (promoted to direct dependency in v0.89.0):**
- `semver` 1.0.27 — semantic version parsing and comparison. Used in self-update logic to compare current vs GitHub release versions. **Promoted from transitive-only to direct `[dependencies]` in v0.89.0** to make the dependency explicit and pin-able.

**Self-Update:**
- `self_update` 0.42.0 (features: `archive-tar`, `archive-zip`, `compression-flate2`, `compression-zip-deflate`, `rustls`, `signatures`) — downloads and applies binary updates from GitHub Releases. The `signatures` feature pulls in `zipsign-api` 0.1.5 (transitive) for ZIP signature verification.
- `rustls` — TLS backend (via self_update); no OpenSSL dependency.

**Utilities:**
- `anyhow` 1.0.98 — error handling and propagation
- `regex` 1.12 — pattern matching (git status parsing, version extraction)
- `libc` 0.2 — `localtime_r`/`localtime_s` for local date formatting in PDF export (`src/browser/pdf_export.rs`)
- `dirs` 5.0 — platform home/download directory resolution

## Configuration

**Environment:**
- Config loaded from (priority order): `./config.yaml` → `~/.config/claude-workbench/config.yaml` → built-in defaults
- No `.env` file; no environment variable-based secrets
- Shell used by user terminal pane: configured via `terminal.shell_path` in `config.yaml`

**Build:**
- `Cargo.toml` — single source of truth
- `Cargo.lock` — committed (binary crate, ensures reproducible builds)
- Feature flags: `pdf-export` (default enabled)

## Platform Requirements

**Development:**
- Rust toolchain (edition 2021)
- `cargo build` / `cargo run`
- `cargo clippy`, `cargo fmt` for linting/formatting

**Production:**
- Self-contained binary; no runtime dependencies except:
  - Optional: `lazygit` binary (LazyGit pane)
  - Optional: `claude` CLI binary (Claude pane)
  - Optional: `git` binary (git status integration)
  - Optional: clipboard helpers (`xclip`, `xsel`, `wl-copy`, `wl-paste`) on Linux
  - Optional: system browser (`open` / `xdg-open`) for file preview

---

*Stack analysis: 2026-05-11*
