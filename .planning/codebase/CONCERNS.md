# Codebase Concerns

**Analysis Date:** 2026-05-11

---

## Security Findings (from SECURITY-NOTES.md — audit 2026-05-11)

### Self-Update Supply-Chain: No Signature Verification (HIGH — open)

- Issue: `src/update/install.rs` uses `self_update` with no checksum or signature verification. Downloads are over HTTPS only. A compromised GitHub release asset installs an arbitrary binary silently with the running user's privileges. Tar-slip protection is entirely delegated to the `self_update` crate.
- Files: `src/update/install.rs`, `Cargo.toml` (`self_update = { version = "0.42", features = ["signatures"] }`)
- Impact: Full binary replacement attack vector on every auto-update. Silent, no user confirmation.
- Fix approach: Two-phase rollout documented in `SECURITY-NOTES.md`:
  1. Sign CI release archives with `zipsign` / ed25519; store private key as GitHub Actions secret; commit public key to `signing/claude-workbench-pub.bin`.
  2. Enable `.verifying_keys([RELEASE_VERIFYING_KEY])` in `src/update/install.rs` using `include_bytes!`. Must ship signing first, then verification, never reversed. The `signatures` feature flag is already present in `Cargo.toml` — only wiring remains.

### Browser/Editor Command Construction: No Allow-List (MEDIUM — open)

- Issue: `src/browser/opener.rs:83-106` splits `config.ui.browser` / `config.ui.editor` strings via a hand-rolled `split_command()` and passes the first token directly to `std::process::Command::new()`. No validation of the program name against an allow-list or path pattern.
- Files: `src/browser/opener.rs` (functions `open_file_with_browser`, `open_file_with_editor`, `split_command`)
- Impact: Low-risk today (config is user-owned), but any future code path that derives the browser/editor field from PTY output, a URL, or a remote source becomes command injection.
- Fix approach: After `split_command`, validate the first token matches `^[A-Za-z0-9_./-]+$` and resolves to an absolute path or a `$PATH` entry. Reject anything else with a clear error.

### Shell Fallback in Dependency Probe (MEDIUM — open)

- Issue: `src/setup/dependency_checker.rs:172-191` builds a shell command string with `shlex::try_quote`, then passes it to `$SHELL -i -c "<cmd>"`. The pattern is fragile: one careless future caller passing PTY-derived text as `args` and a `shlex` edge case yields shell injection.
- Files: `src/setup/dependency_checker.rs` (function `check_dependency_via_shell`, lines ~172-191)
- Current state: All static call sites today are safe. `shlex` 1.3 migration is complete (replaced unmaintained `shell-escape`).
- Fix approach: Replace `$SHELL -i -c` with `Command::new(name).args(args)` for binary lookups. The `-i` flag is only needed to resolve shell aliases/functions, which is unnecessary for the dependency probe's purpose.

### Predictable Temp File Path (MEDIUM — open)

- Issue: `src/browser/pdf_export.rs:115-119` constructs temp paths as `$TMPDIR/{stem}-{dd.mm.yyyy}.html` and `$TMPDIR/{project}-{stem}-{dd.mm.yyyy}.{ext}`. Paths are guessable. On a multi-user system, a local attacker can pre-create the path as a symlink and redirect the write.
- Files: `src/browser/pdf_export.rs` (lines ~107-140)
- Fix approach: Replace with `tempfile::Builder::new().prefix(stem).suffix(".html").tempfile_in(env::temp_dir())?`. The `tempfile` crate is already in `Cargo.toml` and opens with `O_EXCL`.

### Closed: shell-escape Replaced (RESOLVED)

- `shell-escape` (unmaintained) removed. Replaced with `shlex` 1.3 in `src/app/pty.rs` and `src/setup/dependency_checker.rs` as of v0.89.0.

---

## Dependency Debt

### crossterm Pinned at 0.28.1 (BLOCKED — open)

