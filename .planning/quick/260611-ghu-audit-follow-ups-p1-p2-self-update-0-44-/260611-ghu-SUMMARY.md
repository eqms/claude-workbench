---
phase: quick
plan: 260611-ghu
subsystem: deps+ci
tags: [self_update, dirs, msrv, cargo-deny, rustsec, node24, ci-hardening]
dependency_graph:
  requires: []
  provides: [deny.toml, MSRV-documented, CI-audit-action, CI-deny-job, Node24-release]
  affects: [Cargo.toml, Cargo.lock, .github/workflows/ci.yml, .github/workflows/release.yml]
tech_stack:
  added: [rustsec/audit-check@v2.0.0, EmbarkStudios/cargo-deny-action@v2, softprops/action-gh-release@v3]
  patterns: [cargo-deny config, MSRV pin, CI action replacement]
key_files:
  created: [deny.toml]
  modified: [Cargo.toml, Cargo.lock, .github/workflows/ci.yml, .github/workflows/release.yml]
decisions:
  - "self_update 0.44 requires explicit reqwest feature (HTTP backend split from TLS in 0.44 API)"
  - "deny.toml uses multiple-versions=warn not deny (structural duplicates from typst+crossterm ecosystem)"
  - "No [sources] section in deny.toml (would break tui-textarea git dep resolution)"
metrics:
  duration: "~35 min"
  completed: "2026-06-11"
  tasks: 3
  files_changed: 5
---

# Phase quick Plan 260611-ghu: Audit Follow-ups P1+P2 Summary

One-liner: self_update 0.44 + dirs 6 dep bumps, MSRV 1.85 pin, deny.toml with skip rules for structural duplicates, rustsec/audit-check action replacing slow cargo install step, cargo-deny CI gate, softprops@v3 Node24 fix.

## Tasks Completed

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | Bump self_update→0.44, dirs→6.0, add rust-version MSRV | 9932f06 | Cargo.toml, Cargo.lock |
| 2 | Create deny.toml | 9932f06 | deny.toml |
| 3 | Update ci.yml + release.yml | 555657e | .github/workflows/ci.yml, release.yml |

## Verification Results

- `cargo build` (debug): PASSED
- `cargo test`: PASSED (3/3)
- `cargo clippy --all-features -- -D warnings`: PASSED (exit 0)
- `cargo fmt --check`: PASSED (no output)
- `cargo build --release`: PASSED (see smoke test)
- `./target/debug/claude-workbench --check-update`: PASSED (see smoke test section)

## Grep Verification

All plan success criteria confirmed:
- `self_update = { version = "0.44"` — Cargo.toml line 25
- `dirs = "6.0"` — Cargo.toml line 13
- `rust-version = "1.85"` — Cargo.toml line 5
- `version = "0.94.0"` — Cargo.toml line 3
- `rustsec/audit-check@v2.0.0` — ci.yml audit job
- `EmbarkStudios/cargo-deny-action@v2` — ci.yml deny job
- `softprops/action-gh-release@v3` — release.yml Create GitHub Release step

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] self_update 0.44 requires explicit `reqwest` feature**
- **Found during:** Task 1 (cargo build)
- **Issue:** self_update 0.44 split the HTTP backend from the TLS layer. In 0.42, `rustls` activated the full HTTP+TLS stack. In 0.44, the `http_client` module only conditionally exports `get()` when either `reqwest` or `ureq` feature is enabled. Without `reqwest`, compilation fails with E0425 "cannot find function `get` in module `http_client`" (14 errors).
- **Fix:** Added `reqwest` to the self_update feature list (one word, one line). This is the exact HTTP backend that was implicit in 0.42.
- **Assessment:** Small mechanical adjustment per plan constraint — not a code change, purely a Cargo.toml feature declaration. No src/ files modified.
- **Files modified:** Cargo.toml (self_update features array)
- **Commit:** 9932f06

## Known Stubs

None — no UI-facing data stubs introduced.

## Threat Flags

None — no new network endpoints, auth paths, or schema changes introduced. The `reqwest` feature addition uses the same TLS backend (aws-lc-sys via rustls) already present in 0.42.

## Smoke Test

`--check-update` smoke test validates self_update 0.44 network path:

```
Current version: 0.94.0
Checking GitHub releases...
[Update] Current version: 0.94.0
[Update] Checking GitHub: eqms/claude-workbench
[Update] Binary name: claude-workbench
[Update] Platform: macos-aarch64
[Update] GitHub version: 0.92.0
[Update] Current version is newer than latest release
Already up-to-date (v0.94.0)
EXIT: 0
```

Result: PASSED — GitHub API contacted, version comparison correct, exit 0.

## Self-Check: PASSED

All created/modified files verified:
- `deny.toml` — exists at repo root with [advisories], [bans], [licenses] sections
- `Cargo.toml` — version 0.94.0, rust-version 1.85, self_update 0.44 + reqwest, dirs 6.0
- `.github/workflows/ci.yml` — audit job uses rustsec/audit-check@v2.0.0; deny job uses cargo-deny-action@v2
- `.github/workflows/release.yml` — softprops/action-gh-release@v3

Commits verified:
- 9932f06 — [CHG] v0.94.0: self_update 0.44, dirs 6, MSRV 1.85, deny.toml
- 555657e — [CHG] CI: rustsec/audit-check action, cargo-deny job, softprops@v3
