# Codebase Concerns

**Analysis Date:** 2026-05-11

---

## Resolved in Wave 1

The following findings from the pre-Wave-1 audit were addressed by Phase 1 Wave 1 security hardening. They are recorded here for traceability only.

| ID | Finding | Fix | Commit |
|----|---------|-----|--------|
| CR-03 / SEC-04 | Predictable temp paths (race window, symlink attack) | `tempfile::Builder` with `O_EXCL` | 4b84723, 4a0feae |
| CR-02 | `--update-to` flag available in release builds | Gated behind `#[cfg(debug_assertions)]` | 93a6d56 |
| IN-02 | Restart re-execs with `--update-to` in argv | `filter_restart_args()` strips debug-only flags | eb6b6cb |
| WR-01 / SEC-02 | `opener.rs` launched arbitrary programs with no allow-list | `validate_program()` enforces an explicit allow-list | 4b6046d |
| WR-02 / SEC-03 | `$SHELL -i -c` fallback in opener could execute arbitrary shell | Fallback removed entirely | 6fe0862 |
| WR-03 | `clipboard` `which()` did not check executable bit | `is_executable()` helper added | 999fc2c |
| WR-04 | `shlex` failures silently fell back to unquoted paths | Error propagated via `quote_path_for_cd()` | 3cb3668 |
| WR-05 | Self-update selected `releases[0]` (API insertion order) instead of semver max | `max_by` semver chain | 3cb3668 |

---

## Open Security

### SEC-01: Self-Update Has No Signature Verification

**Risk:** The self-update path (`src/update/install.rs`) downloads a tarball from GitHub Releases over HTTPS and replaces the running binary with no cryptographic integrity check. A compromised GitHub account, CDN MITM, or accidental release asset corruption would silently install a malicious binary.

**Files:**
- `src/update/install.rs` — download + atomic replace (no verification step)
- `src/update/check.rs` — release metadata fetch
- `.planning/phases/01-security-hardening/01-05-PLAN.md` — Wave 2: CI signing via zipsign ed25519
- `.planning/phases/01-security-hardening/01-06-PLAN.md` — Wave 3: client verification using embedded `pub.bin`

**Current state:** Neither Wave 2 (CI signs archives) nor Wave 3 (client calls `verifying_keys`) has been executed. No `RELEASE_VERIFYING_KEY` constant, no `zipsign` dependency, no `signing/claude-workbench-pub.bin` exist in the tree.

**Fix approach:**
1. Wave 2 (01-05-PLAN.md): Add `zipsign sign` step to `.github/workflows/release.yml`; commit `signing/claude-workbench-pub.bin` (public key only).
2. Wave 3 (01-06-PLAN.md): Add `zipsign` as Cargo dependency; embed pub key via `include_bytes!`; call `verifying_keys()` in `install.rs` before extracting archive.

**Priority:** High — affects all users who use the built-in updater.

---

## Dependency Debt

### DEP-01: crossterm Pinned at 0.28.1 — Blocks Upstream Updates

**Files:**
- `Cargo.toml` line 8: `crossterm = "0.28.1"`

**Root cause:** `tui-textarea` is sourced from a community fork (`github.com/0xferrous/tui-textarea`, branch `update-ratatui`) whose `From<Event> for Input` impl targets crossterm 0.28 event types. Bumping to crossterm 0.29 causes type-mismatch compile errors at `editor.input(Event::Key(...))` call sites.

**Impact:** crossterm 0.29 and 0.30 contain terminal-handling fixes (including XRDP and Kitty improvements). Being pinned blocks those fixes and may cause transitive dependency conflicts as ratatui moves forward.

**Fix approach (Phase 3):** Either upstream the fork into `tui-textarea` proper (preferred), switch to `tui-textarea`'s official crate once it tracks crossterm 0.29+, or vendor and patch locally. Requires coordinated bump of `crossterm`, `tui-textarea`, and any other event-type consumers.

---

## Code Quality

### REFAC-01: `App` Struct Is a God Object (50+ Fields)

**File:** `src/app/mod.rs` — `pub struct App` at line 68, running to line ~166.

**Field count:** ~50 public/private fields covering PTY terminals, file browser state, preview state, all dialog states, clipboard flash state, drag state, mouse selection, git remote jobs, update jobs, dependency report, export state, and more.

**Impact:** Every feature change requires reading the entire struct. New contributors cannot understand ownership boundaries. Adding fields has zero friction, which is why the count keeps growing.

**Fix approach (Phase 3):** Extract cohesive sub-structs — e.g., `UpdateUiState` (update_state + update_check_job + update_job + update_dialog_button + update_dialog_areas), `ClipboardUiState` (clipboard_error_flash + clipboard_warning + clipboard_warning_dismissed), `FlashState` (last_autosave_time + last_copy_time + copy_flash_*), `SelectionState` (terminal_selection + drag_state + mouse_selection). App holds sub-structs, not individual fields.

### QUAL-02: `lock_or_recover()` Silently Swallows Mutex Poison

**File:** `src/terminal.rs` lines 8–14.

```rust
fn lock_or_recover<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}
```

**Impact:** If the background PTY reader thread panics while holding the parser lock, the parser state may be partially corrupted. `lock_or_recover` silently recovers and continues rendering corrupted vt100 state. There is no log, no metric, and no way to detect this post-hoc. The same pattern appears in `src/ui/terminal_pane.rs:138`.

**Fix approach (Phase 2):** Log the poison recovery (at minimum `eprintln!` to the update log, ideally a dedicated internal error channel). Consider resetting the parser to a clean state on recovery rather than using potentially corrupt inner data.

### 15 Pre-existing `collapsible_match` Clippy Warnings