- Issue: `Cargo.toml` pins `crossterm = "0.28.1"` with an inline comment: "tui-textarea fork branch `update-ratatui` targets crossterm 0.28's Event types via its `From<Event> for Input` impl. Bumping to 0.29 yields a version mismatch in `editor.input(Event::Key(...))` call sites."
- Files: `Cargo.toml`, `tui-textarea = { git = "https://github.com/0xferrous/tui-textarea.git", branch = "update-ratatui" }`
- Impact: Cannot pick up crossterm 0.29 bug fixes or new terminal compatibility improvements. The fork dependency is git-pinned to a branch, not a tag or revision — branch tip can change unpredictably.
- Scaling limit: crossterm 0.29 introduced breaking `Event` type changes. Unblocking requires either the fork to merge upstream changes, or a local patch/fork of `tui-textarea`.
- Fix approach: Watch `https://github.com/0xferrous/tui-textarea` for upstream merge. Alternatively, fork `tui-textarea` under the project's own control and apply the 0.29 compatibility patch directly. Pin to a git commit hash rather than a branch name to prevent silent drift.

### tui-textarea: Git Branch Dependency (FRAGILE)

- Issue: `tui-textarea` is sourced from a git branch (`branch = "update-ratatui"`), not a versioned crate. Branch tips are mutable; `Cargo.lock` pins the commit hash, but any `cargo update` will silently advance it.
- Files: `Cargo.toml` line 14
- Fix approach: Pin to a specific `rev = "<sha>"` in `Cargo.toml`. Document the sha and reason. Revisit when a proper crates.io release is available.

---

## Code Smells

### panic! in Non-Test Update Code

- Issue: `src/update/mod.rs` lines 220-223 contain `panic!("GitHub API error: {}", e)` and `panic!("No releases found for platform: ...")` that are inside `#[ignore]` integration tests but are physically in the `src/update/mod.rs` test module. Not reachable in production. No panic sites exist in non-test production code paths.
- Files: `src/update/mod.rs` (lines 210-313, all within `#[cfg(test)]` `#[ignore]` blocks)
- Impact: Low — only reachable via `cargo test -- --include-ignored`.

### .expect() in Production Code

All production `expect()` calls are in low-risk or justified positions:

- `src/main.rs:289` — `"127.0.0.1:9998".parse().expect("hardcoded address parses")` — infallible parse of a compile-time literal. Acceptable.
- `src/main.rs:339` — `.expect("Failed to create tokio runtime")` — unrecoverable startup failure. Acceptable (process cannot function without a runtime).
- `src/clipboard.rs:210` — `.expect("spawn clipboard worker thread")` — OS thread spawn failure at startup. Acceptable; process cannot function without the clipboard worker.
- `src/filter.rs:52-135` (28 sites) — all `Regex::new(...).expect("static regex pattern must compile")` on compile-time literals. These are infallible in practice; consider using the `once_cell` / `std::sync::LazyLock` pattern with `Regex::new(...).unwrap()` for consistency, or a `static` initialized at startup.

### Mutex Poison Recovery

- Issue: `src/terminal.rs:11-15` defines `lock_or_recover()` which calls `unwrap_or_else(|poisoned| poisoned.into_inner())` to silently recover from mutex poisoning. This means a background PTY thread panic will be silently swallowed rather than propagated.
- Files: `src/terminal.rs` (function `lock_or_recover`, used throughout the file)
- Impact: If the PTY reader thread panics, the main UI thread will continue using potentially corrupt parser state without any indication. `exited: Arc<AtomicBool>` provides a separate signal but it is not checked on every lock.
- Fix approach: Log the poison recovery event at minimum. Consider surfacing it as a PTY error in the pane border (same mechanism as `claude_error`/`lazygit_error`).

### App Struct God Object (40+ fields)

- Issue: `src/app/mod.rs:68-167` — `App` has 47 public fields covering PTY handles, UI flash state, async job receivers, drag state, mouse selection, clipboard warnings, export state, SSH hints, and more. All state lives in one struct with no sub-grouping beyond field comments.
- Files: `src/app/mod.rs`
- Impact: High cognitive load. Adding features requires understanding the full struct. Test isolation is impossible without constructing the entire `App`.
- Fix approach: Group logically related fields into sub-structs (`ClipboardState`, `UpdateUiState`, `FlashState`, `ExportState`). The `JobState<T>` refactor (v0.89.0) is a good example of this pattern already applied.

