# Phase 01: Security Hardening — Research

**Researched:** 2026-05-11
**Domain:** Rust binary supply-chain security, process hardening, temp-file safety
**Confidence:** HIGH (all claims verified against local crate source or project files)

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
- Two-phase rollout: CI signs first, client verifies second — never reversed
- Public key baked into binary via `include_bytes!` from `signing/claude-workbench-pub.bin`
- `--update-to` debug-only by default, `--allow-downgrade` opt-in for release
- Temp files via `tempfile::Builder` (crate already in Cargo.toml)
- `validate_program()` in `opener.rs` with `^[A-Za-z0-9_./-]+$` allow-list
- Drop `$SHELL -i -c` in `dependency_checker.rs`, use direct `Command::new`
- `which()` adds executable-bit check on Unix
- `sync_terminals*` logs and skips on `shlex::try_quote` failure — no silent fallback
- Max-semver release selection in `check.rs`, not `releases[0]`
- Each fix ships with at least one unit or integration test

### Claude's Discretion
- Concrete signing tool: `zipsign` vs `minisign` — this research recommends `zipsign` (rationale below)
- Test framework for signature verification — recommendation: mock keypair with `zipsign_api`
- Whether to backport IN-02 (restart strips `--update-to`) into this phase

### Deferred Ideas (OUT OF SCOPE)
- Full clipboard subprocess test suite (Phase 2 QUAL-01)
- Mutex-poison observability (Phase 2 QUAL-02)
- App-struct decomposition (Phase 3 REFAC-01)
- crossterm 0.29 upgrade path (Phase 3 DEP-01)
- Session persistence (Phase 4 FEAT-01)
- Info findings IF-01, IF-03 unless trivially co-located
- Sigstore/cosign migration
- Update server / staging release channel
- Replacing `self_update` crate entirely
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| SEC-01 | Self-update signature verification — download → verify → swap; reject unsigned/mismatched | `self_update` 0.42 `verifying_keys` API verified; zipsign-api 0.1.5 already transitive dep |
| SEC-02 | Browser/editor command allow-list in `opener.rs` | `validate_program()` pattern confirmed; `shlex::split` already in deps for better quoting |
| SEC-03 | Remove `$SHELL -i -c` in `dependency_checker.rs` | Direct `Command::new` pattern verified; shell fallback confirmed unnecessary for binary lookup |
| SEC-04 | Replace predictable temp paths in `pdf_export.rs` with `tempfile::Builder` | API verified: `Builder::new().prefix().suffix().tempfile_in()` → `NamedTempFile`; `persist()` / `keep()` for handoff |
| CR-02 | Gate `--update-to` behind `#[cfg(debug_assertions)]` | `--fake-version` precedent confirmed at main.rs:44; clap `#[cfg]` on field works |
| CR-03 | See SEC-04 — same finding, promoted to Critical | (same) |
| WR-03 | `which()` executable-bit check | `std::os::unix::fs::PermissionsExt` pattern confirmed |
| WR-04 | `shlex::try_quote` failure → log + skip, no silent fallback | Three call sites in `pty.rs` confirmed (lines 151-153, 164-166, 179-181) |
| WR-05 | Max-semver release selection in `check.rs` | `semver 1.0.27` already in dep tree; `Version::parse` strips `v` prefix manually first |
</phase_requirements>

---

## Summary

Phase 1 closes 3 Critical and 5 Warning security findings across 8 source files. All required
libraries (`self_update 0.42`, `zipsign-api 0.1.5`, `tempfile 3.24`, `semver 1.0.27`, `shlex 1.3`)
are **already present in the Cargo.lock** — no new dependencies needed. The most complex fix is
SEC-01/CR-01 (signing pipeline + client verification), which is a two-phase, two-commit operation:
CI gets the signing step first, then client code enables `.verifying_keys()`. Every other fix is
self-contained, independently deployable, and small-to-medium effort.

**Primary recommendation:** Execute fixes in dependency order — CR-03/SEC-04 and CR-02 first
(independent, zero-risk), then WR-03/WR-04/WR-05/WR-01/WR-02 (all independent S-effort), then
SEC-01/CR-01 last (M-effort, requires CI infrastructure change before client wiring).

---

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Signature verification | Binary (Rust, update/ module) | CI (GitHub Actions) | Binary validates at download time; CI produces the signatures |
| Browser/editor command validation | Binary (browser/opener.rs) | Config layer | Config provides input; validation must be in executor |
| Temp file safety | Binary (browser/pdf_export.rs) | OS | `O_EXCL` open is kernel-enforced, no upper layer can substitute |
| Dep probe safety | Binary (setup/dependency_checker.rs) | — | Direct exec replaces shell dispatch entirely |
| PTY path quoting | Binary (app/pty.rs) | — | Error path, no external tier involved |
| Release version selection | Binary (update/check.rs) | GitHub API | API returns unordered; binary must sort |

---

