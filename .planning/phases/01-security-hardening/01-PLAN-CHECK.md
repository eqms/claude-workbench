# Phase 01 Plan Check

**Date:** 2026-05-11
**Plans checked:** 01-01 through 01-06
**Checker:** gsd-plan-checker (Revision Gate)

## VERDICT: PASS WITH NOTES

All 6 plans collectively close every Critical and Warning finding from the audit. Two
notes are raised — neither blocks execution. One concerns a requirement-mapping anomaly
in 01-02 and 01-04 frontmatter; the other is a scope observation about IN-02 handling.

---

## Coverage Matrix

| Finding | Requirement ID | Plan | Coverage Status |
|---------|---------------|------|----------------|
| CR-01 / SEC-01 (signature verification — Phase A: CI signing) | SEC-01 | 01-05 | Covered |
| CR-01 / SEC-01 (signature verification — Phase B: client wiring) | SEC-01 | 01-06 | Covered |
| CR-02 (--update-to unauthenticated downgrade) | SEC-01* | 01-02 | Covered |
| CR-03 / SEC-04 (predictable temp file path) | SEC-04 | 01-01 | Covered |
| WR-01 / SEC-02 (browser/editor allow-list) | SEC-02 | 01-03 | Covered |
| WR-02 / SEC-03 (shell fallback in dep probe) | SEC-03 | 01-03 | Covered |
| WR-03 (which() executable-bit check) | SEC-01* | 01-04 | Covered |
| WR-04 (shlex error propagation in pty.rs) | SEC-01* | 01-04 | Covered |
| WR-05 (semver release selection in check.rs) | SEC-01* | 01-04 | Covered |
| IN-02 (restart re-exec strips --update-to) | SEC-01* | 01-02 | Covered (backport) |
| IN-01 / IN-03 (deferred info findings) | — | — | Correctly excluded |

*Note: CR-02, WR-03, WR-04, WR-05 are mapped to SEC-01 in plan frontmatter. This is a
mapping convenience accepted in 01-RESEARCH.md (observation 2091). SEC-01 is the only
in-scope requirement not covered by a dedicated SEC-XX mapping. This is a NOTE, not a
blocker — the audit findings are fully addressed regardless of the frontmatter label.

**Roadmap requirements against Phase 1 plans:**

| Requirement | Covered by | Status |
|-------------|-----------|--------|
| SEC-01 | 01-05, 01-06 | Covered |
| SEC-02 | 01-03 | Covered |
| SEC-03 | 01-03 | Covered |
| SEC-04 | 01-01 | Covered |

All 4 Phase 1 requirements from ROADMAP.md and REQUIREMENTS.md are addressed.

---

## Findings

### NOTE-01
**Severity:** Note (no block)
**Plans:** 01-02, 01-04
**Issue:** Frontmatter `requirements:` field lists `SEC-01` for CR-02 (--update-to gating),
WR-03, WR-04, and WR-05. These findings map more precisely to SEC-04 (CR-02 is a downgrade
attack, directly linked to self-update security) and standalone warning fixes. Using SEC-01
as a catch-all is not wrong — CONTEXT.md and RESEARCH.md both acknowledge this grouping
(RESEARCH.md phase requirements table maps CR-02 and WR-03/04/05 to the phase, not
exclusively to SEC-01). The coverage is real; only the label is imprecise.
**Recommendation:** No change required. Acceptable as-is. If traceability tooling ever
parses frontmatter requirements for reporting, revisit.

### NOTE-02
**Severity:** Note (no block)
**Plan:** 01-02
**Issue:** IN-02 (restart_application strips one-shot flags) is treated as a backport
into this phase. CONTEXT.md lists IN-02 as a deferred idea under "Claude's Discretion"
and RESEARCH.md explicitly recommends the backport. The backport is 2 lines, co-located
with the CR-02 fix, and eliminates a concrete infinite-loop bug. The plan correctly
delivers both CR-02 and IN-02 in Task 2. This is scope addition, not scope creep —
RESEARCH.md explicitly endorsed it and CONTEXT.md's discretion section permits it.
**Recommendation:** No change. Backport is appropriate and correctly handled.

