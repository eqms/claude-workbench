# Phase 1 Security Review

**Phase:** 01 — Security Hardening
**Depth:** standard
**Date:** 2026-05-11
**Scope:** 9 files reviewed

---

## Summary

| Severity | Count |
|----------|-------|
| Critical | 3 |
| Warning  | 5 |
| Info     | 3 |

Nine files were reviewed against the security findings catalogued in
`SECURITY-NOTES.md` and `CONCERNS.md`. All four previously identified
findings (1 HIGH, 3 MEDIUM) are confirmed against current code with
refined line numbers. Five additional findings were surfaced that were
not in the prior audit. The most significant new finding is that
`--update-to <version>` is available in release builds with no
authentication, allowing any local user to downgrade the binary to an
older, potentially vulnerable version.

---

## Critical Findings

### CR-01: Self-Update Downloads Binary Without Signature Verification

**File:** `src/update/install.rs:36-73` (also `:105-146`)
**Maps to:** SEC-01 (HIGH — confirmed open)

Both `perform_update_sync()` and `perform_update_to_version_sync()` call
`Update::configure()` without `.verifying_keys(...)`. The `signatures`
feature flag is compiled in (`Cargo.toml` confirms it), but the API call
is never made. The `signing/` directory does not exist in the repository,
meaning the public key is also not committed.

Current code:
```rust
match Update::configure()
    .repo_owner(REPO_OWNER)
    .repo_name(REPO_NAME)
    .bin_name(BIN_NAME)
    .target(target)
    .current_version(CURRENT_VERSION)
    // .verifying_keys([...])  ← MISSING
    .show_download_progress(false)
    .show_output(false)
    .no_confirm(true)
    .build()
```

A compromised GitHub release asset replaces the running binary silently
with the current user's privileges on every auto-update. HTTPS prevents
network interception but not a compromised account or repository.

**Fix:** Follow the two-phase rollout in `SECURITY-NOTES.md` exactly:
1. Generate keypair and sign releases first (CI workflow).
2. Only then wire `.verifying_keys([RELEASE_VERIFYING_KEY])` in both
   `perform_update_sync()` and `perform_update_to_version_sync()`.
3. Commit `signing/claude-workbench-pub.bin`; never commit the private key.

```rust
const RELEASE_VERIFYING_KEY: &[u8] =
    include_bytes!("../../signing/claude-workbench-pub.bin");

match Update::configure()
    .repo_owner(REPO_OWNER)
    .repo_name(REPO_NAME)
    .bin_name(BIN_NAME)
    .target(target)
    .current_version(CURRENT_VERSION)
    .verifying_keys([RELEASE_VERIFYING_KEY])   // ← ADD
    .show_download_progress(false)
    .show_output(false)
    .no_confirm(true)
    .build()
```

**Effort:** S (10 min client code) + M (CI signing pipeline, ~90 min total)

---

### CR-02: `--update-to` Allows Unauthenticated Downgrade to Vulnerable Versions in Release Builds

**File:** `src/main.rs:48-50`, `src/main.rs:321-322`, `src/update/install.rs:80-146`

`--fake-version` is correctly gated behind `#[cfg(debug_assertions)]`
(line 44). `--update-to` is NOT gated — it is present in release binaries
and accepts any version string from the command line:

```rust
/// Update to a specific version (for testing/downgrade ...)
#[arg(long)]
update_to: Option<String>,      // ← no cfg gate
```

Any local user who can invoke the binary can silently downgrade it to any
older release (including pre-security-fix versions) with:

```
claude-workbench --update-to 0.1.0
```

Once signature verification (CR-01) is wired, downgraded binaries will
still be accepted because old releases will have been signed before
verification was required. The combination makes CR-01 partially
bypassable: downgrade to a pre-verification binary, then that binary
auto-updates without verifying.

**Fix (two options, pick one):**

Option A — gate behind `cfg(debug_assertions)` like `--fake-version`:
```rust
#[cfg(debug_assertions)]
#[arg(long)]
update_to: Option<String>,
```