### Session Persistence Stub

- Issue: `src/session.rs` is entirely stubbed. `save_session()` is a no-op; `load_session()` always returns `SessionState::default()`. The struct has a comment `// Add other session data` with no fields.
- Files: `src/session.rs`
- Impact: Working directory, pane layout, and any user preferences set in a session are lost on every restart. CLAUDE.md documents session persistence as "stubbed."
- Fix approach: Implement JSON/YAML serialization using `serde` (already in `Cargo.toml`) to `~/.config/claude-workbench/session.yaml`.

---

## Performance

### Clipboard Worker Thread Architecture (v0.87.0+)

- Design: `src/clipboard.rs` runs `copy_to_clipboard_async()` on a dedicated OS thread (spawned at startup via `.expect("spawn clipboard worker thread")`). Copy operations are submitted via a channel; the worker serializes them and enforces a `SUBPROCESS_TIMEOUT` of 500ms per subprocess call. Paste (`copy_from_clipboard_sync()`) remains synchronous on the main event loop thread.
- Current state: Handles XRDP/Kitty pathologies where `arboard`'s wayland-data-control feature stalls indefinitely and `xclip`/`xsel` can block beyond the subprocess timeout.
- Residual concern: Paste is still synchronous. If `arboard` stalls during paste in an XRDP session and neither `xclip -o` nor `xsel -b -o` is available, the main event loop blocks for up to 500ms per fallback attempt.
- Files: `src/clipboard.rs` (lines 162-228 for worker design, 391-413 for timeout enforcement)

### vt100 Parser: Fixed 1000-Line Scrollback Buffer per Pane

- Issue: `src/terminal.rs:86` initializes each `vt100::Parser` with a fixed 1000-line scrollback. Three PTY panes = ~3000 lines of vt100 screen cells held in memory permanently, regardless of content density.
- Files: `src/terminal.rs:83-87`
- Impact: Memory grows proportional to terminal output density (color spans, wide characters). No mechanism to tune this per pane or reduce it at runtime.
- Fix approach: Make scrollback size configurable in `config.yaml` under `terminal.scrollback_lines`. Default 1000 is reasonable but heavy for low-memory targets.

### 16ms Event Loop Polling

- Design: `App::run()` polls crossterm events with a 16ms timeout (≈60fps). On idle sessions with no input and no PTY output, this generates ~62 wakeups/second.
- Files: `src/app/mod.rs` (event loop), documented in CLAUDE.md
- Impact: Minimal on modern hardware. Could matter in SSH sessions over metered connections if terminal resize events are generated on each tick.

---

## Fragile Areas

### XRDP/SSH Clipboard Compatibility

- Why fragile: Three separate failure modes exist that required fixes across v0.86.x and v0.87.0:
  1. `ButtonRelease` events swallowed by XRDP — caused stuck mouse selections (fixed v0.86.3).
  2. `arboard` wayland-data-control stalling the UI thread under XRDP-X11 (fixed v0.87.0 via async worker).
  3. `xclip` blocking indefinitely when X server is unresponsive (fixed v0.86.4 via subprocess timeout).
- Files: `src/clipboard.rs`, `src/app/mouse.rs`, `src/app/clipboard.rs`
- Safe modification: Any change to clipboard strategy selection (`determine_clipboard_strategy()` in `src/clipboard.rs:85-115`) must be tested in an XRDP session. The `XRDP_SESSION` and `XDG_SESSION_TYPE` environment variable checks are the detection heuristic — removing or reordering them will regress XRDP users.
- Test coverage: No automated tests for clipboard behavior under XRDP. Regressions only catchable via manual testing on an XRDP host or via the `--clipboard-diag` CLI flag.

### Mouse Selection Boundary Logic

- Why fragile: `src/app/mouse.rs` (1072 lines) contains character-level selection logic constrained to pane boundaries. The `is_inside(rect, x, y)` closure pattern is duplicated across multiple event handlers. Mouse coordinate math depends on border widths (1px each side = -2 from content area), which is also assumed in PTY resize logic in `src/terminal.rs`.
- Files: `src/app/mouse.rs`, `src/ui/terminal_pane.rs`
- Safe modification: Any layout change (border thickness, new panes) requires auditing both the resize border accounting (`-2` in terminal resize) and the hit-test math in `mouse.rs`. These are not co-located.