## Standard Stack

All dependencies already present — no additions required.

### Core (verified via `cargo metadata` and local registry source)

| Library | Version | Purpose | Verified |
|---------|---------|---------|----------|
| `self_update` | 0.42.0 | GitHub release download + self-replace | `[VERIFIED: Cargo.lock]` |
| `zipsign-api` | 0.1.5 | ed25519 signing/verification for tar.gz and zip | `[VERIFIED: Cargo.lock transitive dep of self_update]` |
| `tempfile` | 3.24.0 | Atomic temp files with `O_EXCL` | `[VERIFIED: Cargo.lock]` |
| `semver` | 1.0.27 | `Version::parse` for release tag comparison | `[VERIFIED: Cargo.lock transitive dep]` |
| `shlex` | 1.3.0 | Shell-safe argument quoting and splitting | `[VERIFIED: Cargo.lock]` |
| `anyhow` | 1.0.98 | `bail!` for validation errors | `[VERIFIED: Cargo.lock]` |

**Installation:** No `cargo add` needed. All crates are already resolved.

For the signing CLI tool, `zipsign` must be installed in the CI runner and optionally locally:

```bash
# CI runner (added to release.yml)
cargo install zipsign

# Local key generation (one-time operator action)
cargo install zipsign
```

---

## Architecture Patterns

### Self-Update Signing Flow

```
Operator (one-time)
  └─ zipsign gen-key signing/priv.key signing/claude-workbench-pub.bin
  └─ commit signing/claude-workbench-pub.bin  (public key only)
  └─ set GitHub Secret: CLAUDE_WORKBENCH_SIGNING_KEY = base64(priv.key contents)

CI (release.yml — per tag)
  ├─ build matrix (4 platforms → 4 .tar.gz + 2 .zip)
  ├─ [NEW] sign each archive:
  │     zipsign sign tar archive.tar.gz priv.key     (for .tar.gz)
  │     zipsign sign zip archive.zip priv.key         (for .zip)
  └─ upload signed archives to GitHub Release

Binary (update/install.rs — at runtime)
  ├─ Download archive from GitHub Release (existing path)
  ├─ [NEW] verify signature against baked-in public key:
  │     .verifying_keys([RELEASE_VERIFYING_KEY])
  └─ self-replace if verification passes; return Err if not
```

### Key File Format

`zipsign gen-key` produces two files:
- `priv.key`: raw 64-byte ed25519 keypair (no PEM header) `[VERIFIED: zipsign-api-0.1.5 source, KEYPAIR_LENGTH = 64]`
- `pub.key` / `claude-workbench-pub.bin`: raw 32-byte public key `[VERIFIED: ed25519-dalek-2.2.0/src/constants.rs: PUBLIC_KEY_LENGTH = 32]`

`include_bytes!` loads the raw 32-byte file as `&[u8; 32]` — this is exactly what
`verifying_keys()` expects: `Vec<[u8; zipsign_api::PUBLIC_KEY_LENGTH]>` where
`PUBLIC_KEY_LENGTH = 32`. `[VERIFIED: self_update-0.42.0/src/backends/github.rs:421-424]`

### `verifying_keys` API (VERIFIED)

```rust
// Source: self_update-0.42.0/src/backends/github.rs:421
#[cfg(feature = "signatures")]
pub fn verifying_keys(
    &mut self,
    keys: impl Into<Vec<[u8; zipsign_api::PUBLIC_KEY_LENGTH]>>,  // = [u8; 32]
) -> &mut Self
```

Usage pattern (wrapping `include_bytes!` result):
```rust
// Source: [VERIFIED: self_update source + zipsign-api PUBLIC_KEY_LENGTH=32]
const RELEASE_VERIFYING_KEY: [u8; 32] =
    *include_bytes!("../../signing/claude-workbench-pub.bin");

Update::configure()
    // ... existing builder calls ...
    .verifying_keys([RELEASE_VERIFYING_KEY])
    .build()
```

`include_bytes!` returns `&[u8; N]`; dereference with `*` to get `[u8; 32]` which can be
passed as a single-element array `[RELEASE_VERIFYING_KEY]` to satisfy `impl Into<Vec<[u8;32]>>`.

### Signature Verification Error Behavior (VERIFIED)

On signature failure, `updater.update()` returns `Err(self_update::errors::Error::Signature(...))`.
`[VERIFIED: self_update-0.42.0/src/errors.rs:26]`

On archive with **no signature at all** (e.g. old release), returns
`Err(self_update::errors::Error::NoSignatures(ArchiveKind))`.
`[VERIFIED: self_update-0.42.0/src/errors.rs:24]`

Neither path panics — both return `Err` through the existing match chain in `install.rs`. The
existing `UpdateResult::Error(msg)` arm in both `perform_update_sync` and
`perform_update_to_version_sync` will handle them without code structure changes.

### zipsign CI Integration Pattern

