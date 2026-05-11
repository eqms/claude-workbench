# Testing Patterns

**Analysis Date:** 2026-05-11

## Test Framework

**Runner:**
- `cargo test` (standard Rust test harness, no nextest configured)
- Config: `Cargo.toml` — no separate test runner config file

**Assertion Library:**
- Standard `assert!`, `assert_eq!`, `assert_ne!` macros
- No external assertion crates (no `asserting`, no `pretty_assertions`)
- No `assert_cmd` or `predicates` crates — CLI integration tests use `std::process::Command` directly

**Run Commands:**
```bash
cargo test                    # Run all tests (unit + integration)
cargo test --release          # Run all tests including #[cfg(not(debug_assertions))] tests
cargo test test_name          # Run specific test by name substring
cargo test -- --nocapture     # Show println!/eprintln! output
cargo test -p claude-workbench -- --test-threads=1   # Sequential (for stateful tests)
```

## Test File Organization

**Location:**
- **Unit tests**: Co-located with source — `#[cfg(test)] mod tests { ... }` at bottom of each `.rs` file
- **Integration tests**: `tests/` directory at project root — compiled as separate crates that invoke the binary

**Naming:**
- Unit test functions: `test_<what_is_tested>_<condition>` (e.g., `test_filter_restart_args_removes_one_shot_flags`)
- Integration test files: named after the entry-point feature: `tests/cli.rs`
- No `_spec` suffix; no `should_` prefix

**Structure:**
```
workbench/
├── src/
│   ├── app/pty.rs                    # unit tests: quote_path_for_cd, build_claude_command
│   ├── browser/opener.rs             # unit tests: validate_program
│   ├── browser/pdf_export.rs         # unit tests: RAII tempfile, filename generation
│   ├── clipboard.rs                  # unit tests: base64, which, is_executable, SSH detection
│   ├── setup/dependency_checker.rs   # unit tests: check_command
│   ├── update/check.rs               # unit tests: semver max selection
│   └── update/install.rs             # unit tests: filter_restart_args
└── tests/
    └── cli.rs                        # integration tests: --help, --version, unknown flag, release-mode flag gate
```

## Test Suite Size

- **Pre-Wave-1 baseline:** 111 passing tests
- **Post-Wave-1:** 130 passing unit tests + 3 integration tests in `tests/cli.rs`
- Total: 133 tests

## Test Structure

**Unit suite organization:**
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_<subject>_<condition>() {
        // arrange
        let input = ...;
        // act
        let result = function_under_test(input);
        // assert
        assert_eq!(result, expected, "descriptive failure message: {result:?}");
    }
}
```

**Helper factory functions** (no macro-based test infrastructure):
```rust
fn config_with_claude_command() -> Config { ... }
fn base_opts() -> StartupOptions { ... }
```
These are plain `fn` inside the `mod tests` block — not `#[fixture]` or similar.

**Integration test structure** (`tests/cli.rs`):
```rust
fn workbench_binary() -> &'static str {
    env!("CARGO_BIN_EXE_claude-workbench")  // resolves binary path at compile time
}

#[test]
fn help_prints_usage_and_exits_zero() {
    let output = Command::new(workbench_binary())
        .arg("--help")
        .output()
        .expect("failed to invoke claude-workbench --help");
    assert!(output.status.success(), "...");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Usage"), "...");
}
```

## Mocking

**Framework:** None — no mock crate (`mockall`, `double`, etc.) in use.

**Patterns:**
- External processes tested via real binaries (`git`, `sh`) — no process mocking
- File system tested via `tempfile::NamedTempFile` for RAII temp files (`src/browser/pdf_export.rs`)
- Platform-specific behavior isolated with `#[cfg(unix)]` / `#[cfg(windows)]` blocks inside test functions

**What to Mock:**
- Not applicable — pure-function unit tests dominate; integration tests hit the real binary

**What NOT to Mock:**
- The compiled binary in integration tests — `CARGO_BIN_EXE_*` resolves the real build artifact

## Fixtures and Factories

**Test Data:**
```rust
// Factory functions for complex structs — no fixture files
fn config_with_claude_command() -> Config {
    let mut cfg = Config::default();
    cfg.pty.claude_command = vec!["claude".to_string()];
    cfg
}

fn base_opts() -> StartupOptions {
    StartupOptions {
        permission_mode: ClaudePermissionMode::Default,
        model: ClaudeModel::Unset,
        effort: ClaudeEffort::Unset,
        session_name: String::new(),
        worktree: String::new(),
        remote_control: false,
    }
}
```

**Location:**
- Inline within `mod tests` — no separate fixture files or `tests/fixtures/` directory

## Coverage

**Requirements:** None enforced (no `cargo-llvm-cov` config, no coverage gate in CI)

**View Coverage** (manual):
```bash
cargo llvm-cov --html   # if cargo-llvm-cov installed
```

## Test Types

**Unit Tests (130 passing):**
- Scope: single function or small module in isolation
- No I/O except `tempfile::NamedTempFile` for RAII pattern tests
- Pure logic preferred (semver selection, arg filtering, path quoting, base64)

**Integration Tests (3 in `tests/cli.rs`):**
- Scope: full binary invocation via `std::process::Command`
- Verify: exit codes, stdout content, stderr content
- Use `env!("CARGO_BIN_EXE_claude-workbench")` to locate the compiled binary
- Require `cargo build` or `cargo test` (not `cargo test --no-run`) to have run first