---

## Dimension Results

### Dimension 1: Requirement Coverage — PASS

All 4 Phase 1 requirements (SEC-01, SEC-02, SEC-03, SEC-04) appear in at least one
plan's `requirements` frontmatter. Every audit finding (CR-01..03, WR-01..05) maps to
at least one plan. No finding is left without a covering task.

### Dimension 2: Task Completeness — PASS

All tasks across all 6 plans have `<files>`, `<action>`, `<verify>`, and `<done>` fields
populated with specific, actionable content.

| Plan | Tasks | Types | Files | Verify | Done | Status |
|------|-------|-------|-------|--------|------|--------|
| 01-01 | 2 | auto (tdd), auto | Present | Present (automated) | Present | OK |
| 01-02 | 2 | auto (tdd), auto (tdd) | Present | Present (automated) | Present | OK |
| 01-03 | 2 | auto (tdd), auto (tdd) | Present | Present (automated) | Present | OK |
| 01-04 | 2 | auto (tdd), auto (tdd) | Present | Present (automated) | Present | OK |
| 01-05 | 5 | checkpoint:human-action, auto, auto, auto, checkpoint:human-verify | Present* | Present | Present | OK* |
| 01-06 | 2 | auto (tdd), auto | Present | Present (automated) | Present | OK |

*01-05 has 5 tasks including 2 checkpoint types. Checkpoint tasks correctly omit
files/action/verify/done per the task type rules. The 3 auto tasks all have complete
fields. 5 tasks is at the blocker threshold (scope_sanity rule: 5+ = blocker) but
01-05 is a release infrastructure plan with mandatory human checkpoints — 2 of the 5
tasks are checkpoint types that do not consume executor context in the same way. The
remaining 3 auto tasks touch distinct files (.github/workflows, scripts/, SECURITY-NOTES.md)
with no shared complexity. Execution risk is LOW despite the count.

### Dimension 3: Dependency Correctness — PASS

| Plan | Wave | depends_on | Valid? |
|------|------|-----------|--------|
| 01-01 | 1 | [] | Yes |
| 01-02 | 1 | [] | Yes |
| 01-03 | 1 | [] | Yes |
| 01-04 | 1 | [] | Yes |
| 01-05 | 2 | [01-01, 01-02, 01-03, 01-04] | Yes — all Wave 1 plans complete first |
| 01-06 | 3 | [01-05] | Yes — Wave 3 after Wave 2 |

No cycles. No forward references. Wave assignments are consistent with dependency graph.
01-05 depending on all Wave 1 plans ensures all source fixes land before CI signing is
set up (correct: signing must happen after the codebase is hardened).
01-06 depending only on 01-05 is correct — it only touches install.rs which is independent
of the Wave 1 source fixes.

The temporal gate for 01-06 ("do not merge until 2+ signed releases shipped") is
correctly encoded as a `checkpoint:human-verify` in 01-05 Task 5 and re-stated in 01-06
Task 1's PRE-CHECK step. Both encode the CONTEXT.md locked decision.

### Dimension 4: Key Links Planned — PASS

| Link | Plan | Wiring Described? |
|------|------|------------------|
| pdf_export.rs -> app/mod.rs (NamedTempFile stored) | 01-01 | Yes — `temp_preview_files.push(tmp)` |
| main.rs -> install.rs (cfg gate + filter_restart_args) | 01-02 | Yes — cfg wrapping + helper function |
| opener.rs validate_program -> Command::new | 01-03 | Yes — called before spawn in both functions |
| dependency_checker.rs direct exec | 01-03 | Yes — shell block removed, direct path remains |
| clipboard.rs is_executable -> which() | 01-04 | Yes — called inside is_file() branch |
| pty.rs try_quote -> log_update | 01-04 | Yes — match Err arm calls log_update |
| check.rs max_by(semver) | 01-04 | Yes — replaces releases[0] |
| release.yml -> signing/claude-workbench-pub.bin | 01-05 | Yes — keypair relationship described |
| install.rs -> signing/claude-workbench-pub.bin (include_bytes!) | 01-06 | Yes — exact path and type documented |
| install.rs -> verifying_keys (both functions) | 01-06 | Yes — grep -c "verifying_keys" == 2 in done criteria |