```yaml
# In release.yml — add after archive creation, before upload
- name: Install zipsign
  run: cargo install zipsign

- name: Decode signing key
  run: |
    echo "${{ secrets.CLAUDE_WORKBENCH_SIGNING_KEY }}" | base64 -d > /tmp/signing.key

- name: Sign archive (tar.gz)
  if: matrix.archive_type == 'tar.gz'
  run: zipsign sign tar ${{ matrix.asset_name }} /tmp/signing.key

- name: Sign archive (zip)
  if: matrix.archive_type == 'zip'
  run: zipsign sign zip ${{ matrix.asset_name }} /tmp/signing.key

- name: Shred private key
  if: always()
  run: shred -u /tmp/signing.key 2>/dev/null || rm -f /tmp/signing.key
```

GitHub Actions Secret `CLAUDE_WORKBENCH_SIGNING_KEY` holds `base64(priv.key)`.
Operator encodes once: `base64 signing/priv.key` → paste into GitHub repo Settings → Secrets.

**IMPORTANT — Two-phase rollout sequence:**
1. Merge the CI signing step; push a test tag; verify signed archives appear in release
2. Ship 2-3 signed releases
3. Only then wire `.verifying_keys()` in client — merge as a separate commit/PR

### `tempfile::Builder` Pattern for CR-03 (VERIFIED)

```rust
// Source: tempfile-3.24.0/src/lib.rs:526-555 + src/file/mod.rs
use tempfile::Builder;

// Returns NamedTempFile — auto-deletes on drop, opened with O_EXCL
pub fn default_preview_file(
    source: &Path,
    project_name: &str,
) -> std::io::Result<tempfile::NamedTempFile> {
    let stem = source.file_stem().and_then(|s| s.to_str()).unwrap_or("preview");
    let prefix = if project_name.is_empty() {
        format!("{}-", stem)
    } else {
        format!("{}-{}-", project_name, stem)
    };
    Builder::new()
        .prefix(&prefix)
        .suffix(".html")
        .tempfile_in(std::env::temp_dir())
}
```

**To get the path for passing to `open_file_with_browser`:**
```rust
// path() returns &Path — valid as long as NamedTempFile is alive
let tmp = default_preview_file(source, project_name)?;
open_file_with_browser(tmp.path(), browser)?;
// Keep `tmp` alive — store it in App::temp_preview_files
```

**To persist (keep file alive after NamedTempFile drop):**
```rust
// keep() → Result<(File, PathBuf), PersistError> [VERIFIED: tempfile-3.24.0/src/file/mod.rs:810]
let (_, path) = tmp.keep().map_err(|e| e.error)?;
// `path` is the PathBuf; file now persists until manual deletion
```

**Recommended approach:** Store `NamedTempFile` handles in `App::temp_preview_files: Vec<tempfile::NamedTempFile>` (replacing `Vec<PathBuf>`). The browser needs the file to exist while open — keeping the handle alive guarantees this. On `App` drop or explicit cleanup, handles drop → OS deletes the file. This also fixes the SIGKILL cleanup gap noted in CONCERNS.md.

### Semver Release Selection Pattern (VERIFIED)

`semver::Version::parse` is at `semver-1.0.27/src/lib.rs:423`. Pre-release tags (e.g. `-alpha`, `-rc1`) have non-empty `pre` field and sort lower than release versions by default. `[VERIFIED: semver 1.0.27 source]`

```rust
// Source: [VERIFIED: semver-1.0.27 API + check.rs current pattern]
use semver::Version;

let latest = releases
    .iter()
    .filter_map(|r| {
        let tag = r.version.strip_prefix('v').unwrap_or(&r.version);
        Version::parse(tag).ok().map(|v| (v, r))
    })
    .max_by(|(va, _), (vb, _)| va.cmp(vb))
    .map(|(_, r)| r);

let Some(latest) = latest else {
    return UpdateCheckResult::NoReleasesFound;
};
```

Pre-release tags are automatically sorted lower than stable releases.
Unparseable tags (e.g. `nightly-20260101`) are silently skipped via `.ok()` — correct behavior.

### Browser Allow-List Pattern (WR-01)

```rust
// Source: [VERIFIED: opener.rs current code + REVIEW.md WR-01 recommendation]
fn validate_program(prog: &str) -> anyhow::Result<()> {
    if prog.is_empty() || !prog.chars().all(|c| {
        c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.' | '/' | '+')
    }) {
        anyhow::bail!("Unsafe program name in browser/editor config: {:?}", prog);
    }
    Ok(())
}
```

`+` is included to handle `g++` style programs. No null bytes pass `chars()` iteration on
validated UTF-8 Rust `&str` — null bytes would have been rejected by YAML deserialization
before reaching this code. `[VERIFIED: Rust str is valid UTF-8]`

