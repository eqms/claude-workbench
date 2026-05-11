# Phase 01: Security Hardening - Context

**Gathered:** 2026-05-11
**Status:** Ready for planning
**Source:** Synthesized from 01-REVIEW.md (gsd-code-reviewer, standard depth, 9 files) + .planning/codebase/CONCERNS.md + SECURITY-NOTES.md

<domain>
## Phase Boundary

**In scope:**
- Close the 1 HIGH + 3 MEDIUM findings from the pre-planning security audit (`01-REVIEW.md`)
- Address the 2 new Critical findings (CR-02, CR-03) and 3 new Warnings (WR-03, WR-04, WR-05) surfaced during the review
- Establish a signing → verification pipeline that can be operated by CI without manual key handling
- Add minimal regression tests so the security fixes do not silently regress

**Out of scope (deferred to later phases):**
- Full clipboard subprocess test suite (Phase 2: QUAL-01)
- Mutex-poison observability (Phase 2: QUAL-02)
- App-struct decomposition (Phase 3: REFAC-01)
- crossterm 0.29 upgrade path (Phase 3: DEP-01)
- Session persistence (Phase 4: FEAT-01)
- Anything in the 3 Info-severity findings unless trivially co-located with a Critical/Warning fix

</domain>

<decisions>
## Implementation Decisions

### Self-Update Signature Verification (CR-01 / SEC-01)
- Two-phase rollout, **signing must ship before verification**, never reversed
- Phase A: CI signs release archives with `zipsign` + ed25519. Private key in GitHub Actions Secret; public key committed to `signing/claude-workbench-pub.bin`
- Phase B: `src/update/install.rs` enables `.verifying_keys([include_bytes!("../../signing/claude-workbench-pub.bin")])` on both `perform_update_sync` and `perform_update_to_version_sync`
- Public key bytes are baked into the binary via `include_bytes!` — no runtime fetch, no PKI

### `--update-to` Flag Hardening (CR-02 — NEW)
- Restrict `--update-to <version>` to debug builds: wrap dispatch in `#[cfg(debug_assertions)]` OR require `--allow-downgrade` confirmation flag for release builds
- Decision: **debug-only by default**, with a `--allow-downgrade` opt-in for legitimate operator downgrade scenarios. Both paths must still validate signature (Phase B from CR-01).

### Predictable Temp File Paths (CR-03 — escalated from Medium)
- Replace all `$TMPDIR/{stem}-{date}.{ext}` constructions in `src/browser/pdf_export.rs` with `tempfile::Builder::new().prefix(stem).suffix(ext).tempfile_in(...)` — opens with `O_EXCL`
- `tempfile` crate already in `Cargo.toml`, no new dependency

### Browser/Editor Allow-List (WR-01 / SEC-02)
- In `src/browser/opener.rs::split_command`, validate first token matches `^[A-Za-z0-9_./-]+$` after split
- Reject token containing shell metacharacters with a clear error returned through the existing error path (no panic, no silent fallback)
- Optional `path-clean` resolution to absolute path; not required for v1

### Shell-Fallback Removal in Dependency Probe (WR-02 / SEC-03)
- Replace `$SHELL -i -c "<cmd>"` in `src/setup/dependency_checker.rs::check_dependency_via_shell` with direct `Command::new(name).args(args)` for binary lookups
- The `-i` interactive shell was needed for alias resolution — alias resolution is not a security requirement for dependency probing, drop it

### Clipboard `which()` Executable-Bit Check (WR-03 — NEW)
- `src/clipboard.rs::which()` augments `is_file()` with executable-bit check on Unix (`.metadata().permissions().mode() & 0o111 != 0`)
- No-op on platforms without Unix file modes (already Linux/macOS-only project)

### `shlex::try_quote` Failure Path (WR-04 — NEW)
- `sync_terminals*` in `src/app/pty.rs` must `return` the shlex error instead of silently falling back to unescaped path
- Caller of `sync_terminals*` decides whether to ignore or surface the error; the function itself never sends an unescaped path

