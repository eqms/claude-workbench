# External Integrations

**Analysis Date:** 2026-05-11

## APIs & External Services

**GitHub Releases API (self-update):**
- Service: GitHub Releases (`api.github.com`)
- SDK/Client: `self_update` 0.42.0 with `rustls` TLS (no OpenSSL)
- Auth: None (public repo, unauthenticated reads)
- Signature verification: `zipsign-api` 0.1.5 (transitive via `self_update` `signatures` feature)
- Trigger: at startup (silent on error) + manual via `u` key in Help screen (F12)
- CLI flags: `--check-update`, `--update-to <version>`, `--fake-version <version>`
- Log: `/tmp/claude-workbench-update.log`
- Asset naming convention: `claude-workbench-{target}.tar.gz`
  - Targets: `aarch64-apple-darwin`, `x86_64-apple-darwin`, `aarch64-unknown-linux-gnu`, `x86_64-unknown-linux-gnu`

## Data Storage

**Databases:** None

**File Storage:**
- User config: `~/.config/claude-workbench/config.yaml` (read at startup)
- Session state: `src/session/mod.rs` — stubbed, returns default state (no persistence yet)
- PDF/Markdown export: user-configured export directory (default: `~/Downloads`), resolved in `src/browser/pdf_export.rs:resolve_export_dir`
- Browser preview temp files: kernel-enforced exclusive `O_EXCL` via `tempfile::Builder` in `src/browser/pdf_export.rs:default_preview_file`. Format: `{project}-{stem}-{random}.html` in `std::env::temp_dir()`. `NamedTempFile` auto-deletes on drop. Replaces former predictable path (SEC-04/CR-03 fix, v0.89.0).

**Caching:** None

## Authentication & Identity

**Auth Provider:** None — no user accounts, no tokens stored by the application itself.

## External Process Integrations

**Claude CLI (`claude` binary):**
- Launched as PTY child process in the Claude pane (`src/app.rs`, `PaneId::Claude`)
- Shell: `/bin/bash -c "echo 'Claude Code PTY'; exec bash"`
- Inherits full parent environment (HOME, PATH, LANG, etc.) — required for Claude CLI auth
- Detected at startup via `src/setup/dependency_checker.rs:check_command("claude", &["--version"], false)`
- Fallback path lookup: `~/.claude/local/claude`, `/usr/local/bin/claude`, `~/.local/bin/claude`

**LazyGit (`lazygit` binary):**
- Launched as PTY child process in the LazyGit pane (`src/app.rs`, `PaneId::LazyGit`)
- Command: `lazygit` (no args; starts in current directory)
- Detected at startup via `src/setup/dependency_checker.rs:check_command("lazygit", &["--version"], false)`

**Git (`git` binary):**
- Invoked directly (not via shell) for git status coloring in file browser
- Required dependency; startup check: `check_command("git", &["--version"], true)`

**System Browser / File Opener (`open` / `xdg-open` / `explorer`):**
- Entry point: `src/browser/opener.rs`
- macOS: `open <path>`
- Linux: `xdg-open <path>`
- Windows: `cmd /c start "" <path>` or `explorer <path>`
- User-configurable browser/editor commands: parsed with `shlex::split` then validated through `validate_program` allow-list
- `validate_program` accepts only: ASCII alphanumerics, `_`, `-`, `.`, `/`, `+` in the program name
- Shell metacharacters (`;`, `|`, `&`, `$`, `` ` ``, `(`, `)`, spaces) are **rejected with error** — no shell injection possible
- This is a **v0.89.0 security hardening** change (SEC-02/WR-01): replaces former hand-rolled `split_command` that had no validation

**User Shell (terminal pane):**
- Configured via `terminal.shell_path` in `config.yaml` (default: `/bin/bash`)
- Launched as PTY child process in the Terminal pane

## Clipboard Fallback Chain

Detected at startup via `src/setup/dependency_checker.rs`. Detection uses pure-Rust PATH lookup (`crate::clipboard::which`) — **no shell invocation** for clipboard helpers. This avoids the former `$SHELL -i -c` pattern that triggered fish job-control corruption on macOS startup (SEC-03/WR-02 fix, v0.89.0).

Priority order (runtime, not startup):
1. `arboard` 3.6.1 — primary cross-platform clipboard (X11 native + Wayland via `wayland-data-control` feature)
2. `wl-copy` / `wl-paste` — Wayland fallback helper binaries (both must be present)
3. `xclip` — X11 fallback helper binary
4. `xsel` — X11 fallback helper binary
5. OSC 52 escape sequence — last resort (XRDP / headless environments)

Status of each helper reported in F12 Help screen (`ClipboardHelpers` struct in `src/setup/dependency_checker.rs`).

**`cc-clip` integration:**
- Custom clipboard helper for XRDP environments
- Detected via same PATH lookup mechanism
- Part of fallback chain for XRDP sessions where standard X11 clipboard helpers block or fail

## Monitoring & Observability

**Error Tracking:** None (no external service)

**Logs:**
- Self-update operations: `/tmp/claude-workbench-update.log`
- All other errors: stderr / anyhow error chain surfaced in UI

## CI/CD & Deployment

**Hosting:**
- Primary: GitLab (`gitlab.ownerp.io/ki/workbench`) — `origin` remote
- Open Source: GitHub (`github.com/eqms/claude-workbench`) — `upstream` remote
- Both remotes kept in sync on every push

**CI Pipeline:**
- GitHub Actions — builds release binaries for all 4 targets
- Release file: `.github/workflows/release.yml`
- Release action: `softprops/action-gh-release@v2`
- Binary assets published to GitHub Releases (consumed by self-update)

## Environment Configuration

**Required env vars:** None (application has no required env vars; inherits user environment for PTY children)

**Secrets location:** No secrets stored by the application. Claude CLI credentials managed externally by the `claude` binary (typically `~/.claude/`).

## Webhooks & Callbacks

**Incoming:** None

**Outgoing:** None (GitHub API calls are pull-only, no webhooks registered)

---

*Integration audit: 2026-05-11*