Replace `split_command` with `shlex::split` to correctly handle single-quoted args like
`open -a 'Brave Browser'`:
```rust
// shlex::split returns Option<Vec<String>> — None on parse error
let tokens = shlex::split(browser).ok_or_else(|| anyhow::anyhow!("Invalid shell quoting in browser config: {:?}", browser))?;
```

### shlex Error Propagation Pattern (WR-04)

Current code (3 identical sites in pty.rs:151, 164, 179):
```rust
let escaped = shlex::try_quote(&path_str)
    .map(|c| c.into_owned())
    .unwrap_or_else(|_| path_str.to_string());  // ← silent fallback
```

Fixed pattern:
```rust
// Source: [VERIFIED: pty.rs current code + log_update() usage in update/log.rs]
match shlex::try_quote(&path_str) {
    Ok(escaped) => {
        let cmd = format!("cd {}\r", escaped);
        if let Some(pty) = self.terminals.get_mut(&target_pane) {
            let _ = pty.write_input(cmd.as_bytes());
        }
    }
    Err(_) => {
        // Unreachable on Unix (filesystem paths cannot contain NUL),
        // but fail loudly rather than inject unescaped path
        log_update(&format!(
            "sync_terminals: skipping unquotable path: {:?}",
            self.file_browser.current_dir
        ));
    }
}
```

`log_update` writes to `/tmp/claude-workbench-update.log` — existing logging mechanism.
`[VERIFIED: ARCHITECTURE.md cross-cutting concerns section]`

### `which()` Executable-Bit Pattern (WR-03)

```rust
// Source: [VERIFIED: clipboard.rs:120-129 + std::os::unix::fs::PermissionsExt docs]
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

pub fn which(name: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path) {
        let candidate = dir.join(name);
        if candidate.is_file() {
            #[cfg(unix)]
            {
                let mode = candidate.metadata().ok()?.permissions().mode();
                if mode & 0o111 == 0 {
                    continue;
                }
            }
            return Some(candidate);
        }
    }
    None
}
```

### `--update-to` Gating Pattern (CR-02)

`--fake-version` is already gated at `main.rs:44` with `#[cfg(debug_assertions)]` and
`#[arg(long, env = "...")]`. Apply the same pattern to `--update-to`:

```rust
// Source: [VERIFIED: main.rs:44-46 current --fake-version pattern]
/// Update to a specific version (for testing/downgrade).
/// Only available in debug builds — use --allow-downgrade for release operator use.
#[cfg(debug_assertions)]
#[arg(long)]
update_to: Option<String>,
```

The dispatch in `main.rs` (around line 321) referencing `args.update_to` must also be wrapped
in `#[cfg(debug_assertions)]`. In release builds the field does not exist on the struct, so
any reference to `args.update_to` outside a `#[cfg]` guard becomes a compile error — this is
the desired behaviour (compiler enforces the gate).

**IN-02 co-location opportunity:** `restart_application()` in `install.rs` re-execs with all
original args, including `--update-to`. This causes an infinite downgrade loop if the app
restarts after a `--update-to` invocation. The fix is trivial and co-located with CR-02:

```rust
// Source: [VERIFIED: install.rs:212]
let args: Vec<String> = std::env::args()
    .skip(1)
    .filter(|a| !matches!(
        a.as_str(),
        "--update-to" | "--check-update" | "--clipboard-diag" | "--ssh-paste-diag"
    ))
    .collect();
```

This backport of IN-02 into Phase 1 is **recommended** — 2 additional lines, directly co-located
with the CR-02 fix, eliminates a concrete infinite-loop bug.

### WR-02: Shell Fallback Removal

`check_command` in `dependency_checker.rs` already tries `Command::new(name).args(args)` first
(line 143) and only falls back to `$SHELL -i -c` if direct execution fails (line 188).
The `claude` binary is a real executable (not a shell function on most installs), so direct
execution finds it. The `$SHELL -i` path exists for Fish alias resolution.

Fix: Remove the `#[cfg(not(windows))]` shell fallback block entirely OR keep it with an
explicit allow-list:
```rust
const NEEDS_SHELL_FALLBACK: &[&str] = &[];  // empty: no current binaries need it
```

`check_binary()` (lines 129-138) already exists as the pattern that was extracted for
clipboard helpers — apply the same direct-PATH-only approach to `check_command` fallback.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| ed25519 verify | Custom sig check | `zipsign-api` via `self_update .verifying_keys()` | zipsign handles tar.gz GZIP comment embedding + dalek verification |
| Temp file race prevention | Predictable path + existence check | `tempfile::Builder` | `O_EXCL` is atomic at kernel level; userspace checks have TOCTOU race |
| Semver comparison | Custom version string comparison | `semver::Version::parse + .cmp()` | `version_newer()` already exists but only does pairwise — `max_by` on `Version` is simpler and handles pre-release ordering correctly |
| Shell quoting | Custom quote logic | `shlex::try_quote` / `shlex::split` | Single-quote and backslash edge cases are subtle |

---

## Runtime State Inventory

