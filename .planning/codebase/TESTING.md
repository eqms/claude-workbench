# Testing Patterns

**Analysis Date:** 2026-05-11

## Test Framework

**Runner:**
- `cargo test` (standard Rust built-in test harness)
- No separate test runner, no jest/pytest equivalents
- Config: none (uses Cargo defaults)

**Assertion Library:**
- Standard `assert!`, `assert_eq!`, `assert_ne!` macros only
- No `pretty_assertions`, `proptest`, or similar crates

**Run Commands:**
```bash
cargo test              # Run all tests (111 passing as of v0.89.0)
cargo test -- --nocapture  # Show println! output from tests
cargo test test_name    # Run specific test by name substring
cargo clippy            # Lint (16 warnings pre-audit, 0 post-audit target)
cargo fmt -- --check    # Check formatting
```

## Test File Organization

**Location:** All tests co-located in source files as inline `#[cfg(test)]` modules. No separate `tests/` directory exists. No integration test directory.

**Naming:** Test functions use `test_` prefix (enforced by convention, not tooling):
- `test_prompt_filtering`, `test_version_newer_basic`, `test_auto_mode_cli_flag`

**Structure:**
```
src/
├── types.rs          # 8 tests — enum API correctness
├── config.rs         # 4 tests — shell detection, config defaults
├── filter.rs         # 4 tests — terminal output filtering logic
├── clipboard.rs      # ~5 tests — SSH detection, strategy detection
├── app_detector.rs   # tests — app detection helpers
├── filter.rs         # 4 tests — line filtering, syntax detection
├── git/mod.rs        # tests — git status parsing
├── update/mod.rs     # 5 tests — version comparison, state transitions
├── browser/markdown.rs    # tests — HTML generation
├── browser/template.rs    # tests — template rendering
├── browser/syntax.rs      # tests — syntax highlighting
├── browser/typst_pdf.rs   # tests — PDF generation (feature-gated)
├── browser/pdf_export.rs  # tests — PDF export
├── setup/wizard.rs        # tests — setup wizard logic
├── setup/dependency_checker.rs  # tests — dependency detection
├── app/pty.rs        # tests — PTY state management
├── ui/settings.rs    # tests — settings UI state
└── syntax_registry.rs     # tests — syntax registry lookup
```

## Test Structure

**Suite Organization:**
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_something() {
        // arrange
        let input = vec!["line".to_string()];
        // act
        let result = some_function(input);
        // assert
        assert_eq!(result.field, expected);
    }
}
```

**Patterns:**
- `use super::*;` to import all items from the parent module
- No `before_each`/`after_each` — test functions are fully self-contained
- Arrange/act/assert structure without explicit labels
- Platform-gated tests use `#[cfg(windows)]` / `#[cfg(not(windows))]` on the test function itself

## Mocking

**Framework:** None. No `mockall`, `mock_it`, or similar crates.

**Patterns:**
- Pure helper functions extracted specifically to enable testing without env access:
  ```rust
  // Public cached version (untestable — reads env, uses OnceLock)
  pub fn is_ssh_session() -> bool { *IS_SSH.get_or_init(|| detect_ssh_session(...)) }

  // Private pure helper (testable — no side effects)
  fn detect_ssh_session(ssh_tty: Option<&OsStr>, ssh_connection: Option<&OsStr>) -> bool {
      let nonempty = |v: Option<&OsStr>| v.map(|s| !s.is_empty()).unwrap_or(false);
      nonempty(ssh_tty) || nonempty(ssh_connection)
  }
  ```
- Git/subprocess calls not mocked — tests that require git CLI are avoided or skipped
- No test doubles, no dependency injection infrastructure

**What to mock:** Nothing — design for testability via pure function extraction instead.

**What NOT to mock:** File system, git CLI, PTY operations — these are excluded from unit tests entirely.

## Fixtures and Factories

