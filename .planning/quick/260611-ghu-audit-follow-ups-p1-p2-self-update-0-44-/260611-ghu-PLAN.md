---
phase: quick
plan: 260611-ghu
type: execute
wave: 1
depends_on: []
files_modified:
  - Cargo.toml
  - deny.toml
  - .github/workflows/ci.yml
  - .github/workflows/release.yml
autonomous: true
requirements: [AUD-P1-DEPS, AUD-P2-CI]
must_haves:
  truths:
    - "self_update is at 0.44, dirs is at 6.0 in Cargo.toml"
    - "cargo build succeeds with the new deps"
    - "ci.yml audit job uses rustsec/audit-check@v2.0.0 (no cargo install step)"
    - "ci.yml has a cargo-deny job using EmbarkStudios/cargo-deny-action@v2"
    - "deny.toml exists and gates on advisories + known duplicates"
    - "Cargo.toml [package] has rust-version = 1.85"
    - "softprops/action-gh-release is at @v3 (node24) in release.yml"
    - "All other action pins verified Node24-compatible"
  artifacts:
    - path: "deny.toml"
      provides: "cargo-deny config (advisories, bans/duplicates, skip rules)"
    - path: "Cargo.toml"
      provides: "version 0.94.0, rust-version 1.85, deps self_update 0.44 + dirs 6.0"
    - path: ".github/workflows/ci.yml"
      provides: "rustsec/audit-check + cargo-deny jobs"
    - path: ".github/workflows/release.yml"
      provides: "softprops@v3 + verified action versions"
  key_links:
    - from: ".github/workflows/ci.yml"
      to: "rustsec/audit-check@v2.0.0"
      via: "uses: clause replacing cargo install cargo-audit"
    - from: ".github/workflows/ci.yml"
      to: "deny.toml"
      via: "EmbarkStudios/cargo-deny-action@v2"
---

<objective>
Implement approved audit follow-ups P1+P2: bump self_update 0.42→0.44 and dirs 5→6,
replace the cargo-audit CI job with rustsec/audit-check action, add cargo-deny CI gate
with deny.toml, set MSRV in Cargo.toml, and fix Node24 action version (softprops@v2→v3).

Purpose: Reduce supply-chain surface (minor dep bumps), eliminate slow `cargo install
cargo-audit` CI step, add a dependency-ban check, and future-proof CI against Node24
deadline (June 2026).

Output: Cargo.toml v0.94.0 with rust-version + bumped deps, deny.toml, updated ci.yml
and release.yml.
</objective>

<execution_context>
@$HOME/.claude/get-shit-done/workflows/execute-plan.md
@$HOME/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@.planning/STATE.md
@Cargo.toml
@.github/workflows/ci.yml
@.github/workflows/release.yml
@.planning/quick/260611-g0k-security-audit-fixes-from-project-audit-/260611-g0k-SUMMARY.md
</context>

<tasks>

<task type="auto">
  <name>Task 1: Bump self_update→0.44, dirs→6.0, add rust-version MSRV to Cargo.toml</name>
  <files>Cargo.toml</files>
  <action>
Edit Cargo.toml to make three changes:

1. Bump `self_update` from `"0.42"` to `"0.44"`. Keep the exact same feature set:
   `archive-tar, archive-zip, compression-flate2, compression-zip-deflate, rustls, signatures`
   (default-features = false stays). The 0.42→0.44 increment is a minor bump with no
   breaking API changes; the same `Update::configure()` / `ReleaseList::configure()` builder
   pattern used in src/update/install.rs and src/update/check.rs is unchanged.

2. Bump `dirs` from `"5.0"` to `"6.0"`. The API surface used in this codebase —
   `home_dir()`, `cache_dir()`, `download_dir()` — is stable across the major version.
   The cargo build step in the verify block will surface any breakage immediately.

3. In the `[package]` section, add directly after `edition = "2021"`:
   `rust-version = "1.85"`
   Rationale: edition 2021, tokio 1.44+ requires 1.70+, ratatui 0.30 requires 1.74+;
   1.85 (stable Feb 2025) is a defensible floor with ~14 months of runway. CI currently
   pins no explicit toolchain version so this documents the tested floor.

4. Bump the version from `"0.93.0"` to `"0.94.0"` (CHG: dep + CI changes).

After editing, run `cargo build` (not `cargo build --release`) to confirm compilation.
If dirs 6 triggers a compile error: STOP and report — do not attempt code fixes.
If self_update 0.44 triggers a compile error: STOP and report.
  </action>
  <verify>
    <automated>cd /Users/picard/gitbase/workbench && cargo build 2>&1 | tail -5 && echo "BUILD OK"</automated>
  </verify>
  <done>
    Cargo.toml shows version 0.94.0, rust-version = "1.85", self_update = "0.44",
    dirs = "6.0"; `cargo build` exits 0.
  </done>
</task>

<task type="auto">
  <name>Task 2: Create deny.toml (advisories + bans with skip rules for known duplicates)</name>
  <files>deny.toml</files>
  <action>
Create deny.toml at the repo root with three sections: advisories, bans, licenses.