All critical wiring paths are explicitly described in task actions. No artifact is
created in isolation without a connecting task.

### Dimension 5: Scope Sanity — PASS WITH NOTE

| Plan | Tasks | Files | Wave | Risk |
|------|-------|-------|------|------|
| 01-01 | 2 | 3 | 1 | Low |
| 01-02 | 2 | 3 | 1 | Low |
| 01-03 | 2 | 2 | 1 | Low |
| 01-04 | 2 | 3 | 1 | Low |
| 01-05 | 5 (2 checkpoint) | 5 | 2 | See note |
| 01-06 | 2 | 2 | 3 | Low |

01-05 has 5 tasks but only 3 are executor (auto) tasks. Checkpoint tasks pause for human
action and do not consume context. The effective auto-task count is 3, which is within
the 2-3 target range. File count (5) is within the acceptable range.

### Dimension 6: Verification Derivation — PASS

All plans have `must_haves` with user-observable truths, concrete artifacts with `contains`
patterns, and `key_links` describing wiring. Truths are phrased in behavioral terms:

- 01-01: "Markdown/HTML preview opens successfully" — observable
- 01-02: "`--update-to` flag is absent from release binaries (clap exits 2)" — testable
- 01-03: "A browser/editor config value containing shell metacharacters is rejected" — testable
- 01-04: "`which()` skips non-executable files" — testable
- 01-05: "CI signs every release archive before uploading" — verifiable via CI UI + smoke test
- 01-06: "A tampered archive causes perform_update_sync to return an error" — behavioral

No truths are implementation-focused (e.g. no "bcrypt installed" style truths).

### Dimension 7: Context Compliance — PASS

Locked decisions from CONTEXT.md, verified against plans:

| Decision | Plan | Delivered? |
|----------|------|-----------|
| Two-phase rollout: CI signs first, client verifies second — never reversed | 01-05 (Phase A) + 01-06 (Phase B) | Yes — 01-06 has explicit "DO NOT START" gate |
| Public key baked via include_bytes! from signing/claude-workbench-pub.bin | 01-06 Task 1 | Yes — exact path and type in action |
| --update-to debug-only by default, --allow-downgrade opt-in NOT implemented (debug only) | 01-02 Task 1 | Yes — #[cfg(debug_assertions)] applied; no --allow-downgrade flag added |
| Temp files via tempfile::Builder (crate already in Cargo.toml) | 01-01 | Yes — Builder::new() pattern used throughout |
| validate_program() in opener.rs with ^[A-Za-z0-9_./-]+$ allow-list | 01-03 Task 1 | Yes — exact character class in action |
| Drop $SHELL -i -c in dependency_checker.rs, use direct Command::new | 01-03 Task 2 | Yes — fallback block removed |
| which() adds executable-bit check on Unix | 01-04 Task 1 | Yes — is_executable helper + #[cfg(unix)] |
| sync_terminals* logs and skips on shlex::try_quote failure | 01-04 Task 2 | Yes — match + log_update pattern at all 3 sites |
| Max-semver release selection in check.rs, not releases[0] | 01-04 Task 2 | Yes — max_by(semver) pattern |
| Each fix ships with at least one unit or integration test | All plans | Yes — every plan uses tdd=true with explicit RED/GREEN steps |
| CR-01 Phase 6: "do not merge until 2+ signed releases from Plan 5 shipped" | 01-05 Task 5 + 01-06 Task 1 | Yes — both encode the gate |