Option B — keep in release, require explicit confirmation prompt:
```rust
if let Some(target_version) = args.update_to {
    eprintln!("WARNING: Downgrading is a security risk. Type 'yes' to continue:");
    let mut line = String::new();
    std::io::stdin().read_line(&mut line)?;
    if line.trim() != "yes" {
        eprintln!("Aborted.");
        std::process::exit(1);
    }
    return run_update_to_version_cli(&target_version);
}
```

Option A is strongly preferred — the flag exists for testing and should
not be in distributed binaries.

**Effort:** S

---

### CR-03: Predictable Temp File Path — Symlink Redirect on Multi-User Systems

**File:** `src/browser/pdf_export.rs:108-120`
**Maps to:** SEC-04 / MEDIUM — confirmed open, promoted to Critical

`default_preview_filename()` constructs:
```rust
let name = format!("{}-{}-{}.html", project_name, stem, date);
std::env::temp_dir().join(name)
// → /tmp/myproject-README-11.05.2026.html
```

This path is fully predictable (date granularity is one day, stem comes
from the filename, project name comes from the directory name — all
observable without privileges). On a shared Linux host (common in the
XRDP/SSH target environment), a local attacker pre-creates:

```bash
ln -s /home/victim/.ssh/authorized_keys /tmp/myproject-README-11.05.2026.html
```

Then when the victim opens a Markdown preview, the HTML template is
written to their `authorized_keys` file. This is a concrete write-to-
arbitrary-file via symlink. The `CONCERNS.md` classified it MEDIUM, but
on multi-user systems (which are explicitly in scope per XRDP targeting)
this is exploitable without privileges and deserves Critical.

**Fix:**
```rust
use tempfile::Builder;

pub fn default_preview_filename(source: &Path, project_name: &str) -> Result<tempfile::NamedTempFile> {
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
        .map_err(Into::into)
}
```

`tempfile` is already in `Cargo.toml`. Switch callers to hold the
`NamedTempFile` handle (auto-deletes on drop) and store it in
`App::temp_preview_files` as `Vec<tempfile::NamedTempFile>` instead of
`Vec<PathBuf>`. This simultaneously fixes the cleanup-on-SIGKILL gap
noted in `CONCERNS.md`.

**Effort:** M (signature change propagates to callers)

---

## Warning Findings

### WR-01: Browser/Editor Command Construction Has No Allow-List

**File:** `src/browser/opener.rs:83-107`, `src/browser/opener.rs:111-138`
**Maps to:** SEC-02 (MEDIUM — confirmed open)

`open_file_with_browser()` and `open_file_with_editor()` call
`split_command(browser)` and pass the first token directly to
`Command::new()`. No validation of the program name against a pattern
or `$PATH` entry:

```rust
pub fn open_file_with_browser(path: &Path, browser: &str) -> Result<()> {
    if browser.is_empty() {
        open_file(path)
    } else {
        let (program, args) = split_command(browser);
        std::process::Command::new(&program)  // ← no validation
            .args(&args)
            .arg(path)
            .spawn()?;
        Ok(())
    }
}
```

Today `browser` is user-owned config. The risk materialises if any future
code path writes PTY output, URL fragments, or file-derived metadata into
`config.ui.browser` — then `Command::new` executes an attacker-controlled
string. The hand-rolled `split_command()` also fails to handle single
quotes or backslash escapes, so `open -a 'Brave Browser'` would silently
misparse (minor UX bug, not exploitable).

**Fix:**
```rust
fn validate_program(prog: &str) -> anyhow::Result<()> {
    // Only allow safe characters in program name
    let safe = prog.chars().all(|c| c.is_ascii_alphanumeric()
        || matches!(c, '_' | '-' | '.' | '/' | '+'));
    if !safe || prog.is_empty() {
        anyhow::bail!("Unsafe program name in browser/editor config: {:?}", prog);
    }
    Ok(())
}

pub fn open_file_with_browser(path: &Path, browser: &str) -> Result<()> {
    if browser.is_empty() {
        open_file(path)
    } else {
        let (program, args) = split_command(browser);
        validate_program(&program)?;          // ← ADD
        std::process::Command::new(&program)
            .args(&args)
            .arg(path)
            .spawn()?;
        Ok(())
    }
}
```

Apply the same guard in `open_file_with_editor`. Also consider replacing
`split_command` with `shlex::split` (already a dependency) for correct
single-quote and backslash handling.