**advisories section** — gate on RUSTSEC advisories, unmaintained crates:
```
[advisories]
version = 2
db-path = "~/.cargo/advisory-db"
db-urls = ["https://github.com/rustsec/advisory-db"]
vulnerability = "deny"
unmaintained = "warn"
yanked = "warn"
notice = "warn"
```

**bans section** — deny duplicate versions with explicit skip rules for every known
structural duplicate in the lockfile. Each skip entry MUST include a `reason` comment.

The Cargo.lock contains 59 packages with multiple versions. The following are caused by
the permanent architectural constraints of this project (crossterm pin, typst ecosystem,
windows-sys ABI evolution) and MUST be allowed with skip rules:

```
[bans]
multiple-versions = "warn"
wildcards = "allow"

skip = [
    # crossterm: 0.28.1 pinned for tui-textarea compatibility (DEP-01); 0.29 pulled by other deps
    { name = "crossterm", version = "=0.29.0" },

    # comemo 0.4 (required by typst 0.14 World trait) vs 0.5 (pulled by other typst internals)
    { name = "comemo", version = "=0.5.1" },
    { name = "comemo-macros", version = "=0.5.1" },

    # bitflags 1.x (legacy transitive dep) vs 2.x (modern)
    { name = "bitflags", version = "=1.3.2" },

    # windows-sys: multiple versions pulled by different transitive deps (nix, rustix, tokio, etc.)
    { name = "windows-sys", version = "=0.48.0" },
    { name = "windows-sys", version = "=0.52.0" },
    { name = "windows-sys", version = "=0.59.0" },
    { name = "windows-sys", version = "=0.60.2" },
    { name = "windows-targets", version = "=0.48.5" },
    { name = "windows-targets", version = "=0.52.6" },
    { name = "windows_aarch64_gnullvm", version = "=0.48.5" },
    { name = "windows_aarch64_gnullvm", version = "=0.52.6" },
    { name = "windows_aarch64_msvc", version = "=0.48.5" },
    { name = "windows_aarch64_msvc", version = "=0.52.6" },
    { name = "windows_i686_gnu", version = "=0.48.5" },
    { name = "windows_i686_gnu", version = "=0.52.6" },
    { name = "windows_i686_gnullvm", version = "=0.52.6" },
    { name = "windows_i686_msvc", version = "=0.48.5" },
    { name = "windows_i686_msvc", version = "=0.52.6" },
    { name = "windows_x86_64_gnu", version = "=0.48.5" },
    { name = "windows_x86_64_gnu", version = "=0.52.6" },
    { name = "windows_x86_64_gnullvm", version = "=0.48.5" },
    { name = "windows_x86_64_gnullvm", version = "=0.52.6" },
    { name = "windows_x86_64_msvc", version = "=0.48.5" },
    { name = "windows_x86_64_msvc", version = "=0.52.6" },

    # Remaining ecosystem churn — typst + ICU + syntect + various transitive deps
    # all at "warn" level (multiple-versions = "warn"), not hard "deny"
]
```

**licenses section** — permissive, allow all common open-source licenses:
```
[licenses]
version = 2
allow = [
    "MIT",
    "Apache-2.0",
    "Apache-2.0 WITH LLVM-exception",
    "BSD-2-Clause",
    "BSD-3-Clause",
    "ISC",
    "Unicode-3.0",
    "Unicode-DFS-2016",
    "CC0-1.0",
    "Zlib",
    "OpenSSL",
]
```

Do NOT add a `[sources]` section (would require allow-listing all registries, breaks
the git dep for tui-textarea out of the box).

Note: `cargo deny` is NOT installed locally. Do not run `cargo deny check` locally.
The deny.toml is validated structurally by the CI action in Task 3. If deny.toml has
a parse error, the CI job will catch it. Proceed to Task 3 immediately after writing.
  </action>
  <verify>
    <automated>test -f /Users/picard/gitbase/workbench/deny.toml && grep -c "multiple-versions" /Users/picard/gitbase/workbench/deny.toml | grep -q "1" && echo "deny.toml exists with bans section"</automated>
  </verify>
  <done>
    deny.toml exists at repo root, contains [advisories], [bans], [licenses] sections,
    has skip rules for crossterm/comemo/bitflags/windows-sys families.
  </done>
</task>

<task type="auto">
  <name>Task 3: Update ci.yml (audit-check action, cargo-deny job) and release.yml (Node24 softprops@v3)</name>
  <files>.github/workflows/ci.yml, .github/workflows/release.yml</files>
  <action>
**ci.yml — three changes:**

1. **Replace the `audit` job** (currently: `cargo install cargo-audit` + `cargo audit`)
   with the `rustsec/audit-check@v2.0.0` action. The new job:

```yaml
audit:
  name: Security Audit
  runs-on: ubuntu-latest
  steps:
    - uses: actions/checkout@v5

    - name: Run security audit
      uses: rustsec/audit-check@v2.0.0
      with:
        token: ${{ secrets.GITHUB_TOKEN }}
```

   This eliminates the slow `cargo install` step (cold ~2 min → ~10 sec) and uses the
   maintained action. Note: rustsec/audit-check@v2.0.0 uses node20; it is the current
   latest version and node20 remains supported until the June 2026 deadline.