### PTY Resize Timing

- Why fragile: PTY resize runs on every `draw()` call before rendering (CLAUDE.md: "PTY resize happens during every `draw()` call"). If PTY resize fails (e.g., `portable-pty` error), the failure is silently ignored and the terminal dimensions drift from the UI.
- Files: `src/app/drawing.rs`
- Safe modification: Do not add expensive work to the resize path. Any resize error should at minimum be logged to the update log file at `/tmp/claude-workbench-update.log`.

### Browser Preview Temp File Cleanup

- Why fragile: `src/app/mod.rs:161` tracks temp preview files in `pub temp_preview_files: Vec<std::path::PathBuf>`. Cleanup happens on exit. If the process is killed (SIGKILL) or panics, temp files in `$TMPDIR` accumulate.
- Files: `src/app/mod.rs`, `src/browser/pdf_export.rs`
- Safe modification: Switching to `tempfile::NamedTempFile` (which auto-deletes on drop) would solve this without requiring manual tracking, and also fixes the predictable path security finding above.

---

## Test Coverage Gaps

### Clipboard Layer: No Automated Tests for Subprocess Fallback Chain

- What's not tested: The `SubprocessFirst` clipboard strategy (xclip → xsel → wl-copy fallback chain), the async worker handoff, and the 500ms timeout enforcement.
- Files: `src/clipboard.rs`
- Risk: Clipboard regressions under XRDP or when specific tools are absent go undetected until user reports.
- Priority: High — this area has had 3 separate regressions across 4 patch versions.

### PTY/Terminal: No Unit Tests

- What's not tested: `PseudoTerminal` initialization, `lock_or_recover` poison handling, scrollback behavior, PTY resize propagation.
- Files: `src/terminal.rs`
- Risk: Changes to the PTY threading model (e.g., the `PtyCallbacks` DSR/DA response mechanism) cannot be validated without running the full TUI.
- Priority: Medium.

### Mouse Selection: No Unit Tests

- What's not tested: Character-level selection boundary clamping, pane hit detection, drag state transitions.
- Files: `src/app/mouse.rs` (1072 lines, zero test functions)
- Risk: Any layout change silently breaks selection without a failing test.
- Priority: Medium.

### Update/Self-Update: Integration Tests Require Network and Are `#[ignore]`d

- What's not tested by default: `test_github_release_accessible`, `test_release_notes_fetchable`, `test_update_check_with_fake_version` — all marked `#[ignore]`.
- Files: `src/update/mod.rs`
- Risk: CI never runs these. A GitHub API format change or rename of release assets would be undetected until a user reports a broken auto-update.
- Priority: Low (network tests are inherently flaky in CI, but a mock-based unit test for the version-parsing logic would be low-cost and valuable).

### Session Persistence: Not Tested (Stub)

- What's not tested: `save_session` and `load_session` are no-ops; there is nothing to test. This is a consequence of the stub implementation.
- Files: `src/session.rs`
- Priority: Low until the stub is implemented.

---

## Scaling Limits

### tui-textarea Fork Blocks crossterm Upgrade

- Current capacity: Works correctly on crossterm 0.28.1.
- Limit: Cannot adopt crossterm 0.29+ until the `0xferrous/tui-textarea` fork is updated or replaced. Any crossterm 0.29 security/compatibility fix is inaccessible.
- Scaling path: Pin the fork to a specific git revision (`rev = "<sha>"`). Evaluate upstreaming the ratatui 0.30 compatibility patch to `rhysd/tui-textarea` directly.

### App Struct at 47 Fields

- Current capacity: Manageable with the current feature set.
- Limit: Each new feature adds fields directly to `App`. The struct is already at the boundary where adding one more cross-cutting concern (e.g., a second async PTY resize job) requires understanding all 47 existing fields to avoid conflicts.
- Scaling path: Extract sub-structs as described in the Code Smells section above.

---

*Concerns audit: 2026-05-11*