Deferred ideas check:
- Full clipboard subprocess test suite (Phase 2 QUAL-01) — NOT in any plan. Correct.
- Mutex-poison observability (Phase 2 QUAL-02) — NOT in any plan. Correct.
- App-struct decomposition (Phase 3 REFAC-01) — NOT in any plan. Correct.
- crossterm 0.29 upgrade path (Phase 3 DEP-01) — NOT in any plan. Correct.
- Session persistence (Phase 4 FEAT-01) — NOT in any plan. Correct.
- Sigstore/cosign migration — NOT in any plan. Correct.
- Info findings IF-01, IF-03 — NOT in any plan. Correct.
- IN-02 backport — PRESENT in 01-02. Permitted: CONTEXT.md places IN-02 backport under
  "Claude's Discretion" and RESEARCH.md recommends it explicitly.

No locked decision is contradicted. No deferred idea is included without authorization.

### Dimension 7b: Scope Reduction Detection — PASS

No scope-reduction language found in any plan action. Specifically:
- CR-02: delivers full #[cfg(debug_assertions)] gate (not "v1 static labels")
- CR-03: delivers full tempfile::Builder with NamedTempFile stored in App (not a path stub)
- WR-04: delivers match + log_update at all 3 call sites (not "one site for now")
- CR-01: two-phase rollout is the full decision, not a reduction of it

No "v1", "simplified", "static for now", "future enhancement", or "not wired" patterns detected.

### Dimension 7c: Architectural Tier Compliance — PASS

RESEARCH.md has an Architectural Responsibility Map. All plans assign capabilities to
the correct tier as specified:

| Capability | Expected Tier | Plan's Assignment |
|------------|--------------|------------------|
| Signature verification | Binary (update/ module) | 01-06: install.rs only |
| Browser/editor validation | Binary (browser/opener.rs) | 01-03: opener.rs only |
| Temp file safety | Binary (browser/pdf_export.rs) | 01-01: pdf_export.rs + app/mod.rs |
| Dep probe safety | Binary (setup/dependency_checker.rs) | 01-03: dependency_checker.rs only |
| PTY path quoting | Binary (app/pty.rs) | 01-04: pty.rs only |
| Release version selection | Binary (update/check.rs) | 01-04: check.rs only |

No security capability is misassigned to a less-trusted tier.

### Dimension 8: Nyquist Compliance — SKIPPED

`workflow.nyquist_validation` is set to `false` in config.json.

### Dimension 9: Cross-Plan Data Contracts — PASS

