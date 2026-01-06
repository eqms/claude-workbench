# Claude Workbench - Implementation Plan (Phase 1)

## Goal Description
Initialize the **Claude Workbench** project, a Rust-based TUI application.
This phase covers the project skeleton, dependency setup (`Cargo.toml`), and the core event loop with Ratatui integration.

## Proposed Changes

### Configuration
#### [NEW] [Cargo.toml](file:///Users/picard/.gemini/antigravity/playground/shimmering-interstellar/Cargo.toml)
- Define workspace package `claude-workbench`.
- Add dependencies:
  - `ratatui` (TUI framework)
  - `crossterm` (Terminal manipulation)
  - `tokio` (Async runtime)
  - `anyhow` (Error handling)
  - `serde`, `serde_yaml` (Config/Session)
  - `directories` (Path resolution)
  - `tui-textarea` (Input - optional for now but good to have)
  - `portable-pty` (Future use, add now or wait? Spec says Phase 1 is basic. I will verify if I need it immediately. The user asked for "concrete Cargo.toml with all suggested dependencies". So I will include it.)

### Source Code Structure
#### [NEW] [src/main.rs](file:///Users/picard/.gemini/antigravity/playground/shimmering-interstellar/src/main.rs)
- CLI argument parsing (clap).
- Terminal setup (raw mode, automated restoration logic).
- Initialization of `App`.
- Main event loop (polled).

#### [NEW] [src/app.rs](file:///Users/picard/.gemini/antigravity/playground/shimmering-interstellar/src/app.rs)
- `App` struct definition.
- `AppState` management.
- `run` method containing the logic to handle events and trigger drawing.

#### [NEW] [src/ui/mod.rs](file:///Users/picard/.gemini/antigravity/playground/shimmering-interstellar/src/ui/mod.rs)
- Module definitions for `layout`, `file_browser`, `preview`, `terminal_pane`.

#### [NEW] [src/config.rs](file:///Users/picard/.gemini/antigravity/playground/shimmering-interstellar/src/config.rs)
- Basic `Config` struct and loading logic.

## Verification Plan

### Automated Tests
- Run `cargo check` to ensure dependencies and modules resolve.
- Run `cargo build` to produce the binary.

### Manual Verification
- Run `./target/debug/claude-workbench`
- Verify TUI launches, renders 3 main areas (Left, Middle, Right), and exits cleanly on `q` or `Ctrl+C`.
