# claude-workbench

## What This Is

A Rust-based TUI (Terminal User Interface) multiplexer that gives developers an integrated cockpit for Claude Code workflows: file browser with preview, three embedded PTY panes (Claude Code, LazyGit, system terminal), mouse + keyboard navigation, and scrollback. Built with Ratatui, Crossterm, and portable-pty. Currently at v0.89.0, distributed as a single binary via GitHub Releases with self-update.

## Core Value

Stay in one terminal: file navigation, Claude Code, LazyGit, and a shell side-by-side, with the panes always pointing at the same working directory. If everything else fails, the three PTY panes must remain reliably interactive and synchronized.

## Requirements

### Validated

<!-- Shipped and confirmed valuable. Inferred from current codebase. -->

- ✓ **CORE-01**: File browser with vim-like navigation and git status colors — existing
- ✓ **CORE-02**: Preview pane with syntax highlighting + markdown rendering — existing
- ✓ **CORE-03**: Three PTY panes (Claude/LazyGit/Terminal) with vt100 emulation + 1000-line scrollback — existing
- ✓ **CORE-04**: Mouse hit-testing across 6 panes with click-to-focus + scroll — existing
- ✓ **CORE-05**: Character-level mouse selection with clipboard auto-copy — existing (v0.41.0)
- ✓ **CORE-06**: Directory sync across panes (file-browser cd propagates to Claude + Terminal) — existing
- ✓ **CORE-07**: Self-update from GitHub Releases with version compare + signed download — existing (signatures feature enabled v0.89.0)
- ✓ **CORE-08**: Browser preview for HTML/Markdown/PDF/images via platform opener — existing (v0.10)
- ✓ **CORE-09**: Fuzzy file finder (Ctrl+P) with syntect-based preview — existing
- ✓ **CORE-10**: Async clipboard worker thread — UI stays responsive when X-server hangs (v0.87.0)
- ✓ **CORE-11**: SSH/XRDP image-paste hint + cc-clip integration — existing (v0.88.0)
- ✓ **CORE-12**: Settings menu with persistent YAML config — existing
- ✓ **CORE-13**: Setup wizard for first-run experience — existing
- ✓ **CORE-14**: Typst-based PDF export pipeline (feature-gated, default-on) — existing
- ✓ **CORE-15**: JobState<T> generic async job state machine — existing (v0.89.0 refactor)
- ✓ **CORE-16**: Modular keyboard.rs split into submodules (dialogs/global/...) — existing (v0.89.0)

### Active

<!-- Improvements derived from CONCERNS.md and outstanding audit findings. -->

- [ ] **SEC-01**: Self-update signature verification — finish hardening (1 HIGH finding open)
- [ ] **SEC-02**: Address 3 MEDIUM security findings from SECURITY-NOTES.md
- [ ] **QUAL-01**: Add automated tests for clipboard subprocess fallback chain (3 regressions in 4 patch versions, currently 0 coverage)
- [ ] **QUAL-02**: Replace silent `lock_or_recover()` mutex-poison swallow with at-minimum logging
- [ ] **REFAC-01**: Break up the 47-field `App` god-struct into composed sub-states
- [ ] **DEP-01**: Plan crossterm 0.29 upgrade path (currently pinned at 0.28.1 — blocked on tui-textarea fork)
- [ ] **FEAT-01**: Implement `src/session.rs` properly — currently a no-op stub for save/restore

### Out of Scope

- **Multi-host remoting** — claude-workbench is local-only; SSH usage is the user's existing shell, not a built-in client
- **Plugin system** — too much surface area; features stay in-tree
- **Embedded LSP** — preview uses syntect only; full LSP belongs in real editors
- **Non-TUI frontend** — TUI is the product, not a render target
- **Windows native support** — Linux + macOS only; Windows users use WSL

## Context

**Technical environment:**
- Single Rust binary, ~50 source files in `src/`, built with cargo, distributed via GitHub Releases (GitLab origin + GitHub upstream)
- Tokio multi-thread runtime + dedicated clipboard worker thread + one reader thread per PTY
- vt100-based terminal emulation with 1000-line scrollback per pane
- Self-update via reqwest + self_update crate with optional signature verification

**Relevant prior work:**
- v0.86.x: Mouse selection and XRDP/Kitty compatibility fixes
- v0.87.0: Async clipboard worker thread (UI stays responsive when X server hangs)
- v0.88.0: SSH image-paste hint + cc-clip integration
- v0.89.0: Project audit follow-through — JobState refactor, keyboard.rs split into submodules, dependency hardening (shlex 1.3, pulldown-cmark 0.13, self_update signatures), mouse focus fix

**Known concerns (from CONCERNS.md):**
- 4 open security findings (1 HIGH, 3 MEDIUM) with file-level remediation pointers
- crossterm pinned at 0.28.1 — tui-textarea fork blocks 0.29 upgrade
- App struct has 47 fields — god-object, no test isolation
- Clipboard subprocess fallback chain has 0 automated tests despite 3 regressions
- `src/session.rs` is a no-op stub

## Constraints

- **Tech stack**: Rust 2021, Ratatui 0.30, Crossterm 0.28.1 (pinned), portable-pty, vt100, tokio multi-thread — locked unless an Active phase explicitly addresses migration
- **Platform**: Linux + macOS only (XRDP and Kitty are first-class targets due to known compatibility work)
- **Distribution**: Single binary via GitHub Releases (eqms/claude-workbench); GitLab (origin) and GitHub (upstream) must stay in sync
- **Compatibility**: Existing config.yaml format must be preserved or migrated transparently — users rely on persistent settings
- **Performance**: 16ms event-loop polling target; PTY reader threads must never block UI; clipboard work stays off the UI thread

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| Single monolithic `App` struct (47 fields) | Fast iteration during 0.x; one place to find state | ⚠️ Revisit — REFAC-01 |
| Pin crossterm to 0.28.1 | tui-textarea fork incompatible with 0.29 event types | ⚠️ Revisit — DEP-01 |
| `lock_or_recover()` swallows poison silently | Avoid panics during PTY reader thread crashes | ⚠️ Revisit — QUAL-02 |
| `src/session.rs` left as stub | Session restore not yet validated as a real user need | — Pending — FEAT-01 |
| Dual-remote git push (GitLab + GitHub) | GitLab origin for dev, GitHub for OSS distribution + CI release builds | ✓ Good |
| Clipboard worker thread architecture | UI stays responsive when X-server hangs (XRDP) | ✓ Good |
| Self_update with signature verification | Required for safe binary distribution | ✓ Good (signatures enabled in v0.89.0) |

## Evolution

This document evolves at phase transitions and milestone boundaries.

**After each phase transition** (via `/gsd-transition`):
1. Requirements invalidated? → Move to Out of Scope with reason
2. Requirements validated? → Move to Validated with phase reference
3. New requirements emerged? → Add to Active
4. Decisions to log? → Add to Key Decisions
5. "What This Is" still accurate? → Update if drifted

**After each milestone** (via `/gsd-complete-milestone`):
1. Full review of all sections
2. Core Value check — still the right priority?
3. Audit Out of Scope — reasons still valid?
4. Update Context with current state

---
*Last updated: 2026-05-11 after initialization*
