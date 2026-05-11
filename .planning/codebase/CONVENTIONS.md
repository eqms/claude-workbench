# Coding Conventions

**Analysis Date:** 2026-05-11

## Naming Patterns

**Files:**
- `snake_case.rs` for all source files
- Module directories use `snake_case/` with `mod.rs` or a single representative file
- Integration tests in `tests/` mirror the feature being tested: `tests/cli.rs` for CLI entry-point contracts

**Functions:**
- `snake_case` for all functions, methods, and closures
- Boolean-returning predicates prefixed: `is_executable`, `is_markdown`, `is_ssh_session`
- Constructors follow Rust convention: `check()`, `collect()`, `new()`, `default()`
- `pub(super)` used for intra-module public API (e.g., `sync_terminals`, `sync_terminals_initial` in `src/app/pty.rs`)

**Variables:**
- `snake_case` throughout
- Short-lived temporaries use abbreviated names: `tmp`, `cfg`, `cmd`, `arg`
- Descriptive names for complex values: `filtered`, `skip_next`, `target_tag`

**Types:**
- Structs and enums: `PascalCase`
- Enum variants: `PascalCase` (e.g., `ClaudePermissionMode::Auto`, `ClipboardOutcome::Arboard`)
- Derive macros grouped: `#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]`
- Full derive set used on public enum types in `src/types.rs`

**Constants/Statics:**
- `SCREAMING_SNAKE_CASE`: `CURRENT_VERSION`, `BIN_NAME`, `REPO_OWNER`, `REPO_NAME`

## Code Style

**Formatting:**
- Tool: `rustfmt` with `rustfmt.toml`
- `edition = "2021"`
- `reorder_imports = true`
- `reorder_modules = true`
- `newline_style = "Unix"`

**Linting:**
- Tool: `cargo clippy`
- 15 `collapsible_match` warnings pre-exist in `src/input/keyboard/dialogs.rs`, `src/input/mouse.rs`, `src/app/file_ops.rs`, `src/ui/typst_pdf.rs` — deferred, not to be introduced in new code
- Targeted `#[allow(clippy::...)]` at call-site rather than module-wide suppression
- Known inline suppression: `#[allow(clippy::type_complexity)]` in `src/types.rs:606`, `#[allow(clippy::needless_range_loop)]` in `src/app/clipboard.rs:162`

## Import Organization

**Order (enforced by rustfmt `reorder_imports = true`):**
1. `std::` standard library
2. External crates (alphabetical)
3. `super::` or `crate::` local imports

**Example pattern** (from `src/update/install.rs`):
```rust
use std::sync::mpsc;
use std::thread;

use self_update::backends::github::Update;

use super::check::get_target;
use super::log::log_update;
```

**Path Aliases:** None in use — explicit paths preferred.

## Error Handling

**Primary crate:** `anyhow` throughout the codebase.

**Patterns in use:**
- `anyhow::bail!("...")` for early return with formatted error (e.g., `src/browser/opener.rs:91`)
- `anyhow::anyhow!("...")` with `ok_or_else(|| ...)` for `Option → Result` conversion
- `.context("...")` for adding context to propagated errors
- `Result<()>` as standard return type for fallible operations; `Result<T>` where value is needed
- `anyhow::Result` aliased via `use anyhow::Result` at file top

**`validate_program` error contract** (`src/browser/opener.rs`):
- `fn validate_program(prog: &str) -> Result<()>`
- Returns `Err` for: empty string, any char not in `[a-zA-Z0-9_\-./+]`
- Error message: `"Unsafe program name in browser/editor config: {:?}"` with the offending string
- Called before spawning any external process; callers must not bypass it

**`quote_path_for_cd` Option contract** (`src/app/pty.rs:435`):
- `fn quote_path_for_cd(path_str: &str) -> Option<String>`
- Returns `None` only when `shlex::try_quote` rejects the path (NUL byte only)
- `None` means **skip the cd command** — callers log and skip, never fall back to unescaped path
- Callers (`sync_terminals`, `sync_terminals_initial`) use `match` with an explicit `None` arm that logs via `eprintln!`

**`filter_restart_args` contract** (`src/update/install.rs`):
- `fn filter_restart_args(args: impl Iterator<Item = String>) -> Vec<String>`
- Strips one-shot flags `--update-to` (+ its value), `--check-update`, `--clipboard-diag`, `--ssh-paste-diag`
- Guards against infinite restart loops (IN-02)
- Stateful skip: `skip_next = true` after seeing `--update-to`

## RAII-Over-Explicit-Cleanup Pattern

Established pattern after `src/browser/pdf_export.rs` refactor:

- Temporary files are managed via `tempfile::NamedTempFile` (returned from `default_preview_file`)
- Caller holds the `NamedTempFile` value for the duration the file must exist; file is deleted on `Drop`
- Functions that create temp resources return the RAII guard to the caller — **never** delete manually
- Doc comment on `default_preview_file` explicitly states: *"The returned `NamedTempFile` must be kept alive until the browser has finished"*
- Do **not** use `std::fs::remove_file` for cleanup; use RAII guards instead

## Logging

**Framework:** `eprintln!` for debug/diagnostic messages inside PTY and sync code; dedicated `log_update()` helper in `src/update/log.rs` writes to `/tmp/claude-workbench-update.log`

**Patterns:**
- Update subsystem: all ops logged via `log_update("=== FUNCTION STARTED ===")` pattern with `===` delimiters for section headers
- Sync failures: `eprintln!("sync_terminals: skipping unquotable path: {:?}", path)` — informational, not fatal
- No structured logging crate (no `tracing`, no `log`)

## Comments

**When to Comment:**
- Module-level `//!` doc comments on every module file (e.g., `//! Binary installation and restart logic.`)
- `///` doc comments on public functions, especially those with non-obvious contracts
- Inline `//` for non-obvious logic, safety invariants, and workaround explanations
- Doc comments on `filter_restart_args` explain the IN-02 infinite-loop risk explicitly

**Key doc comment conventions:**
- RAII lifetime requirements stated in doc: *"The returned X must be kept alive until..."*
- Error paths documented: *"Returns `None` only when..."*
- Platform-specific code blocks preceded by comment explaining the reason (e.g., Linux `/proc/self/exe (deleted)` stripping in `src/update/install.rs`)

## Function Design

**Size:** Short, focused functions; complex state machines factored into helper methods

**Parameters:**
- `impl Iterator<Item = T>` preferred over `Vec<T>` for sequence consumers (e.g., `filter_restart_args`)
- `&str` for string inputs, `&Path` for filesystem paths
- Config structs passed by reference: `&Config`, `&StartupOptions`

**Return Values:**
- `Option<String>` for operations that can fail silently and callers must skip (e.g., `quote_path_for_cd`)
- `Result<()>` for operations that must propagate errors (e.g., `validate_program`, `open_file_with_browser`)
- `Vec<String>` for filtered/transformed argument lists

## Feature Flags

**`#[cfg(debug_assertions)]`** used to gate development-only CLI flags:
- `--update-to` flag is `#[cfg(debug_assertions)]` — absent from release builds
- Tests that verify release-only behavior use `#[cfg(not(debug_assertions))]`

## Module Design

**Exports:**
- `pub` for public API crossing module boundaries
- `pub(super)` for items shared within a parent module but not externally
- Private helpers (e.g., `filter_restart_args`, `validate_program`, `quote_path_for_cd`) are `fn` with no visibility modifier

**Module Files:**
- Each module file starts with a `//!` doc comment
- `#[cfg(test)] mod tests { ... }` at the bottom of the file containing the code under test

---

*Convention analysis: 2026-05-11*