**Effort:** S

---

### WR-02: Shell Fallback in Dependency Probe Uses `$SHELL -i -c`

**File:** `src/setup/dependency_checker.rs:172-191`
**Maps to:** SEC-03 (MEDIUM — confirmed open)

The interactive shell fallback in `check_command()` constructs a command
string via `shlex::try_quote` and passes it to `$SHELL -i -c`:

```rust
let shell_cmd = std::iter::once(name.to_string())
    .chain(args.iter().map(|a| a.to_string()))
    .map(|a| shlex::try_quote(&a).map(|c| c.into_owned()).unwrap_or(a))
    .collect::<Vec<_>>()
    .join(" ");

Command::new(&user_shell).args(["-i", "-c", &shell_cmd]).output()
```

All current call sites pass static string literals for both `name` and
`args`, so no injection is possible today. The risk is the pattern itself:
`check_command` is a public function within the module and `args` is
`&[&str]`. A future caller passing dynamic input (e.g., a user-provided
tool name from a config field) and a `shlex` edge case (NUL bytes,
certain Unicode) would produce shell injection. The comment says "Use
shell-escaped arguments to prevent injection" but `shlex::try_quote`'s
fallback (`unwrap_or(a)`) means unquotable strings are passed raw.

**Fix:** Replace the `-i -c` path with direct `Command::new`:
```rust
#[cfg(not(windows))]
let shell_result = Command::new(name).args(args).output();
```

If the `-i` flag is genuinely required to resolve shell functions (e.g.,
the `claude` function alias in Fish), restrict it to an explicit allow-list:
```rust
const ALIAS_RESOLVED: &[&str] = &["claude"];
if ALIAS_RESOLVED.contains(&name) {
    // use shell fallback
} else {
    // direct Command::new
}
```

**Effort:** S

---

### WR-03: `which()` Checks `is_file()` But Not Executable Bit

**File:** `src/clipboard.rs:120-129`

```rust
pub fn which(name: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path) {
        let candidate = dir.join(name);
        if candidate.is_file() {      // ← no executable check
            return Some(candidate);
        }
    }
    None
}
```

`is_file()` returns true for non-executable regular files (e.g., data
files named `xclip` on `$PATH`). On a tampered `$PATH` (e.g.,
`.cargo/bin` with a non-executable file), `which` returns the path, the
clipboard strategy switches to `SubprocessFirst`, and `Command::new`
fails with "Permission denied" at copy time — producing a `Failed`
outcome with no clear diagnostic. This is a reliability bug, not a
security hole, but it would be confusing to diagnose.

**Fix:**
```rust
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
                    continue; // not executable
                }
            }
            return Some(candidate);
        }
    }
    None
}
```

**Effort:** S

---

### WR-04: `shlex::try_quote` Failure in PTY `cd` Injection Falls Back to Unescaped String

**File:** `src/app/pty.rs:152-153`, `:165-166`, `:179-180`

All three `sync_terminals*` functions and `insert_path_at_cursor` share
this pattern:
```rust
let escaped = shlex::try_quote(&path_str)
    .map(|c| c.into_owned())
    .unwrap_or_else(|_| path_str.to_string()); // ← raw unescaped fallback
let cmd = format!("cd {}\r", escaped);
```

`shlex::try_quote` fails when the input contains a NUL byte (returns
`Err`). On Unix, filesystem paths cannot contain NUL, so this is
unreachable in practice. However `path_str` is derived from
`to_string_lossy()` on a `PathBuf`, which replaces invalid UTF-8 with
`U+FFFD`. The fallback means a path with unusual bytes (non-UTF-8 on
Linux) gets injected into the shell verbatim. On most shells a
replacement character is harmless; but the silent `unwrap_or_else`
hides the failure.

**Fix:** Fail loudly rather than fall back silently. If `try_quote`
returns `Err`, skip the sync and log it:
```rust
match shlex::try_quote(&path_str) {
    Ok(escaped) => {
        let cmd = format!("cd {}\r", escaped);
        if let Some(pty) = self.terminals.get_mut(&PaneId::Terminal) {
            let _ = pty.write_input(cmd.as_bytes());
        }
    }
    Err(_) => {
        // path contains NUL or is otherwise unquotable — skip cd sync
        log_update(&format!("sync_terminals: skipping unquotable path: {:?}",
            self.file_browser.current_dir));
    }
}
```

