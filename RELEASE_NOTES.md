# Release Notes

## Version 0.80.0 (21.04.2026)

### Added
- **`pdf-export` Cargo Feature (default-enabled)** — The typst-based PDF pipeline
  (`typst`, `typst-pdf`, `typst-library`, `typst-kit`, `comemo`, `ecow`) is now gated
  behind the `pdf-export` feature. `cargo build --no-default-features` produces a
  smaller binary without the PDF toolchain, while `cargo build` keeps the default
  behavior. Callers of `export_markdown()` receive a descriptive error when PDF
  export is requested in a build without the feature.
- **CLI integration tests (`tests/cli.rs`)** — First integration-test suite verifies
  `--help` shows usage including `--check-update`, `--version` prints the Cargo
  package version, and unknown flags are rejected. Runs alongside the 78 unit tests
  via `cargo test`.
- **`rustfmt.toml` and `clippy.toml`** — Style/lint contract is now explicit:
  edition 2021, Unix newlines, reordered imports/modules, cognitive-complexity
  threshold 30, too-many-arguments threshold 8.
- **Multi-OS test coverage in CI** — `ci.yml` test job now matrixes over
  `ubuntu-latest`, `macos-latest`, `windows-latest` (previously only Linux),
  matching the release-build target platforms.

### Changed
- **`src/app/keyboard.rs` split into 15 focused methods** — `handle_key_event`
  shrunk from 1,375 lines (the single function body) to a ~180-line orchestrator
  that dispatches to dedicated handlers per overlay and pane: `handle_fuzzy_finder_key`,
  `handle_update_dialog_key`, `handle_export_chooser_key`, `handle_active_dialog_key`,
  `handle_menu_key`, `handle_about_key`, `handle_help_key`,
  `handle_permission_mode_dialog_key`, `handle_claude_startup_key`,
  `handle_global_shortcut`, `handle_pane_resize_key`,
  `handle_file_browser_pane_key`, `handle_preview_pane_key`,
  `handle_preview_edit_key`, `handle_preview_readonly_key`,
  `handle_terminal_pane_key`. Behavior is preserved 1:1.
- **`src/update/mod.rs` split into submodules** — The 986-line update module is
  now composed of six focused files: `log.rs` (file-based update log),
  `state.rs` (`UpdateState`, `UpdateCheckResult`, `UpdateResult`),
  `version.rs` (`version_newer`, `CURRENT_VERSION`),
  `release_notes.rs` (fetch + platform filtering),
  `check.rs` (sync/async update checks, `get_target`),
  `install.rs` (`perform_update_*`, `restart_application`). All public items are
  re-exported from `update/` for source-compatibility.
- **Regex `unwrap()` audit** — The 49 `Regex::new(…).unwrap()` sites in
  `src/filter.rs` are now `.expect("static regex pattern must compile")`, so any
  future pattern-compilation failure emits a descriptive panic instead of a bare
  `called Option::unwrap() on None`.
- **Single-file modules flattened** — `src/input/mod.rs` → `src/input.rs` and
  `src/session/mod.rs` → `src/session.rs`. Module paths and imports are unchanged.

### Internal
- This is a pure refactor/infrastructure release — no user-facing behavior
  changes beyond the optional `--no-default-features` build. Existing
  shortcuts, dialogs, and workflows are untouched.

## Version 0.79.0 (30.03.2026)

### Added
- **HTML Export Cross-File Link Resolution** — When a Markdown file references other `.md`
  files (e.g., `[Usage](USAGE.md)`), the HTML export now automatically converts all
  referenced Markdown files to HTML and rewrites the links to point to the generated
  HTML files. This ensures link integrity when previewing documentation in the browser.
- 6 new unit tests for `collect_md_links()` and `fix_md_links()`: simple links, fragment
  preservation, absolute URL filtering, dot-slash normalization, unknown link passthrough.

### Changed
- **`markdown_to_html()` signature** — Now returns `Vec<PathBuf>` instead of `PathBuf`,
  containing the primary HTML file and all linked dependency files for proper cleanup tracking.
- **Internal refactoring** — Extracted `convert_single_md()` helper for single-file conversion
  without disk I/O, enabling the two-phase convert-then-rewrite approach.

### Security
- Path traversal guard on linked `.md` files — resolved paths must remain under the source
  file's directory (same guard as used for image paths).

## Version 0.78.0 (30.03.2026)

### Fixed
- **Internal Anchor Links in HTML Export** — Clicking `[text](#section)` links in the
  browser preview (`o` key) and HTML export now correctly jumps to the target heading.
  Root cause: `pulldown-cmark` does not auto-generate `id` attributes on headings.
  New `inject_heading_ids()` function walks the event stream, collects heading text,
  slugifies it, and injects `id="slug"` into each heading tag.

### Changed
- **Shared `slugify()` function** — The heading-to-slug conversion (used for anchor links)
  is now shared between the HTML exporter (`markdown.rs`) and the Typst/PDF exporter
  (`typst_pdf.rs`) via `browser::slugify()`, ensuring consistent behavior across both
  export paths.

### Added
- 6 new unit tests for heading ID injection: basic heading, spaces, special characters,
  anchor link resolution, multiple heading levels, and inline code in headings.

## Version 0.77.0 (28.03.2026)

### Added
- **Unified Export/Preview System** — All export paths (PDF, HTML Markdown preview, Syntax
  preview) now share the same configurable values. 14 new config fields replace hardcoded values
  across `template.rs`, `typst_pdf.rs`, and `syntax.rs`.
- **7 New Document Settings (F8)** — Table Font Size, Header Font Size, Line Height,
  Code Block BG color, Heading Separator color, Table Cell Padding, and Blockquote Border
  are now editable in the Document Settings dialog.
- **Consistent Preview Filenames** — Browser previews (`o` key) now use the same naming
  convention as PDF export: `{project}-{filename}-{dd.mm.yyyy}.html` instead of random
  `cwb-preview-XXXXXXXX.html` / `cwb-syntax-XXXXXXXX.html` temp names.

### Changed
- **TemplateContext Unification** — `syntax.rs` (code file preview) now uses `TemplateContext`
  for footer styling and config-driven font sizes/line heights, consistent with `markdown.rs`.
- **Heading Separator Consistency** — H1 separator line now uses the same color (`#cccccc`)
  in both HTML preview and PDF export. Previously inconsistent (`#eee` vs `#cccccc`).

### Fixed
- **Pre-code font-size** — `pre code` blocks now correctly inherit the configured
  `code_font_size` instead of using a hardcoded value.