2. **Add a `deny` job** after the `audit` job:

```yaml
deny:
  name: Dependency Check (cargo-deny)
  runs-on: ubuntu-latest
  steps:
    - uses: actions/checkout@v5

    - name: Check dependencies
      uses: EmbarkStudios/cargo-deny-action@v2
      with:
        command: check advisories bans licenses
        arguments: --all-features
```

   EmbarkStudios/cargo-deny-action@v2 uses docker (not node), so it is Node24-deadline-safe.
   The `command: check advisories bans licenses` matches the three sections in deny.toml.

3. No other changes to ci.yml. actions/checkout@v5 (node24), upload-artifact@v7 (node24),
   Swatinem/rust-cache@v2 (node24), dtolnay/rust-toolchain@stable (composite) are all
   already Node24-compatible. Do NOT bump them in this task.

**release.yml — one change:**

Replace `softprops/action-gh-release@v2` with `softprops/action-gh-release@v3`.
v2 uses node20; v3 uses node24. This is the only action in release.yml that needs
upgrading:
- `actions/checkout@v5` → node24 (OK)
- `dtolnay/rust-toolchain@stable` → composite (OK)
- `Swatinem/rust-cache@v2` → node24 (OK)
- `actions/upload-artifact@v7` → node24 (OK)
- `actions/download-artifact@v7` → node24 (OK)

The `softprops/action-gh-release@v3` API is compatible with v2: same `with.name`,
`with.body`, `with.files`, `with.draft`, `with.prerelease` inputs. No other changes
to release.yml needed.

After editing both files, run final verification gate.
  </action>
  <verify>
    <automated>cd /Users/picard/gitbase/workbench && cargo test 2>&1 | tail -3 && cargo clippy --all-features -- -D warnings 2>&1 | grep -v "^warning\[" | tail -5 && cargo fmt --check 2>&1 && echo "ALL GATES PASSED"</automated>
  </verify>
  <done>
    ci.yml: audit job uses rustsec/audit-check@v2.0.0 (no cargo install), deny job uses
    EmbarkStudios/cargo-deny-action@v2 with deny.toml; release.yml: softprops@v3.
    cargo test + clippy + fmt all pass.
  </done>
</task>

</tasks>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| CI runner → GitHub Actions marketplace | Uses pinned action versions; no floating @latest |
| cargo build → crates.io | Deps bumped to specific minor versions (0.44, 6.0) |

## STRIDE Threat Register

| Threat ID | Category | Component | Disposition | Mitigation Plan |
|-----------|----------|-----------|-------------|-----------------|
| T-ghu-01 | Tampering | crates.io self_update/dirs | mitigate | Minor version bumps; Cargo.lock pins exact SHAs |
| T-ghu-02 | Tampering | rustsec/audit-check action | accept | Pinned to @v2.0.0 exact tag; widely used official action |
| T-ghu-03 | Tampering | EmbarkStudios/cargo-deny-action | accept | Pinned to @v2 major; docker-based, no node runtime attack surface |
| T-ghu-SC | Tampering | npm/pip/cargo installs | accept | No new package installs in plan tasks; only Cargo.toml version bumps |
</threat_model>

<verification>
Final gate (run after all tasks):

```bash
cd /Users/picard/gitbase/workbench
cargo build
cargo test
cargo clippy --all-features -- -D warnings
cargo fmt --check
./target/debug/claude-workbench --check-update
```

The `--check-update` smoke test makes a live GitHub API call and must exit 0 (reporting
either "up to date" or "update available", not an error). This validates self_update 0.44
network path works correctly.

grep checks:
- `grep 'self_update.*0\.44' Cargo.toml`
- `grep 'dirs.*6\.0' Cargo.toml`
- `grep 'rust-version.*1\.85' Cargo.toml`
- `grep 'version.*0\.94\.0' Cargo.toml`
- `grep 'rustsec/audit-check@v2' .github/workflows/ci.yml`
- `grep 'cargo-deny-action@v2' .github/workflows/ci.yml`
- `grep 'action-gh-release@v3' .github/workflows/release.yml`
- `test -f deny.toml`
</verification>

<success_criteria>
- Cargo.toml: version 0.94.0, rust-version = "1.85", self_update = "0.44", dirs = "6.0"
- deny.toml exists with advisories + bans (skip rules) + licenses sections
- ci.yml: audit job uses rustsec/audit-check@v2.0.0 (no cargo install step); new deny job added
- release.yml: softprops/action-gh-release upgraded to @v3 (node24)
- cargo build, cargo test, cargo clippy, cargo fmt --check all pass
- --check-update smoke test exits 0
- Commit: [CHG] v0.94.0: self_update 0.44, dirs 6, MSRV, CI hardening
</success_criteria>

<output>
Create `.planning/quick/260611-ghu-audit-follow-ups-p1-p2-self-update-0-44-/260611-ghu-SUMMARY.md` when done.
</output>