**Test Data:** Inline literals in each test function. No shared fixtures or factory helpers:
```rust
#[test]
fn test_prompt_filtering() {
    let input = vec![
        "user@host:~$ ".to_string(),
        "ls -la".to_string(),
        "total 123".to_string(),
    ];
    let options = FilterOptions::default();
    let result = filter_lines(input, &options);
    assert_eq!(result.lines.len(), 1);
}
```

**Default trait usage:** `FilterOptions::default()`, `Config::default()`, `UpdateState::new()` used as test fixtures — the `Default` impl serves as the canonical test baseline.

**Location:** All fixtures inline — no `tests/fixtures/` directory, no separate data files.

## Coverage

**Requirements:** None enforced. No `cargo-tarpaulin` or `grcov` configured.

**Current state:** 111 passing tests as of v0.89.0 (verified post-audit).

**View Coverage:**
```bash
# Not configured — add cargo-tarpaulin for coverage reporting:
cargo install cargo-tarpaulin
cargo tarpaulin --out Html
```

**Known gaps:** PTY/terminal rendering, UI rendering functions, mouse event routing, clipboard async worker, most of `src/app/mod.rs` event loop — none of these have unit tests. See `CONCERNS.md`.

## Test Types

**Unit Tests:** 100% of tests are unit tests. Co-located inline `#[cfg(test)]` modules testing pure logic.

**Integration Tests:** None (`tests/` directory does not exist).

**E2E Tests:** None. No Playwright, no terminal automation.

**Doc Tests:** None. No `///` examples with ```` ```rust ```` blocks that run under `cargo test`.

## Common Patterns

**Enum exhaustiveness testing:**
```rust
#[test]
fn test_all_permission_modes_have_unique_names() {
    let all = ClaudePermissionMode::all();
    let names: Vec<&str> = all.iter().map(|m| m.name()).collect();
    let unique: std::collections::HashSet<&&str> = names.iter().collect();
    assert_eq!(names.len(), unique.len(), "duplicate permission mode names");
}
```

**State transition testing:**
```rust
#[test]
fn test_update_state_transitions() {
    let mut state = UpdateState::new();
    state.start_check();
    assert!(state.checking);
    state.set_available("1.0.0".to_string(), Some("Release notes".to_string()));
    assert!(!state.checking);
    assert!(state.show_dialog);
    state.close_dialog();
    assert!(!state.show_dialog);
}
```

**Version comparison / ordering:**
```rust
#[test]
fn test_version_newer_basic() {
    assert!(version_newer("0.38.0", "0.37.2"));
    assert!(version_newer("1.0.0", "0.99.99"));
    assert!(!version_newer("0.37.2", "0.37.2"));
}
```

**Error/boundary testing:**
```rust
#[test]
fn test_traceback_preservation() {
    let input = vec![
        "Traceback (most recent call last):".to_string(),
        "  File \"test.py\", line 10".to_string(),
        "ValueError: test error".to_string(),
    ];
    let result = filter_lines(input, &FilterOptions::default());
    assert!(result.contains_error);
    assert_eq!(result.lines.len(), 3);
}
```

**Platform-conditional tests:**
```rust
#[cfg(not(windows))]
#[test]
fn default_shell_path_unix_is_absolute_or_shell_env() {
    let s = default_shell_path();
    assert!(s.starts_with('/'), "got: {s}");
}
```

## Adding New Tests

**Rule:** Every pure function or state machine method should have a corresponding test in the same file's `#[cfg(test)] mod tests` block.

**Template for a new testable module:**
```rust
// Production code above...

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_is_sane() {
        let s = MyStruct::default();
        assert_eq!(s.field, expected_value);
    }
}
```

**When to extract a pure helper for testing** (follow the `detect_ssh_session` pattern):
- Function reads env vars or global state → extract pure inner function taking those values as parameters
- Function spawns threads or processes → test the pure data-transformation logic separately

---

*Testing analysis: 2026-05-11*
