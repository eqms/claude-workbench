# Roadmap: claude-workbench

## Overview

v0.89.0 audit surfaced 4 security findings, zero clipboard test coverage, a 47-field god-struct, a pinned dependency blocking upstream, and a no-op session stub. This roadmap closes those gaps in four phases: harden security first (users download binaries), then lock in test coverage, then reduce structural debt while resolving the dependency question, then deliver the one missing feature users actually notice.

## Phases

- [ ] **Phase 1: Security Hardening** - Close 1 HIGH + 3 MEDIUM findings from the v0.89.0 security audit
- [ ] **Phase 2: Test Coverage + Reliability** - Automated clipboard tests + observable mutex-poison error surfacing
- [ ] **Phase 3: Refactor + Dependency Strategy** - App god-struct decomposition + crossterm 0.29 upgrade path decision
- [ ] **Phase 4: Session Persistence** - Replace no-op session stub with working cwd save/restore

## Phase Details

### Phase 1: Security Hardening
**Goal**: All open security audit findings are closed — binary update path is safe, and the three MEDIUM vectors are mitigated
**Depends on**: Nothing (first phase)
**Requirements**: SEC-01, SEC-02, SEC-03, SEC-04
**Success Criteria** (what must be TRUE):
  1. Running `./claude-workbench --check-update` with a tampered or missing signature rejects the update and exits non-zero
  2. A binary with a valid signature passes verification and self-updates successfully (regression: this must still work)
  3. Each of the 3 MEDIUM findings has a code-level fix landed and a note in SECURITY-NOTES.md marking it resolved
  4. `cargo audit` reports no HIGH or MEDIUM advisories in the dependency tree
**Plans**: TBD

### Phase 2: Test Coverage + Reliability
**Goal**: Clipboard fallback chain is covered by automated tests and mutex-poison events are observable in production
**Depends on**: Phase 1
**Requirements**: QUAL-01, QUAL-02
**Success Criteria** (what must be TRUE):
  1. `cargo test` includes tests for the xclip, pbcopy, and cc-clip fallback paths — each path exercises both success and failure branches
  2. XRDP failure mode (clipboard subprocess timeout/hang) has a test that verifies the async worker returns an error without blocking the UI thread
  3. CI (GitHub Actions) runs the clipboard tests on both Linux and macOS matrices
  4. A mutex-poison event in any PTY reader thread produces a visible log line (not silently swallowed) — verifiable by injecting a poisoned lock in a test
**Plans**: TBD

### Phase 3: Refactor + Dependency Strategy
**Goal**: App struct is decomposed into testable sub-states and the crossterm 0.29 blocker has a documented, committed resolution
**Depends on**: Phase 2
**Requirements**: REFAC-01, DEP-01
**Success Criteria** (what must be TRUE):
  1. `App` struct has fewer than 25 fields (down from 47); at least one logical sub-domain (e.g., clipboard state, selection state, or update state) is extracted into its own struct with isolated unit tests
  2. The extracted sub-module compiles and its tests pass with `cargo test` independently of the full app state
  3. A ROADMAP-update entry exists in `.planning/` (or a PR/commit) with either: a concrete crossterm 0.29 migration plan with the tui-textarea blocker resolved, or documented rationale for a permanent pin with a tracked alternative
**Plans**: TBD

### Phase 4: Session Persistence
**Goal**: Users' working directory is remembered across launches — the session stub delivers its first real behavior
**Depends on**: Phase 3
**Requirements**: FEAT-01
**Success Criteria** (what must be TRUE):
  1. Launching `claude-workbench` in a fresh shell opens the file browser at the directory from the last session (not always `$HOME` or `$CWD`)
  2. The session file is written on clean exit and on SIGTERM; a crash does not corrupt it
  3. An integration test verifies: launch → navigate to directory → exit → relaunch → assert file browser starts at saved directory
  4. Existing `config.yaml` format is unaffected — session state uses a separate file
**Plans**: TBD

## Progress

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 1. Security Hardening | 0/TBD | Not started | - |
| 2. Test Coverage + Reliability | 0/TBD | Not started | - |
| 3. Refactor + Dependency Strategy | 0/TBD | Not started | - |
| 4. Session Persistence | 0/TBD | Not started | - |