**Effort:** S

---

### WR-05: `check.rs` Trusts `releases[0]` as Latest Without Version Ordering

**File:** `src/update/check.rs:42-43`

```rust
let latest = &releases[0];
let target_version = &latest.version;
```

The GitHub Releases API returns releases in descending creation-date
order by default, not by semantic version. If a patch release for an
older branch (e.g., `v0.85.1`) is published after `v0.89.0`, the API
returns `v0.85.1` first. The update check would then report a downgrade
as "up to date" or even as an "update available" (depending on the
`version_newer` comparison), potentially preventing users from seeing
the real latest version.

This is a reliability bug that could be exploited: a maintainer account
could post a backdated release to suppress legitimate updates.

**Fix:** Select the release with the highest semantic version rather than
trusting list order:
```rust
let latest = releases
    .iter()
    .max_by(|a, b| {
        // strip 'v' prefix before comparing
        let va = a.version.strip_prefix('v').unwrap_or(&a.version);
        let vb = b.version.strip_prefix('v').unwrap_or(&b.version);
        compare_semver(va, vb)   // implement using version_newer logic
    });
let Some(latest) = latest else {
    return UpdateCheckResult::NoReleasesFound;
};
```

**Effort:** S

---

## Info Findings

### IN-01: OSC 52 Always Reports Success — Misleading Outcome

**File:** `src/clipboard.rs:288-295`

```rust
// Last resort: OSC 52. We always claim success here because we can't
// verify whether the terminal forwarded it ...
osc52_copy(text);
let _ = errors;
ClipboardOutcome::Osc52
```

`ClipboardOutcome::Osc52.is_success()` returns `true`. The user sees a
success flash even when the terminal silently discards the escape (e.g.,
xrdp-chansrv ignores OSC 52 in most configs). The collected `errors` from
all failed prior backends are also discarded. No way to distinguish "OSC 52
was the intended strategy" from "everything else failed and we fell back."

**Fix:** Introduce `ClipboardOutcome::Osc52Fallback` to distinguish the
last-resort path, or at minimum preserve the errors for `--clipboard-diag`.
The success flash is acceptable but the caller should know it is
unverified:
```rust
pub enum ClipboardOutcome {
    // ...
    Osc52,          // used when strategy is Osc52Only (intentional)
    Osc52Fallback,  // used when all other backends failed (unverified)
    // ...
}
```

**Effort:** S

---

### IN-02: `restart_application()` Re-Executes With All Original CLI Args Including `--update-to`

**File:** `src/update/install.rs:212-213`

```rust
let args: Vec<String> = std::env::args().skip(1).collect();
// ...
let mut cmd = std::process::Command::new(&exe);
cmd.args(&args);
let error = cmd.exec();
```

If the user invoked `claude-workbench --update-to v0.38.0`, completed
the downgrade, and the app restarts, it re-executes with `--update-to
v0.38.0` again — triggering another downgrade immediately. This is an
infinite loop on the downgrade path. In practice `restart_application()`
is only called after a normal auto-update, not after `--update-to`, but
the re-exec does not strip potentially dangerous flags.

**Fix:** Filter known one-shot flags before re-exec:
```rust
let args: Vec<String> = std::env::args()
    .skip(1)
    .filter(|a| !matches!(a.as_str(), "--update-to" | "--check-update" | "--clipboard-diag" | "--ssh-paste-diag"))
    .collect();
```

**Effort:** S

---

### IN-03: `date_now_dmy()` Uses `unsafe` `libc::localtime_r` Without Bounds Check on `tm_mon`

**File:** `src/browser/pdf_export.rs:78-103`

```rust
let time_t = now as libc::time_t;
let mut tm: libc::tm = unsafe { std::mem::zeroed() };
unsafe { libc::localtime_r(&time_t, &mut tm); }
format!("{:02}.{:02}.{}", tm.tm_mday, tm.tm_mon + 1, tm.tm_year + 1900)
```

