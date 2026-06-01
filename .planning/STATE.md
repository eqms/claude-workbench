# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-05-11)

**Core value:** Stay in one terminal: file navigation, Claude Code, LazyGit, and a shell side-by-side, panes always pointing at the same working directory.
**Current focus:** Phase 2 — Test Coverage + Reliability

## Current Position

Phase: 2 of 4 (Test Coverage + Reliability) — entering planning
Plan: 0 of TBD
Status: Phase 1 ACCEPTED as delivered (Wave 1). SEC-01 (HIGH) explicitly deferred to v0.91 — plans 01-05/01-06 remain open in backlog.
Last activity: 2026-06-01 — Phase 1 accepted at 4/6; routing to Phase 2 planning

Progress: [██████░░░░] 67% Phase 1 (accepted at 4/6; SEC-01 carried to v0.91)

## Performance Metrics

**Velocity:**
- Total plans completed: 4 (Wave 1 of Phase 1)
- Average duration: ~21 min/plan (parallel execution, worktree-isolated)
- Total execution time: ~1.4 hours wall-clock (4 parallel ≈ 21 min)

**By Phase:**

| Phase | Plans | Total | Avg/Plan |
|-------|-------|-------|----------|
| 1 (Wave 1) | 4 | ~1.4h wall | 21 min |

**Recent Trend:**
- Last 4 plans: 01-01 (35m), 01-02 (12m), 01-03 (18m), 01-04 (18m)
- Trend: Stable — all S-complexity Wave 1 plans completed on first pass

*Updated after each plan completion*

## Accumulated Context

### Decisions

Decisions are logged in PROJECT.md Key Decisions table.
Recent decisions affecting current work:

- Pin crossterm at 0.28.1 (tui-textarea fork incompatible with 0.29) — DEP-01 tracks resolution
- lock_or_recover() swallows mutex poison silently — QUAL-02 tracks fix
- src/session.rs left as stub — FEAT-01 delivers first real behavior

### Pending Todos

None yet.

### Blockers/Concerns

- **SEC-01 (HIGH) — CARRIED TO v0.91 BACKLOG:** self-update signature path remains unverified. Phase 1 was accepted at 4/6 by operator decision (2026-06-01); Phase 1 success criteria 1+2 (signature reject/accept) are NOT met, deferred not closed. Plan 01-05 (CI signing) blocks on operator generating zipsign ed25519 keypair + adding GitHub Actions secret. Plan 01-06 (client verification) blocks on 2+ signed releases shipped. **Must be reopened in v0.91.**
- DEP-01: crossterm 0.29 blocked on tui-textarea fork — out of scope for Phase 1 (Phase 3)
- **Clippy baseline red (toolchain drift):** `cargo clippy -- -D warnings` reports 16 pre-existing errors on Rust 1.95.0 (`collapsible_if`/`collapsible_match`, `useless_vec`) in file_ops.rs, typst_pdf.rs, dialogs.rs, mouse.rs (resize block), update/check.rs. Not introduced by any current work. Recommend a dedicated `clippy --fix` + toolchain-pin quick task.

### Quick Tasks Completed

| # | Description | Date | Commit | Directory |
|---|-------------|------|--------|-----------|
| 260601-iwi | Fix file-browser mouse-wheel scroll (viewport, not selection) | 2026-06-01 | affd62c | [260601-iwi-...](./quick/260601-iwi-fix-file-browser-mouse-wheel-scroll-whee/) |
| v0.90.2 | Fix file-browser scroll snapback — true root cause: auto-refresh reset list offset every 2s (refresh() now preserves offset) | 2026-06-01 | ab9da43 | — |

## Session Continuity

Last session: 2026-06-01
Last activity: 2026-06-01 — Shipped v0.90.2: file-browser scroll snapback root-cause fix (auto-refresh was resetting list offset every 2s; refresh() now preserves offset). 139 tests pass.
Stopped at: v0.90.2 tagged + pushed. The v0.90.1 wheel-to-viewport change was correct but exposed a pre-existing auto-refresh reset — that is the actual root cause, now fixed. Phase 2 discussion still paused (no CONTEXT.md). Phase 1 accepted at 4/6; SEC-01 (HIGH) carried to v0.91 backlog.
Resume file: None
Next actions:
1. Plan Phase 2 (`/gsd-plan-phase`) — clipboard fallback tests + mutex-poison observability (QUAL-01, QUAL-02)
2. v0.91 backlog (carried): operator `zipsign generate-keys` + GitHub secret → execute 01-05 → ship 2+ signed releases → execute 01-06 (closes SEC-01 HIGH)
