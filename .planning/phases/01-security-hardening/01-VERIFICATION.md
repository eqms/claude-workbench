# Phase 01 Verification — Wave 1 Partial Ship

**Date:** 2026-05-11
**Status:** pass (wave_1_partial)
**Scope:** Wave 1 only (plans 01-01..04). Wave 2 (01-05) + Wave 3 (01-06) explicitly deferred — gated on operator action and on 2+ signed releases shipping respectively.

## Wave 1 Verdict: PASS

All 4 Wave 1 plans completed; merged to `main`; tests + build green.

| Plan | Reqs Closed | Verification |
|------|-------------|--------------|
| 01-01 tempfile-symlink-fix | SEC-04 (CR-03) | 4 new unit tests + 130 total green |
| 01-02 `--update-to` gate + restart filter | CR-02, IN-02 | `cargo build --release` rejects `--update-to`; `tests/cli.rs::update_to_flag_not_present_in_release_build` green |
| 01-03 allow-list + drop `$SHELL -i -c` | SEC-02, SEC-03 | `test_validate_program_*` + `test_check_command_*` all green; 50-line shell-fallback block removed |
| 01-04 clipboard exec-bit + shlex + semver | WR-03, WR-04, WR-05 | New tests for is_executable, sync_terminals Err propagation, semver max selection — all green |

## Automated Checks

- **`cargo test`:** 130 passed, 3 ignored, 0 failed (was 111 pre-Wave-1; +19 new test bodies)
- **`cargo check`:** clean
- **`cargo clippy`:** 15 `collapsible_match` warnings remain — **all pre-existing** in `file_ops.rs`, `keyboard/dialogs.rs`, `mouse.rs`, `typst_pdf.rs`; not introduced by Wave 1
- **Merge conflict:** 1 (in `dependency_checker.rs`, 01-01 fmt vs 01-03 block removal); resolved by taking 01-03's version since the block was being deleted entirely

## Requirements Status (post-Wave-1)

| REQ-ID | Status |
|--------|--------|
| SEC-01 | **Gated** — Wave 2 (CI signing) requires operator zipsign keypair generation + GitHub Actions secret. Wave 3 (client `verifying_keys()`) requires 2+ signed releases to have shipped. |
| SEC-02 | ✓ Done — commit `4b6046d` |
| SEC-03 | ✓ Done — commit `6fe0862` |
| SEC-04 | ✓ Done — commit `4b84723` |

## Audit Findings Status (from 01-REVIEW.md)

| Finding | Plan | Status |
|---------|------|--------|
| CR-01 (HIGH) | 01-05, 01-06 | Deferred — Wave 2 + Wave 3 |
| CR-02 (Critical) | 01-02 | ✓ Done |
| CR-03 (Critical) | 01-01 | ✓ Done |
| WR-01 | 01-03 | ✓ Done |
| WR-02 | 01-03 | ✓ Done |
| WR-03 | 01-04 | ✓ Done |
| WR-04 | 01-04 | ✓ Done |
| WR-05 | 01-04 | ✓ Done |
| IN-01 (OSC 52 reports success) | — | Deferred (project memory only) |
| IN-02 (restart re-exec flags) | 01-02 | ✓ Done (backported) |
| IN-03 (`localtime_r` bounds) | — | Mitigated separately (`MAX_SAFE_TIMESTAMP` guard exists) |

## Ship Decision

**Approved for dual-remote push to main.** Wave 2 + Wave 3 work continues in subsequent commits — no release tag yet (the operator decides when to cut v0.90.x).

## Reviewer Notes

- This is an in-place ship: no PR, no feature branch, dual-remote push per `CLAUDE.md` convention.
- VERIFICATION.md status `wave_1_partial` is non-standard for the GSD ship workflow; treat as `pass` for the closed reqs and `gated` for SEC-01.
- The phase is NOT closed in ROADMAP.md — 2 of 6 plans remain.