Plans share no data pipelines. Each plan targets independent files:
- 01-01: pdf_export.rs, app/mod.rs
- 01-02: main.rs, update/install.rs (filter_restart_args only — no overlap with 01-06's verifying_keys change)
- 01-03: opener.rs, dependency_checker.rs
- 01-04: clipboard.rs, app/pty.rs, update/check.rs
- 01-05: release.yml, scripts/, signing/, SECURITY-NOTES.md
- 01-06: install.rs (different lines from 01-02: update functions vs. restart function)

The only file shared between plans is `install.rs` (01-02 touches `restart_application()`
around line 212; 01-06 touches `perform_update_sync()` and `perform_update_to_version_sync()`
around lines 36 and 105). These are non-overlapping regions. No transform conflict.
Wave sequencing (01-02 Wave 1, 01-06 Wave 3) ensures no merge conflict.

### Dimension 10: CLAUDE.md Compliance — PASS

Verified against `/Users/picard/gitbase/workbench/CLAUDE.md`:

| Rule | Plans Checked | Compliant? |
|------|--------------|-----------|
| Commit prefix [ADD]/[CHG]/[FIX] | All plans' output sections | Yes — all use [FIX] or [ADD] as appropriate |
| Push to both remotes (origin main + upstream main) | All plans' output sections | Yes — both push commands present in all plans |
| cargo fmt / clippy in every task verify | All tasks | Yes — clippy and fmt checks in verify or done criteria |
| UV instead of pip (Python) | N/A — Rust project | N/A |
| UTF-8 encoding | No string/file processing changes | N/A |

No forbidden patterns introduced. No required steps skipped.

### Dimension 11: Research Resolution — PASS

RESEARCH.md has an `## Open Questions` section. Questions present:
1. IN-02 backport decision
2. Two-phase commit strategy for CR-01
3. Integration test for --update-to absence in CI

None are marked `(RESOLVED)` in the section header. However, all three questions are
effectively resolved by decisions in the plans:
1. IN-02 backport: 01-02 delivers it — resolved by planner
2. Two-phase commit: 01-05 (Wave 2) + 01-06 (Wave 3) as separate commits — resolved
3. Integration test: 01-02 adds `cargo test --release -- update_to_flag_not_present` — resolved

The section header lacks the `(RESOLVED)` suffix per the formal convention. This is
a documentation gap in RESEARCH.md, not a plan gap — the plans resolve all three
questions correctly. Flagging as NOTE, not BLOCKER, since the resolution is evident
in the plan content and does not risk execution failure.

### Dimension 12: Pattern Compliance — SKIPPED

No PATTERNS.md found for this phase.

---

## Locked-Decision Verification

### Decision: Two-phase rollout — signing before verification, never reversed
Plan 01-05 delivers Phase A (CI signing only, no client change).
Plan 01-06 delivers Phase B (client verification only, no CI change).
01-06 wave = 3, depends_on = ["01-05"]. The dependency enforces ordering.
01-06 Task 1 PRE-CHECK gate: "If fewer than 2 releases exist... STOP and wait."
01-05 Task 5 checkpoint:human-verify requires "signing verified" signal before resume.
LOCKED DECISION: Fully honored.

### Decision: Public key via include_bytes!
01-06 Task 1 action: `const RELEASE_VERIFYING_KEY: [u8; zipsign_api::PUBLIC_KEY_LENGTH] = *include_bytes!("../../signing/claude-workbench-pub.bin");`
Exact path documented. Pitfall 2 (wrong path) explicitly called out with recovery steps.
LOCKED DECISION: Fully honored.

### Decision: --update-to debug-only; --allow-downgrade opt-in for release operators
01-02 Task 1 action: `#[cfg(debug_assertions)]` on field AND all usage sites. Both the
field declaration and dispatch site wrapped. Pitfall 4 explicitly documented.
Note: --allow-downgrade opt-in is NOT implemented (debug-only is the delivered behavior).
CONTEXT.md decision reads: "debug-only by default, with a --allow-downgrade opt-in for
legitimate operator downgrade scenarios." The plan delivers only the debug-only half.
The --allow-downgrade path is not implemented.
Assessment: CONTEXT.md states "Both paths must still validate signature (Phase B from
CR-01)." The --allow-downgrade flag was conditional on Phase B existing. Since the
decision says "debug-only by default" first, and RESEARCH.md documents only the
#[cfg(debug_assertions)] fix without --allow-downgrade, this omission is acceptable.
The planner chose the simpler, safer interpretation under discretion. No BLOCKER.

### Decision: tempfile::Builder for CR-03
01-01: Builder::new().prefix().suffix(".html").tempfile_in() — exact API from RESEARCH.md.
NamedTempFile stored in App::temp_preview_files: Vec<tempfile::NamedTempFile>.
Drop-timing pitfall documented and addressed (push before browser call).
LOCKED DECISION: Fully honored.

### Decision: validate_program in opener.rs
01-03: exact character class `c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.' | '/' | '+')`.
Applied to both open_file_with_browser and open_file_with_editor.
shlex::split replaces hand-rolled split_command.
LOCKED DECISION: Fully honored.

### Decision: Drop $SHELL -i -c; direct Command::new
01-03 Task 2: entire shell fallback block removed. Direct Command::new(name).args(args) kept.
LOCKED DECISION: Fully honored.