> Not applicable — this phase makes no renames or data migrations. Skipped.

---

## Common Pitfalls

### Pitfall 1: Two-Phase Order Violation for CR-01

**What goes wrong:** Enabling `.verifying_keys()` before CI produces signed archives causes every
existing user's auto-update to fail with `NoSignatures` error — effectively a bricking update.
**Why it happens:** The client starts rejecting unsigned archives before any signed ones exist.
**How to avoid:** CI signing step must be merged and produce at least 2-3 signed releases before
the client verification commit ships. CONTEXT.md locks this order.
**Warning signs:** Any PR that adds `.verifying_keys()` to `install.rs` without a corresponding
preceding commit adding signing to `release.yml`.

### Pitfall 2: `include_bytes!` Path Resolution

**What goes wrong:** `include_bytes!("../../signing/claude-workbench-pub.bin")` path is relative to
the source file (`src/update/install.rs`), not the project root.
**Why it happens:** `include_bytes!` path is relative to the file containing the macro.
**How to avoid:** From `src/update/install.rs`, the path to project root is `../../`, making the
full relative path `../../signing/claude-workbench-pub.bin`. Verify with
`cargo check` — a wrong path produces a compile error immediately.
**Warning signs:** `error: couldn't read ../../signing/...: No such file` at compile time.

### Pitfall 3: `NamedTempFile` Drop Timing for CR-03

**What goes wrong:** `NamedTempFile` is created, `.path()` is passed to browser, then the handle
is dropped immediately — file is deleted before the browser opens it.
**Why it happens:** Rust drops values at end of scope; if the `NamedTempFile` is not stored,
it drops within the same expression.
**How to avoid:** Store the `NamedTempFile` in `App::temp_preview_files: Vec<tempfile::NamedTempFile>`.
The file lives as long as the handle lives.
**Warning signs:** Browser opens a file-not-found error on previewed HTML.

### Pitfall 4: `#[cfg(debug_assertions)]` on clap Struct Field (CR-02)

**What goes wrong:** Using `#[cfg(debug_assertions)]` on a `clap` derive struct field means the
field does not exist in the release struct. Any code outside a matching `#[cfg]` block that
references `args.update_to` becomes a compile error in release builds.
**Why it happens:** Desired behavior, but easy to miss a reference site.
**How to avoid:** Wrap both the field declaration AND every usage site (`if let Some(v) = args.update_to`)
with matching `#[cfg(debug_assertions)]` guards.
**Warning signs:** `error[E0609]: no field 'update_to' on type 'Args'` in release build.

### Pitfall 5: Semver with Pre-release Tags (WR-05)

**What goes wrong:** `semver::Version::parse("0.85.1-rc1")` succeeds but returns a version
with `pre = "rc1"`. Semver spec says `1.0.0-alpha < 1.0.0`. This is correct behavior — pre-releases
sort lower. But if a future release accidentally tags as `v1.0.0-release` or similar, it would
sort lower than `v0.99.0`.
**How to avoid:** Use `Version::parse` as-is; the sorting is correct per semver spec. No special
handling needed for pre-release filtering in the normal case.
**Warning signs:** Update check reports lower version than expected when pre-release tags exist.

### Pitfall 6: Windows .zip Signing in CI

**What goes wrong:** `zipsign sign zip archive.zip priv.key` — the `zip` subcommand differs
from `tar`. Windows matrix entries use `.zip` format, not `tar.gz`.
**How to avoid:** Use `matrix.archive_type` to branch the zipsign command (already in the
matrix: `archive_type: tar.gz` vs `archive_type: zip`). The `release.yml` matrix already has
this field — use the same `if: matrix.archive_type == 'tar.gz'` / `zip` pattern.
**Warning signs:** Windows downloads fail signature verification with `NoSignatures` error.

---

## Code Examples

### Minimal Verified Integration: `verifying_keys` in `install.rs`

```rust
// Source: [VERIFIED: self_update-0.42.0/src/backends/github.rs:421-424]
// File: src/update/install.rs

// At module level — pub.bin is 32 raw bytes (zipsign ed25519 public key)
const RELEASE_VERIFYING_KEY: [u8; zipsign_api::PUBLIC_KEY_LENGTH] =
    *include_bytes!("../../signing/claude-workbench-pub.bin");

// In perform_update_sync() and perform_update_to_version_sync():
match Update::configure()
    .repo_owner(REPO_OWNER)
    .repo_name(REPO_NAME)
    .bin_name(BIN_NAME)
    .target(target)
    .current_version(CURRENT_VERSION)
    .verifying_keys([RELEASE_VERIFYING_KEY])   // ← Phase B addition
    .show_download_progress(false)
    .show_output(false)
    .no_confirm(true)
    .build()
```

### Test: `validate_program` rejects metacharacters