**E2E Tests:** Not used — no Playwright or similar harness.

## New Test Patterns Introduced by Wave 1

### CLI Integration Pattern (`tests/cli.rs`)
Invoke binary via `std::process::Command` + `CARGO_BIN_EXE_<name>`:
```rust
fn workbench_binary() -> &'static str {
    env!("CARGO_BIN_EXE_claude-workbench")
}

let output = Command::new(workbench_binary())
    .arg("--help")
    .output()
    .expect("...");
assert!(output.status.success(), "exit code: {:?}", output.status);
let stdout = String::from_utf8_lossy(&output.stdout);
assert!(stdout.contains("Usage"), "stdout: {}", stdout);
```

### Compile-Time Flag Gate Testing
Verify flags are absent in release builds using `#[cfg(not(debug_assertions))]`:
```rust
#[test]
#[cfg(not(debug_assertions))]
fn update_to_flag_not_present_in_release_build() {
    let output = Command::new(workbench_binary())
        .args(["--update-to", "0.1.0"])
        .output()
        .expect("...");
    assert_eq!(output.status.code(), Some(2));  // clap exits 2 for unknown args
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("unexpected argument") || stderr.contains("unrecognized"));
}
```

### Stateful Iterator Filtering Pattern (`src/update/install.rs`)
Test `skip_next` stateful filtering by verifying full removal of flag+value pairs:
```rust
#[test]
fn test_filter_restart_args_removes_one_shot_flags() {
    let input = vec!["--update-to", "0.1.0", "--check-update", ...].map(String::from);
    let filtered = filter_restart_args(input.into_iter());
    assert!(filtered.is_empty(), "all one-shot flags must be removed, got: {filtered:?}");
}
```

### RAII Tempfile Lifecycle Testing (`src/browser/pdf_export.rs`)
Verify deletion-on-drop by capturing the path before drop then asserting absence:
```rust
#[test]
fn test_namedtempfile_deletes_on_drop() {
    let path = {
        let tmp = default_preview_file(Path::new("x.md"), "p").unwrap();
        tmp.path().to_path_buf()
    }; // tmp drops here — file must be deleted
    assert!(!path.exists(), "file must be deleted after NamedTempFile drops");
}
```

### Unix Permission Testing (`src/clipboard.rs`)
Test executable bit logic with `std::os::unix::fs::PermissionsExt`:
```rust
#[test]
#[cfg(unix)]
fn test_is_executable_respects_mode() {
    use std::os::unix::fs::PermissionsExt;
    let tmp = tempfile::NamedTempFile::new().unwrap();
    std::fs::set_permissions(tmp.path(), Permissions::from_mode(0o644)).unwrap();
    assert!(!is_executable(tmp.path()));
    std::fs::set_permissions(tmp.path(), Permissions::from_mode(0o755)).unwrap();
    assert!(is_executable(tmp.path()));
}
```

### Pure Semver Logic Testing (`src/update/check.rs`)
Test algorithmic correctness independent of network/GitHub by constructing tag vectors inline:
```rust
#[test]
fn test_max_semver_selection_ignores_creation_order() {
    let tags = vec!["v0.85.1", "v0.89.0", "v0.88.3", "v0.85.2"];
    let best = tags.iter()
        .filter_map(|t| { let s = t.strip_prefix('v').unwrap_or(t); Version::parse(s).ok().map(|v| (v, t)) })
        .max_by(|(va, _), (vb, _)| va.cmp(vb))
        .map(|(_, t)| *t);
    assert_eq!(best, Some("v0.89.0"));
}
```

### Security Boundary Testing (`src/browser/opener.rs`)
Test allowlist via reject-on-metacharacter pattern — one `assert!` per metachar with label:
```rust
#[test]
fn test_validate_program_rejects_metacharacters() {
    assert!(validate_program("").is_err(), "empty must be rejected");
    assert!(validate_program("fire;fox").is_err(), "semicolon");
    assert!(validate_program("$(rm -rf /)").is_err(), "subshell");
    assert!(validate_program("a b").is_err(), "space");
    assert!(validate_program("a|b").is_err(), "pipe");
    assert!(validate_program("a&b").is_err(), "ampersand");
    assert!(validate_program("a`b`").is_err(), "backtick");
}
```

## Common Patterns

**Async Testing:** Not used — async code tested indirectly through sync wrappers (e.g., `perform_update_sync`); `mpsc::channel` used for threading, tested at the sync boundary.

**Error Testing:**
```rust
assert!(result.is_err(), "descriptive reason");
// For specific error messages — not yet standardized; use string matching on Display
```

**Panic-safety checks** (used for `DiagCollect` and SSH detection):
```rust
#[test]
fn test_diag_collect_does_not_panic() {
    let diag = ClipboardDiag::collect();
    assert!(matches!(diag.strategy, ClipboardStrategy::ArboardFirst | ClipboardStrategy::SubprocessFirst));
}
```

**Assertion message convention:** Always include the actual value in the failure message:
```rust
assert!(condition, "descriptive label: {value:?}");
assert_eq!(actual, expected, "context: {extra}");
```

---

*Testing analysis: 2026-05-11*