`tm.tm_mon` is 0-indexed (0–11). If `localtime_r` fails (e.g., TZ
database missing), `tm` remains zeroed — `tm_mon = 0` produces
`01.01.1900`, which is wrong but not exploitable. A simpler concern: if
the `time_t` cast from `u64` to `i64` overflows (after year 2038 on
32-bit time_t systems — unlikely but the cast is silent), the output
date is garbage. The `unsafe` block is minimally scoped, which is good.

**Fix:** Replace the entire `unsafe` block with the `chrono` crate (or
use `std::time` and a manual calculation). Alternatively, add a defensive
check:
```rust
// Defensive: ensure localtime_r succeeded (non-zero year range)
let year = tm.tm_year + 1900;
let month = (tm.tm_mon + 1).clamp(1, 12);
let day = tm.tm_mday.clamp(1, 31);
format!("{:02}.{:02}.{}", day, month, year)
```

**Effort:** S

---

## Recommended Phase 1 Plan Outline

### Plan A — Self-Update Supply Chain (SEC-01, CR-01)
**Findings:** CR-01
**Complexity:** M
**Execution order:** 1st (prerequisite for all other update security)
**Steps:**
1. Generate ed25519 keypair (`zipsign generate-keys`); store private key as GitHub Actions secret.
2. Commit `signing/claude-workbench-pub.bin` to repo.
3. Add signing step to `.github/workflows/release.yml` before `gh release upload`.
4. Ship 2–3 signed releases before enabling verification in the client.
5. Wire `.verifying_keys([RELEASE_VERIFYING_KEY])` in both `perform_update_sync` and `perform_update_to_version_sync`.

---

### Plan B — Downgrade Attack Surface (CR-02, IN-02)
**Findings:** CR-02, IN-02
**Complexity:** S
**Execution order:** 2nd (independent of Plan A, but logical companion)
**Steps:**
1. Gate `--update-to` behind `#[cfg(debug_assertions)]`.
2. Filter one-shot flags from `restart_application()` args.
3. Add unit test asserting `--update-to` is absent from release arg parser.

---

### Plan C — Temp File Symlink Hardening (CR-03)
**Findings:** CR-03
**Complexity:** M
**Execution order:** 3rd (independent of Plans A/B)
**Steps:**
1. Change `default_preview_filename` to return `Result<tempfile::NamedTempFile>`.
2. Update all callers to store `NamedTempFile` handles (auto-delete on drop).
3. Replace `Vec<PathBuf>` in `App::temp_preview_files` with `Vec<tempfile::NamedTempFile>`.
4. Remove the manual cleanup-on-exit path (now handled by drop).

---

### Plan D — Browser/Editor Command Validation (WR-01)
**Findings:** WR-01
**Complexity:** S
**Execution order:** 4th (independent)
**Steps:**
1. Add `validate_program()` function in `opener.rs`.
2. Call it in both `open_file_with_browser` and `open_file_with_editor`.
3. Replace hand-rolled `split_command` with `shlex::split` for correctness.
4. Add unit tests for malformed program names and quoted args.

---

### Plan E — Shell Fallback Elimination (WR-02)
**Findings:** WR-02
**Complexity:** S
**Execution order:** 5th (independent)
**Steps:**
1. Replace `$SHELL -i -c` with `Command::new(name).args(args)` for the non-Windows path.
2. If shell-alias resolution is required for `claude`, add explicit allow-list gate.
3. Verify `check_command("claude", ...)` still finds the Claude CLI via direct exec.
4. Add unit test for `check_command_not_found` and a mock for the alias path.

---

### Plan F — Update Check Correctness + Minor Hardening (WR-03, WR-04, WR-05, IN-01, IN-03)
**Findings:** WR-03, WR-04, WR-05, IN-01, IN-03
**Complexity:** S
**Execution order:** 6th (independent, can be split across minor PRs)
**Steps:**
1. Fix `which()` to check executable bit on Unix.
2. Fix `check.rs` to select max-semver release rather than `releases[0]`.
3. Harden `sync_terminals*` to log and skip on `shlex::try_quote` failure.
4. Introduce `ClipboardOutcome::Osc52Fallback` for the last-resort path.
5. Add `clamp` defensive guards in `date_now_dmy()`.

---

*Review generated: 2026-05-11*
*Reviewer: Claude (gsd-code-reviewer)*
*Depth: standard*
