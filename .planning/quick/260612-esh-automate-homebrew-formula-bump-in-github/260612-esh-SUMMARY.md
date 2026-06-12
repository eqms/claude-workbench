---
phase: quick-260612-esh
plan: 01
subsystem: ci-release
tags: [github-actions, homebrew, tap, deploy-key, release-automation]
dependency_graph:
  requires: []
  provides: [automated-homebrew-tap-updates]
  affects: [.github/workflows/release.yml, Cargo.toml, Cargo.lock]
tech_stack:
  added: [TAP_DEPLOY_KEY secret, deploy key on eqms/homebrew-claude-workbench]
  patterns: [post-release job gated on needs.release.result, workflow_dispatch manual trigger]
key_files:
  created: []
  modified:
    - .github/workflows/release.yml
    - Cargo.toml
    - Cargo.lock
decisions:
  - "workflow_dispatch skips build/release jobs via if: github.event_name == 'push' guards"
  - "update-homebrew-tap uses always() + conditional to handle both push and workflow_dispatch triggers"
  - "SHA256 validated with length check (64 chars) and count sanity (exactly 4 lines) before formula write"
  - "deploy key provisioned via stdin redirect (private key never echoed), temp dir wiped immediately"
metrics:
  duration: 12m
  completed: 2026-06-12
  tasks_completed: 3
  files_changed: 3
---

# Phase quick-260612-esh Plan 01: Automate Homebrew Formula Bump Summary

**One-liner:** Adds automated Homebrew tap update job to release pipeline using SSH deploy key, eliminating the manual post-release formula bump step.

## What Was Built

Extended `.github/workflows/release.yml` with an `update-homebrew-tap` job that:
- Triggers automatically after `release` job succeeds on `v*` tag pushes
- Can be manually triggered via `workflow_dispatch` with a `tag` input (e.g. `v0.96.0`) for dry-run testing
- Downloads all 4 Homebrew-relevant release assets (macOS aarch64/x86_64, Linux aarch64/x86_64)
- Computes SHA256 for each asset and validates (64 hex chars, exactly 4 values)
- Checks out `eqms/homebrew-claude-workbench` via SSH deploy key (`TAP_DEPLOY_KEY` secret)
- Rewrites `Formula/claude-workbench.rb` using `sed` (URL version bump) and `awk` (SHA256 replacement)
- Commits with `[CHG] Update to vX.Y.Z` message and pushes -- no-ops if formula is already current

Also bumped `Cargo.toml` to `0.96.1` (PATCH; CI-only change) and synced `Cargo.lock`.

## Deploy Key Provisioning

- Ed25519 keypair generated in `mktemp -d` (chmod 700), private key never echoed
- Public key added as write deploy key on `eqms/homebrew-claude-workbench` with title "claude-workbench release pipeline"
- Private key stored as `TAP_DEPLOY_KEY` secret on `eqms/claude-workbench` via stdin redirect
- Temp dir wiped with `rm -rf` immediately after secret upload

## Commits

| Hash | Message | Files |
|------|---------|-------|
| 3880a66 | [ADD] v0.96.1: automate Homebrew formula bump in release pipeline | .github/workflows/release.yml, Cargo.toml, Cargo.lock |

## Deviations from Plan

None - plan executed exactly as written, with orchestrator overrides applied:
- Task 3 (push) skipped per override (orchestrator merges to main and pushes)
- `actions/checkout@v4` used in update-homebrew-tap job (Node 24 compatible, per override)

## Threat Mitigations Applied

| Threat | Mitigation |
|--------|-----------|
| T-esh-01 Tampering formula | SHA256 length check (64 chars) + count check (must be 4) before commit |
| T-esh-02 Key disclosure | Private key passed via stdin only, temp dir wiped, never echoed to logs |
| T-esh-04 Missing asset | Explicit `exit 1` if any of the 4 asset files is absent after download |

## Known Stubs

None. The formula update job is fully wired. The workflow_dispatch dry-run against v0.96.0 can be triggered by the orchestrator after merge+push to verify end-to-end.

## Self-Check: PASSED

- [x] `.github/workflows/release.yml` contains `update-homebrew-tap` job
- [x] `.github/workflows/release.yml` contains `workflow_dispatch` trigger
- [x] `Cargo.toml` shows `version = "0.96.1"`
- [x] Commit `3880a66` exists on worktree-agent branch
- [x] Deploy key "claude-workbench release pipeline" visible on eqms/homebrew-claude-workbench
- [x] `TAP_DEPLOY_KEY` secret visible on eqms/claude-workbench (updated 2026-06-12)
- [x] No key files remain in /tmp
- [x] `cargo check` passes (Finished dev profile)
