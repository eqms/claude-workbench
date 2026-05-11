# Coding Conventions

**Analysis Date:** 2026-05-11

## Naming Patterns

**Files:**
- `snake_case` for all module files: `job_state.rs`, `file_browser.rs`, `terminal_pane.rs`
- Descriptive compound names preferred over abbreviations: `dependency_checker.rs`, `release_notes.rs`
- Module groups use `mod.rs` as entry point: `src/app/mod.rs`, `src/git/mod.rs`, `src/update/mod.rs`

**Types (structs, enums):**
- `CamelCase` universally: `PseudoTerminal`, `ClipboardOutcome`, `GitFileStatus`, `JobState<T>`
- Enum variants: `CamelCase` — `ClipboardOutcome::Arboard`, `PollOutcome::Ready(T)`, `GitFileStatus::Untracked`
- Generic parameters: single uppercase letter `T`

**Functions and methods:**
- `snake_case` universally: `filter_lines()`, `find_repo_root()`, `detect_strategy()`
- Boolean predicates prefixed `is_` or `has_`: `is_running()`, `is_yolo()`, `has_meaningful_selection()`
- Constructors use `new()` or descriptive names: `UpdateState::new()`, `JobState::running(rx)`
- Helper functions that return static `&str`: `name()`, `label()`, `symbol()`

**Variables:**
- `snake_case` universally
- `_` prefix for intentionally unused: `_path` in `set_restrictive_permissions(_path: &Path)`
- Descriptive over short: `consecutive_blanks`, `in_traceback`, not `n` or `flag`

**Constants:**
- `SCREAMING_SNAKE_CASE`: `SUBPROCESS_TIMEOUT`, `WAIT_POLL_INTERVAL`, `STRATEGY_ENV`, `CURRENT_VERSION`
- `pub(crate)` constants for cross-module sharing: `REPO_OWNER`, `REPO_NAME`, `BIN_NAME` in `src/update/mod.rs`

**Modules:**
- `snake_case` directory/file names
- Public API re-exported at module root via `pub use`: `src/update/mod.rs` re-exports all submodule items

## Code Style

**Formatting:**
- Tool: `rustfmt` with `rustfmt.toml`
- `edition = "2021"`
- `reorder_imports = true`
- `reorder_modules = true`
- `newline_style = "Unix"`
- No explicit line-length override — Rust edition 2021 default (100 chars)

**Linting:**
- Tool: `clippy` with `clippy.toml`
- `cognitive-complexity-threshold = 30` (relaxed from default 25)
- `too-many-arguments-threshold = 8` (relaxed from default 7)
- `type-complexity-threshold = 300` (relaxed from default 250)
- `#[allow(clippy::type_complexity)]` used sparingly for `MouseSelection::finish()` return type

## Import Organization

**Order** (enforced by `reorder_imports = true`):
1. Standard library (`std::`)
2. External crates (`anyhow`, `serde`, `ratatui`, etc.)
3. Internal crate imports (`crate::types`, `crate::config`, `super::*`)

**Pattern:**
```rust
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

use crate::types::{ClaudeEffort, ClaudeModel, ClaudePermissionMode};
```

**Platform-specific imports:**
```rust
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
```

**No wildcard imports** except in `#[cfg(test)] mod tests { use super::*; }`.

## Error Handling

**Primary pattern:** `anyhow::Result<T>` for all fallible public functions.

```rust
use anyhow::Result;

pub fn load_config() -> Result<Config> {
    let contents = fs::read_to_string(local_config)?;   // ? propagation
    let config: Config = serde_yaml_ng::from_str(&contents)?;
    Ok(config)
}
```

**Internal/infallible helpers:** Return `Option<T>` — no `anyhow` involved:
```rust
pub fn find_repo_root(path: &Path) -> Option<PathBuf> {
    Command::new("git").output().ok()?;  // .ok()? to convert Result→Option
}
```

**Static regex / must-compile patterns:** `.expect("static regex pattern must compile")` — acceptable for compile-time guaranteed patterns in `LazyLock` statics (`src/filter.rs`).

**No custom error types** — `anyhow::Error` used throughout; no `thiserror` dependency.

**`unwrap()` policy:** Avoided in production code. `expect()` used only where a panic would be a programmer bug (static regex, impossible states). `.ok()` used to silently ignore errors in optional operations (git CLI calls).

**OS command errors:** Swallowed via `.ok()` / `if let Ok(output) = ...` — git/subprocess failures are non-fatal and treated as "feature unavailable":
```rust
let output = Command::new("git").output();
if let Ok(output) = output {
    if output.status.success() { /* parse */ }
}
```

## Logging