### Release Selection by Semver (WR-05 — NEW)
- `src/update/check.rs` must select highest-semver release tag, not `releases[0]` (GitHub creation order)
- Use `semver::Version::parse` (semver crate already transitively present via `self_update`) to compare; tie-breaker: most recent `published_at`

### Test Coverage
- Each fix above ships with at least one unit or integration test that fails on the unfixed code and passes after the fix
- Signature verification path is the exception: covered by a manual integration script (`scripts/test-signed-update.sh`) since CI can't sign on a forked PR — Phase 2 QUAL-01 will revisit

### Claude's Discretion
- Concrete signing tool: `zipsign` vs `minisign` vs custom ed25519 with `ed25519-dalek` — research will recommend; default leaning: `zipsign` (already integrated with `self_update`)
- Test framework for signature verification (mock keypair, fixture archive, etc.) — researcher's call
- Whether to backport an Info-finding (e.g., `localtime_r` bounds guard, restart-with-`--update-to` flag stripping) into this phase or defer

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Audit + Concerns
- `.planning/phases/01-security-hardening/01-REVIEW.md` — pre-planning code review (3 Critical, 5 Warning, 3 Info)
- `.planning/codebase/CONCERNS.md` — full audit context with file pointers
- `SECURITY-NOTES.md` — existing remediation plans (if file exists at repo root)

### Files Targeted by Fixes
- `src/update/install.rs` — CR-01 (verifying_keys wiring)
- `src/update/mod.rs` — CR-02 (--update-to gating)
- `src/update/check.rs` — WR-05 (semver selection)
- `src/browser/opener.rs` — WR-01 (allow-list in split_command)
- `src/browser/pdf_export.rs` — CR-03 (tempfile crate)
- `src/setup/dependency_checker.rs` — WR-02 (drop $SHELL -i -c)
- `src/clipboard.rs` — WR-03 (executable-bit check)
- `src/app/pty.rs` — WR-04 (shlex error propagation)

### Dependencies Already Present
- `Cargo.toml`: `self_update` has `signatures` feature flag enabled
- `Cargo.toml`: `tempfile`, `shlex`, `semver` (transitive)

### CI / Distribution
- `.github/workflows/release.yml` (or equivalent) — needs zipsign step added
- Dual-remote push: GitLab origin + GitHub upstream (per CLAUDE.md)
- GitHub Releases at `eqms/claude-workbench` — public binary distribution

</canonical_refs>

<specifics>
## Specific Ideas

- Signing key generation: documented one-time operator script (`scripts/generate-signing-key.sh`) using `zipsign generate-keys`
- Public key file: `signing/claude-workbench-pub.bin` — committed
- Private key: GitHub Actions Secret `CLAUDE_WORKBENCH_SIGNING_KEY` — set manually, never committed
- A "verify-from-disk" smoke test: download release archive locally, run `zipsign verify`, ensure exit code 0
- CR-02 fix may be ordered before CR-01 (lower risk; doesn't depend on signing infra)
- CR-03 fix is independent and can be plan-1 task-1 (smallest blast radius)

</specifics>

<deferred>
## Deferred Ideas

- Full automated end-to-end signed-update test in CI (needs cross-runner key access) — Phase 2 QUAL-01
- Sigstore / cosign migration if zipsign proves limiting — re-evaluate after Phase 1 ships
- Update server / staging release channel — out of scope for v1 security work
- Replacing `self_update` crate entirely — premature; verify the `signatures` feature path first
- Info findings IF-01 (OSC 52 always-success), IF-02 (restart re-execs with `--update-to`), IF-03 (`localtime_r` bounds) — IF-02 is partially addressed if CR-02 lands; IF-01 and IF-03 deferred

</deferred>

---

*Phase: 01-security-hardening*
*Context gathered: 2026-05-11 — synthesized from 01-REVIEW.md*