`cargo clippy` reports 15 warnings (confirmed 2026-05-11), all `clippy::collapsible_match`. Locations:

- `src/app/file_ops.rs`: lines 595, 604, 609, 617, 624, 632, 647
- `src/app/keyboard/dialogs.rs`: lines 113, 118
- `src/app/mouse.rs`: lines 790, 798
- `src/browser/typst_pdf.rs`: lines 323, 328, 418, 454

These are mechanical fixes (`cargo clippy --fix` can auto-apply all 15). Deferred to a cleanup phase; they do not affect correctness.

### Deferred Info-Severity Findings (from 01-REVIEW.md)

#### IN-01: OSC 52 Clipboard Always Reports Success

**File:** `src/clipboard.rs` line 309:
```
// Last resort: OSC 52. We always claim success here because we can't
// verify whether the terminal forwarded it
```

`ClipboardOutcome::Osc52` is returned even when the terminal silently discards the sequence (e.g., terminal has OSC 52 disabled). Users see no error. This is architecturally constrained (OSC 52 has no synchronous response path), but the UX could be improved with a one-time warning that OSC 52 is unverifiable.

**Impact:** Low — cosmetic/UX. User may think copy succeeded when it didn't.

#### IN-03: `localtime_r` Cast Has No Bounds Guard on 64-bit for Years > 9999

**File:** `src/ui/file_browser.rs` lines 22–27.

The constant `MAX_SAFE_TIMESTAMP = 253_402_300_799` caps at year 9999, and 32-bit platforms get an explicit `i32::MAX` check. However the `utc_secs as libc::time_t` cast on 64-bit platforms is unchecked between `i32::MAX` and `MAX_SAFE_TIMESTAMP` — on a 64-bit platform where `time_t` is `i64`, this range is safe, so the risk is theoretical. The identical pattern exists in `src/browser/pdf_export.rs:86–95` and `src/browser/typst_pdf.rs:168`. No action needed before Phase 3 review.

---

## Test Coverage Gaps

### QUAL-01: Clipboard Fallback Chain — No Integration Tests

**Files:**
- `src/clipboard.rs` — `try_xclip_copy()` (line 362), `try_xsel_copy()` (line 373), `try_xclip_paste()` (line 395), `try_xsel_paste()` (line 400), `osc52_copy()` (line 476)

**Current coverage (Wave 1 additions):** 22 unit-level `#[test]` functions exist in `src/clipboard.rs`, covering: base64 encode, outcome labels, `which()` finding binaries, `is_executable()` mode check, SSH session detection, and `diag_collect()` not panicking.

**Gap:** The *fallback chain itself* — the decision tree that tries xclip → xsel → wl-copy → OSC 52 in sequence — has no test. If `try_xclip_copy` silently returns `None` due to a wrong exit code interpretation, the chain falls through to OSC 52 with no signal. No test covers: "xclip present but returns error → falls to xsel", "all X11 helpers absent → OSC 52 used", or "paste returns None from all helpers → empty string returned".

**Fix approach (Phase 2):** Add integration tests using a mock `PATH` that substitutes fake `xclip`/`xsel` scripts returning controlled exit codes. Use `std::env::set_var("PATH", ...)` in test setup with a temp dir containing stubs. Test each fallback step individually and the full chain.

**Priority:** Medium — XRDP environments where the chain matters most are also the environments most likely to break silently.

---

## Stub

### FEAT-01: Session Persistence Is a No-Op

**File:** `src/session.rs`

```rust
pub fn save_session(_state: &SessionState) {
    // Implement save logic
}

pub fn load_session() -> SessionState {
    // Implement load logic
    SessionState::default()
}
```

`SessionState` has one field: `pub last_cwd: String`. Neither function does anything. The application boots to `$HOME` every time regardless of where the user was working.

**Impact:** Low functional impact currently. Becomes blocking once any feature relies on cross-session state (e.g., remembered pane layout, last opened file).

**Fix approach (Phase 4):** Serialize to `~/.config/claude-workbench/session.json` using the already-imported `serde`/`serde_json`. Save on quit, load in `App::new`. Guard against corrupt files with `load_session() -> Option<SessionState>` fallback.

---

## Performance / Fragile Areas

### XRDP Clipboard Pathology (Historical Context)

**Files:**
- `src/clipboard.rs` — async worker thread, `ClipboardStrategy`, `Osc52` fallback
- `src/app/mod.rs` — `poll_clipboard_outcome()` integrated in event loop

**Background:** Under XRDP+X11 sessions, two distinct failure modes were observed and fixed across v0.86.x–v0.87.0:

1. **ButtonRelease swallowed** (v0.86.3/v0.86.4): XRDP's X server consumed `ButtonRelease` events during clipboard negotiation, causing the left-click drag selection to stick. Fixed by sending synthetic `ButtonRelease` on timeout.
2. **xclip blocking** (v0.87.0): `xclip` and `xsel` called synchronously from the UI thread caused the entire TUI to freeze for 5–30 seconds when the X clipboard owner (e.g., a disconnected XRDP session) never responded. Fixed by moving clipboard writes to an async worker thread (`ClipboardWorker` in `src/clipboard.rs`).

**Residual fragility:** The async worker thread is not supervised. If the worker thread panics (e.g., during `NamedTempFile::new().unwrap()` at line 639 — an `unwrap()` in non-test code), the clipboard channel becomes permanently closed and all subsequent copies silently fail with `ClipboardOutcome::Osc52` fallback. There is no worker restart logic.

**Mitigation:** The `CLAUDE_WORKBENCH_CLIPBOARD=osc52` environment variable bypasses the subprocess chain entirely and is documented as the escape hatch for broken XRDP environments.

---

*Concerns audit: 2026-05-11*
