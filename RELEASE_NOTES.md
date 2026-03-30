# Release Notes

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