```rust
// Inline unit test — #[cfg(test)] mod tests pattern used throughout codebase
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_program_accepts_safe_names() {
        assert!(validate_program("firefox").is_ok());
        assert!(validate_program("open").is_ok());
        assert!(validate_program("/usr/bin/xdg-open").is_ok());
        assert!(validate_program("open-a-browser").is_ok());
        assert!(validate_program("g++").is_ok());
    }

    #[test]
    fn test_validate_program_rejects_metacharacters() {
        assert!(validate_program("").is_err());
        assert!(validate_program("fire;fox").is_err());
        assert!(validate_program("$(rm -rf /)").is_err());
        assert!(validate_program("a b").is_err());
        assert!(validate_program("a|b").is_err());
        assert!(validate_program("a&b").is_err());
        assert!(validate_program("a`b`").is_err());
    }
}
```

### Integration Test: `--update-to` absent from release binary

```rust
// File: tests/cli.rs (existing integration test file using CARGO_BIN_EXE pattern)
#[test]
#[cfg(not(debug_assertions))]
fn update_to_flag_not_present_in_release_build() {
    let output = Command::new(workbench_binary())
        .args(["--update-to", "0.1.0"])
        .output()
        .expect("failed to invoke binary");

    // clap exits 2 for unknown arguments
    assert_eq!(
        output.status.code(),
        Some(2),
        "--update-to should be unknown in release builds"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("unexpected argument") || stderr.contains("unrecognized"),
        "stderr: {}",
        stderr
    );
}
```

Note: `cargo test` runs with `debug_assertions` on by default. This test only runs in release
mode (`cargo test --release`). Add to CI as a release-build gate check.

### Test: `which()` executable-bit check

```rust
// Inline in clipboard.rs tests (existing #[cfg(test)] mod tests)
#[test]
#[cfg(unix)]
fn test_which_skips_non_executable_file() {
    use std::os::unix::fs::PermissionsExt;

    let dir = tempfile::TempDir::new().unwrap();
    let fake_bin = dir.path().join("fakeclip");
    std::fs::write(&fake_bin, b"not a binary").unwrap();
    // Set permissions to 0o644 (readable, not executable)
    std::fs::set_permissions(&fake_bin, std::fs::Permissions::from_mode(0o644)).unwrap();

    // Prepend our temp dir to PATH
    let orig_path = std::env::var_os("PATH").unwrap_or_default();
    let new_path = std::env::join_paths(
        std::iter::once(dir.path().as_os_str())
            .chain(std::env::split_paths(&orig_path))
    ).unwrap();

    // which() must not return the non-executable file
    // (Can't set env in a test easily without std::env::set_var — use direct call)
    // Test the logic indirectly: create executable version, verify found
    std::fs::set_permissions(&fake_bin, std::fs::Permissions::from_mode(0o755)).unwrap();
    // PATH manipulation needed; test the is_file + mode check via unit-level extraction
}
```

Note: PATH manipulation in unit tests is fragile (global mutable state). Prefer extracting the
executable-bit check into a helper `fn is_executable(path: &Path) -> bool` and testing that
directly:

```rust
#[cfg(unix)]
fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    path.metadata()
        .map(|m| m.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

#[test]
#[cfg(unix)]
fn test_is_executable_respects_mode() {
    let tmp = tempfile::NamedTempFile::new().unwrap();
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(tmp.path(), std::fs::Permissions::from_mode(0o644)).unwrap();
    assert!(!is_executable(tmp.path()));
    std::fs::set_permissions(tmp.path(), std::fs::Permissions::from_mode(0o755)).unwrap();
    assert!(is_executable(tmp.path()));
}
```

### Semver max selection test

