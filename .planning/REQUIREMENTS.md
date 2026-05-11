# Requirements: claude-workbench

**Defined:** 2026-05-11
**Core Value:** Stay in one terminal: file navigation, Claude Code, LazyGit, and a shell side-by-side, with the panes always pointing at the same working directory.

## v1 Requirements

Active improvements derived from the v0.89.0 audit (`.planning/codebase/CONCERNS.md` + `SECURITY-NOTES.md`). Each maps to a roadmap phase below.

### Security Hardening

- [ ] **SEC-01**: Self-update signature verification path is hardened — finish HIGH-severity finding (binary integrity end-to-end: download → verify signature → swap; reject unsigned or mismatched) — *Wave 2 gated on operator keypair; Wave 3 gated on 2+ signed releases*
- [x] **SEC-02**: Address MEDIUM finding #1 from `SECURITY-NOTES.md` — *Wave 1: validate_program() allow-list + shlex::split in opener.rs (commit 4b6046d)*
- [x] **SEC-03**: Address MEDIUM finding #2 from `SECURITY-NOTES.md` — *Wave 1: $SHELL -i -c fallback removed from dependency_checker.rs (commit 6fe0862)*
- [x] **SEC-04**: Address MEDIUM finding #3 from `SECURITY-NOTES.md` — *Wave 1: tempfile::Builder with O_EXCL replaces predictable temp paths in pdf_export.rs (commit 4b84723)*

### Test Coverage

- [ ] **QUAL-01**: Clipboard subprocess fallback chain has automated tests covering xclip / pbcopy / cc-clip paths and XRDP failure modes (currently 0 coverage despite 3 regressions in 4 patch versions)
- [ ] **QUAL-02**: Mutex-poison events are at minimum logged (replace silent `lock_or_recover()` swallow) and surfaced in a way that can be triaged in production

### Code Quality / Refactor

- [ ] **REFAC-01**: `App` god-struct (47 fields) is decomposed into composed sub-states with at least one isolated, testable sub-domain extracted

### Dependencies

- [ ] **DEP-01**: A documented crossterm 0.29 upgrade path exists (either tui-textarea fork updated/replaced, or alternative textarea identified) — outcome may be a tracked deferral, not a hard upgrade

### Feature Completion

- [ ] **FEAT-01**: `src/session.rs` saves and restores at least the file-browser working directory across launches (no more no-op stub)

## v2 Requirements

Deferred — captured for future roadmap cycles.

### Platform

- **PLAT-01**: Windows native support (currently Linux + macOS only)

### Distribution

- **DIST-01**: Homebrew tap automation for macOS users
- **DIST-02**: AUR / .deb / .rpm packages for Linux distros

### Features

- **FEAT-02**: Session state beyond cwd (pane focus, scrollback position, open file in preview)
- **FEAT-03**: Configurable layout (resize panes, swap positions)
- **FEAT-04**: Theme system beyond the current `theme: default` placeholder

## Out of Scope

| Feature | Reason |
|---------|--------|
| Multi-host remoting / built-in SSH client | Users have their own SSH; we're a local cockpit |
| Plugin system | Surface area too large; features stay in-tree |
| Embedded LSP | Preview uses syntect — full LSP belongs in real editors |
| Non-TUI frontend | TUI is the product, not a render target |
| Windows native (v1) | Linux + macOS only for v1; deferred to PLAT-01 |

## Traceability

Updated during roadmap creation.

| Requirement | Phase | Status |
|-------------|-------|--------|
| SEC-01 | Phase 1 (Wave 2+3) | Gated (operator + signed releases) |
| SEC-02 | Phase 1 (Wave 1) | ✓ Done — 4b6046d |
| SEC-03 | Phase 1 (Wave 1) | ✓ Done — 6fe0862 |
| SEC-04 | Phase 1 (Wave 1) | ✓ Done — 4b84723 |
| QUAL-01 | Phase 2 | Pending |
| QUAL-02 | Phase 2 | Pending |
| REFAC-01 | Phase 3 | Pending |
| DEP-01 | Phase 3 | Pending |
| FEAT-01 | Phase 4 | Pending |