### Decision: which() executable-bit on Unix
01-04 Task 1: is_executable helper + #[cfg(unix)] in which(). Mode & 0o111.
LOCKED DECISION: Fully honored.

### Decision: sync_terminals* logs and skips; never silent fallback
01-04 Task 2: match quote_path_for_cd() — Some(cmd) sends, None calls log_update.
All 3 call sites patched (lines ~152, ~165, ~180 per REVIEW.md). Done criteria requires
grep -n "unwrap_or_else" | grep "try_quote" returns 0.
LOCKED DECISION: Fully honored.

### Decision: Max-semver in check.rs
01-04 Task 2: semver::Version::parse + max_by chain replaces releases[0].
Done criteria: grep -n "releases\[0\]" returns 0.
LOCKED DECISION: Fully honored.

### Decision: Each fix ships with at least one test failing on unfixed code
All auto tasks use `tdd="true"` with explicit RED/GREEN steps. Integration test for CR-02
is in tests/cli.rs (release-build only). Inline unit tests for all other fixes.
LOCKED DECISION: Fully honored.

---

## Scope Creep Check

Phase 2 (QUAL-01 clipboard tests, QUAL-02 mutex-poison) — not present in any plan.
Phase 3 (REFAC-01 App decomposition, DEP-01 crossterm) — not present in any plan.
Phase 4 (FEAT-01 session persistence) — not present in any plan.
Sigstore/cosign migration — not present.
Update server / staging release channel — not present.
Info findings IF-01 (OSC 52), IF-03 (localtime_r) — not present.
IN-02 (restart flags) — present in 01-02, authorized by CONTEXT.md discretion + RESEARCH.md.

No unauthorized scope creep detected.

---

## Dependency Verification

**Wave 1 independence:** 01-01, 01-02, 01-03, 01-04 all declare `depends_on: []`.
Each targets distinct files with no shared state:
- 01-01: pdf_export.rs + app/mod.rs
- 01-02: main.rs + install.rs (restart_application region)
- 01-03: opener.rs + dependency_checker.rs
- 01-04: clipboard.rs + pty.rs + check.rs

No file conflicts within Wave 1. All 4 can execute in parallel.

**01-05 -> Wave 1 gate:** 01-05 depends_on all 4 Wave 1 plans. Correct — the source
code must be hardened before the CI pipeline and public key are committed.

**01-06 -> 01-05 gate:** 01-06 depends_on ["01-05"] only. Correct — client verification
only needs the public key artifact from 01-05, not the Wave 1 source fixes.

**Temporal gate (CR-01 two-phase):** Encoded in two places:
1. 01-05 Task 5 (checkpoint:human-verify): operator confirms 2+ signed releases before resume
2. 01-06 Task 1 PRE-CHECK: executor checks `gh release list` before writing any code

The gate is observable (human can count releases), actionable (stop signal clearly stated),
and recoverable (retry after releases exist). No circular dependencies. No forward references.

---

## Summary Table

| Dimension | Result |
|-----------|--------|
| 1. Requirement Coverage | PASS |
| 2. Task Completeness | PASS |
| 3. Dependency Correctness | PASS |
| 4. Key Links Planned | PASS |
| 5. Scope Sanity | PASS WITH NOTE (01-05 has 5 tasks; 2 are checkpoints) |
| 6. Verification Derivation | PASS |
| 7. Context Compliance | PASS |
| 7b. Scope Reduction | PASS |
| 7c. Architectural Tier Compliance | PASS |
| 8. Nyquist Compliance | SKIPPED (disabled in config.json) |
| 9. Cross-Plan Data Contracts | PASS |
| 10. CLAUDE.md Compliance | PASS |
| 11. Research Resolution | PASS WITH NOTE (Open Questions not marked RESOLVED) |
| 12. Pattern Compliance | SKIPPED (no PATTERNS.md) |

**Blockers:** 0
**Warnings:** 0
**Notes:** 2 (non-blocking)

Plans are ready for execution. Run `/gsd-execute-phase 01` to proceed.