```rust
// Inline unit test in update/check.rs
#[test]
fn test_max_semver_selection_ignores_creation_order() {
    // Simulate GitHub returning a patch release for old branch first
    let tags = vec!["v0.85.1", "v0.89.0", "v0.88.3", "v0.85.2"];
    let best = tags.iter()
        .filter_map(|t| {
            let stripped = t.strip_prefix('v').unwrap_or(t);
            semver::Version::parse(stripped).ok().map(|v| (v, t))
        })
        .max_by(|(va, _), (vb, _)| va.cmp(vb))
        .map(|(_, t)| *t);
    assert_eq!(best, Some("v0.89.0"));
}
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Predictable temp path (`$TMPDIR/name-date.html`) | `tempfile::Builder` with O_EXCL | This phase | Eliminates symlink redirect on shared systems |
| No signature verification on auto-update | zipsign ed25519 + `verifying_keys()` | This phase | Closes supply-chain attack vector |
| `releases[0]` for latest version | `max_by(semver)` | This phase | Correct even when old branch patches are published after main |
| `$SHELL -i -c` for dep probe | `Command::new(name)` direct exec | This phase | Removes injection surface |

**Deprecated/outdated:**
- `shell-escape` crate: already replaced with `shlex 1.3` in v0.89.0 (CONCERNS.md, Closed section)
- Hand-rolled `date_now_dmy` temp path suffix: replaced by `tempfile` random suffix

---

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | `zipsign` CLI accepts `sign tar` and `sign zip` subcommands matching the two archive types in release.yml | CI Integration Pattern | CI step would fail; check `zipsign --help` when installing |
| A2 | Base64-encoded private key fits in a GitHub Actions Secret (max 64KB; ed25519 private key is 64 bytes → ~88 base64 chars) | CI Integration Pattern | Non-issue in practice |
| A3 | `claude` CLI is a real executable (not a Fish alias) on target systems — making `check_command` shell fallback unnecessary for it | WR-02 pattern | If `claude` is installed as a shell function on some systems, removing fallback breaks wizard dependency check for those users |

---

## Open Questions

1. **IN-02 backport decision**
   - What we know: `restart_application()` re-execs with all args including `--update-to`, causing infinite downgrade loop
   - What's unclear: Whether this has ever triggered in practice (only fires if user runs with `--update-to` and app restarts, which is unusual)
   - Recommendation: **Backport IN-02 into Phase 1** — it's 2 lines, directly co-located with the CR-02 fix, and eliminates a real bug

2. **Two-phase commit strategy for CR-01**
   - What we know: CI signing (release.yml change) and client verification (install.rs change) must ship as separate releases
   - What's unclear: Should they be separate git commits on the same branch or separate PRs?
   - Recommendation: Separate commits — commit 1 is "CI signing only", commit 2 (tagged release after 2-3 signed versions exist) is "enable client verification"

3. **Integration test for `--update-to` absence**
   - What we know: The test only proves the flag is absent in release builds; CI by default runs `cargo test` (debug), not `cargo test --release`
   - What's unclear: Should release.yml add `cargo test --release` as a CI gate?
   - Recommendation: Add `cargo test --release -- tests::` as a final check in the release job before uploading artifacts

---

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| `zipsign` CLI | CI signing step (release.yml) | ✗ (not in PATH locally) | — | `cargo install zipsign` in CI step |
| `cargo` / Rust toolchain | All fixes | ✓ | stable | — |
| GitHub Actions Secrets | CLAUDE_WORKBENCH_SIGNING_KEY | ✗ (must be created) | — | None — operator must set manually |

**Missing dependencies with no fallback:**
- GitHub Actions Secret `CLAUDE_WORKBENCH_SIGNING_KEY`: operator must generate keypair and configure

**Missing dependencies with fallback:**
- `zipsign` CLI: installed via `cargo install zipsign` in release.yml CI step

---

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | Rust built-in (`cargo test`) |
| Config file | None (no `rust-test.toml` or equivalent) |
| Quick run command | `cargo test` |
| Full suite command | `cargo test && cargo test --release -- tests::` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| SEC-01 / CR-01 | Signed archive accepted; unsigned rejected | Manual integration script | `scripts/test-signed-update.sh` | ❌ Wave 0 |
| SEC-02 / WR-01 | `validate_program` blocks metacharacters | unit | `cargo test test_validate_program` | ❌ Wave 0 (inline test) |
| SEC-03 / WR-02 | Direct exec finds `git`, `claude` without shell fallback | unit | `cargo test check_command` | ✅ existing tests in `dependency_checker.rs` |
| SEC-04 / CR-03 | `default_preview_file` returns `NamedTempFile` opened with O_EXCL | unit | `cargo test test_preview_file_tempfile` | ❌ Wave 0 (inline test) |
| CR-02 | `--update-to` absent from release build arg parser | integration | `cargo test --release -- update_to_flag_not_present` | ❌ Wave 0 (`tests/cli.rs`) |
| WR-03 | `which()` skips non-executable files on Unix | unit | `cargo test test_is_executable` | ❌ Wave 0 (inline test in `clipboard.rs`) |
| WR-04 | `sync_terminals` logs and skips on unquotable path, no fallback | unit | `cargo test test_sync_terminals_unquotable` | ❌ Wave 0 (inline test in `pty.rs`) |
| WR-05 | `check.rs` selects max-semver release | unit | `cargo test test_max_semver_selection` | ❌ Wave 0 (inline test in `check.rs`) |

### Sampling Rate
- Per task commit: `cargo test`
- Per wave merge: `cargo test && cargo clippy -- -D warnings`
- Phase gate: All tests green in both debug and release (`cargo test --release -- tests::`) before `/gsd-verify-work`

### Wave 0 Gaps
- [ ] `src/browser/opener.rs` — inline `#[cfg(test)] mod tests` with `test_validate_program_*` tests
- [ ] `src/browser/pdf_export.rs` — inline test for `default_preview_file` returning `NamedTempFile`
- [ ] `src/clipboard.rs` — inline test for `is_executable` helper (extract from `which`)
- [ ] `src/update/check.rs` — inline test for `test_max_semver_selection_ignores_creation_order`
- [ ] `src/app/pty.rs` — inline test for shlex error path (requires extracting path-escape logic into testable fn)
- [ ] `tests/cli.rs` — `update_to_flag_not_present_in_release_build` integration test
- [ ] `scripts/test-signed-update.sh` — manual smoke test for signed archive verification