**Framework:** None — no `log`, `tracing`, or `env_logger` crate.

**Patterns:**
- `println!` used only in CLI diagnostic modes (`--check-update`, `--clipboard-diag`) in `src/main.rs`
- TUI operation: no stdout/stderr logging during normal operation (would corrupt terminal rendering)
- Update operations write to `/tmp/claude-workbench-update.log` via `src/update/log.rs`
- Errors that cannot be surfaced to the user are silently dropped (by design — TUI constraint)

## Comments

**Module-level doc comments:** `//!` style at top of every non-trivial module:
```rust
//! Clipboard utility with multi-stage fallback chain.
//!
//! Stage order (copy):
//!   1. arboard  — native display server
//!   2. xclip    — X11 selection bridge
```

**Item-level doc comments:** `///` for all public types, methods, and functions:
```rust
/// Non-blocking poll. Returns the appropriate `PollOutcome` and
/// transitions the state to `Idle` whenever a terminal outcome is observed.
pub fn poll(&mut self) -> PollOutcome<T> {
```

**Inline comments:** `//` for non-obvious implementation decisions, especially platform quirks and non-obvious defaults:
```rust
// Empty = use terminal shell (Fish/Bash), user starts claude manually
claude_command: vec![],
```

**Section separators:** `// ─────────` (Unicode box-drawing) used to visually group methods within large impls (`src/types.rs` `SearchState`).

**German in UI strings:** `description_de()` methods return German user-facing strings. All code, comments, and doc comments are in English.

## Function Design

**Size:** Functions tend to be medium-sized (20–80 lines). The clippy threshold of 30 cognitive complexity is the practical limit.

**Parameters:** Maximum 8 (per clippy config). Prefer passing structs for related data over long parameter lists. State structs (`HelpState`, `SearchState`, `DragState`) group related fields and their methods together.

**Return values:**
- Fallible operations: `Result<T>` (anyhow) or `Option<T>`
- Infallible state mutation: `()` (methods on state structs)
- Boolean queries: `bool`

**State mutation pattern:** Structs own their state and expose named methods:
```rust
impl HelpState {
    pub fn open(&mut self) { ... }
    pub fn close(&mut self) { ... }
    pub fn scroll_up(&mut self, amount: usize) { ... }
}
```

## Module Design

**Exports:**
- Public API declared with `pub` at item level
- Crate-internal sharing uses `pub(crate)`
- Re-export at module root via `pub use submodule::Item` for clean external API

**Barrel files:** `mod.rs` used as module roots for multi-file modules (`app/`, `browser/`, `setup/`, `update/`, `ui/`, `git/`). Re-exports flatten internal structure.

**Feature gating:** Optional features use `#[cfg(feature = "pdf-export")]` with `optional = true` in `Cargo.toml`. Feature-gated code in `src/browser/typst_pdf.rs` and `src/browser/pdf_export.rs`.

**Platform gating:** `#[cfg(unix)]`, `#[cfg(windows)]`, `#[cfg(target_os = "macos")]`, `#[cfg(target_os = "linux")]` used for platform-specific paths in `src/config.rs`, `src/app_detector.rs`, `src/clipboard.rs`.

**Serde defaults pattern** (prominent in `src/config.rs`): Each optional field uses a dedicated `fn default_*() -> T` function referenced via `#[serde(default = "default_*")]`. This avoids `Option` wrapping while keeping YAML backward-compatible.

## Concurrency Patterns

**Threading model:** Background threads communicate via `std::sync::mpsc` channels. `Arc<Mutex<vt100::Parser>>` shared between PTY reader thread and main UI thread.

**`JobState<T>` pattern** (`src/app/job_state.rs`): Typed wrapper around `Receiver<T>` that makes async job lifecycle explicit (`Idle` / `Running`). Use this instead of raw `Option<Receiver<T>>`:
```rust
pub enum JobState<T> { Idle, Running(Receiver<T>) }
pub enum PollOutcome<T> { Pending, Ready(T), Disconnected }
```

**`OnceLock` for process-lifetime caching** (`src/clipboard.rs`): Environment-based detection computed once:
```rust
static STRATEGY: OnceLock<ClipboardStrategy> = OnceLock::new();
fn strategy() -> ClipboardStrategy { *STRATEGY.get_or_init(detect_strategy) }
```

**`LazyLock` for static regex** (`src/filter.rs`): All compiled regexes stored in `LazyLock<Vec<Regex>>`:
```rust
static PROMPT_PATTERNS: LazyLock<Vec<Regex>> = LazyLock::new(|| vec![
    Regex::new(r"...").expect("static regex pattern must compile"),
]);
```

---

*Convention analysis: 2026-05-11*