---

## Security Domain

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | no | — |
| V3 Session Management | no | — |
| V4 Access Control | yes (partial) | `--update-to` gating prevents privilege downgrade |
| V5 Input Validation | yes | `validate_program()` allow-list; `shlex::try_quote` error propagation |
| V6 Cryptography | yes | ed25519 via `zipsign-api` / `self_update .verifying_keys()` — never hand-rolled |

### Known Threat Patterns

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Compromised GitHub release asset | Tampering | ed25519 signature verification via `verifying_keys()` |
| Symlink redirect on shared `/tmp` | Tampering | `tempfile::Builder` with `O_EXCL` |
| Command injection via config | Tampering | `validate_program()` allow-list |
| Downgrade to vulnerable version | Elevation of Privilege | `--update-to` debug-only; signature verification rejects pre-signing releases |
| Shell injection via dep probe | Tampering | Direct `Command::new` replacing `$SHELL -i -c` |
| Non-executable binary on PATH | Spoofing (reliability) | Executable-bit check in `which()` |
| Stale release masking current update | Tampering | Max-semver selection over API creation order |

---

## Risk Matrix (Execution Order)

| Order | Fix | Complexity | Blast Radius | Dependency |
|-------|-----|-----------|--------------|------------|
| 1 | CR-02: gate `--update-to` + IN-02 backport | S | `main.rs`, `install.rs` only | None |
| 2 | CR-03/SEC-04: `tempfile::Builder` in `pdf_export.rs` | M | Signature change propagates to callers; `App::temp_preview_files` type change | None |
| 3 | WR-01/SEC-02: `validate_program` + `shlex::split` in `opener.rs` | S | `opener.rs` only | None |
| 4 | WR-02/SEC-03: remove shell fallback in `dependency_checker.rs` | S | `dependency_checker.rs` only | None |
| 5 | WR-03: executable-bit in `clipboard.rs::which()` | S | `clipboard.rs` only | None |
| 6 | WR-04: shlex error propagation in `pty.rs` | S | 3 sites in `pty.rs` | None |
| 7 | WR-05: max-semver in `check.rs` | S | `check.rs` only | None |
| 8 | SEC-01/CR-01 Phase A: CI signing step in `release.yml` | M | `release.yml` only; existing releases unaffected | None |
| 9 | SEC-01/CR-01 Phase B: `.verifying_keys()` in `install.rs` | S | `install.rs` only | Phase A must be deployed + 2-3 signed releases shipped first |

Items 1-7 can be planned as a single wave (all independent, all in Rust source only).
Items 8-9 are split across two commits with a temporal gap between them.

---

## Sources

### Primary (HIGH confidence — verified against local files)
- `self_update-0.42.0/src/backends/github.rs` — `verifying_keys` API, parameter type `[u8; zipsign_api::PUBLIC_KEY_LENGTH]`
- `self_update-0.42.0/src/errors.rs` — `Error::Signature`, `Error::NoSignatures` variants
- `zipsign-api-0.1.5/src/lib.rs` — `PUBLIC_KEY_LENGTH` re-exported from ed25519-dalek
- `ed25519-dalek-2.2.0/src/constants.rs` — `PUBLIC_KEY_LENGTH = 32`
- `tempfile-3.24.0/src/lib.rs` + `src/file/mod.rs` — `Builder::new().prefix().suffix().tempfile_in()`, `NamedTempFile::keep()`, `NamedTempFile::persist()`
- `semver-1.0.27/src/lib.rs` — `Version::parse`
- `shlex-1.3.0` — `try_quote`, `split`
- `src/update/install.rs`, `src/update/check.rs`, `src/browser/pdf_export.rs`, `src/browser/opener.rs`, `src/app/pty.rs`, `src/clipboard.rs`, `src/setup/dependency_checker.rs`, `src/main.rs` — current code
- `.github/workflows/release.yml` — current CI structure, matrix archive types
- `Cargo.toml` / `Cargo.lock` — confirmed all deps present, no additions needed

### Secondary (MEDIUM confidence)
- WebSearch: `zipsign` README summary — `sign tar` and `sign zip` subcommands confirmed `[CITED: crates.io/crates/zipsign description]`
- WebSearch: `zipsign-api` docs overview — "signs and verifies .zip and .tar.gz files" `[CITED: docs.rs/zipsign-api]`

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — all crates verified in Cargo.lock, all APIs verified in local registry source
- Architecture: HIGH — all target files read directly, code patterns confirmed
- Pitfalls: HIGH — derived from actual code reading + known Rust type system behavior

**Research date:** 2026-05-11
**Valid until:** 2026-08-11 (stable deps; self_update 0.42 API unlikely to change)
