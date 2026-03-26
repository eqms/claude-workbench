//! Native PDF generation via Typst (pure Rust, no external binaries).
//!
//! Pipeline: Markdown → pulldown-cmark → Typst markup → typst compile → PDF
//!
//! Features:
//! - Page numbers ("Seite X von Y") on every page
//! - Configurable header/footer with company branding
//! - A4 format with configurable margins
//! - Table styling with colored headers
//! - Syntax highlighting via Typst built-in raw blocks
//! - Bundled Carlito font (metric-compatible Calibri replacement)

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use anyhow::Result;
use typst::diag::{FileError, FileResult};
use typst::foundations::{Bytes, Datetime};
use typst::syntax::{FileId, Source};
use typst::text::{Font, FontBook};
use typst::utils::LazyHash;
use typst::LibraryExt;

use crate::browser::pdf_export::ExportOptions;
use crate::config::DocumentConfig;

// --- Bundled Carlito font files (SIL Open Font License) ---
static CARLITO_REGULAR: &[u8] = include_bytes!("../../assets/fonts/Carlito-Regular.ttf");
static CARLITO_BOLD: &[u8] = include_bytes!("../../assets/fonts/Carlito-Bold.ttf");
static CARLITO_ITALIC: &[u8] = include_bytes!("../../assets/fonts/Carlito-Italic.ttf");
static CARLITO_BOLD_ITALIC: &[u8] = include_bytes!("../../assets/fonts/Carlito-BoldItalic.ttf");

/// The Typst page template with placeholders for config values.
const TYPST_TEMPLATE: &str = r##"
#set page(
  paper: "{page_size}",
  margin: (left: {margin}, right: {margin}, top: {margin}, bottom: {margin}),
  header: [
    #set text(size: 9pt, fill: rgb("{header_border}"))
    {title}
    #v(2pt)
    #line(length: 100%, stroke: 0.5pt + rgb("{header_border}"))
  ],
  footer: [
    #line(length: 100%, stroke: 0.5pt + rgb("{header_border}"))
    #v(2pt)
    #set text(size: {footer_size}, fill: rgb("{footer_color}"))
    #grid(
      columns: (1fr, 1fr, 1fr),
      align: (left, center, right),
      [{company_name}],
      [{date}],
      [Seite #context counter(page).display() von #context counter(page).final().first()],
    )
  ],
)

#set text(font: ("{font_family}", "Carlito", "Liberation Sans"), size: {body_size}, lang: "de")
#set par(justify: true, leading: 0.65em)
#set heading(numbering: none)

#show heading.where(level: 1): it => [
  #set text(size: {title_size}, weight: "bold")
  #v(0.8em)
  #it
  #v(0.2em)
  #line(length: 100%, stroke: 0.5pt + rgb("#cccccc"))
  #v(0.3em)
]

#show heading.where(level: 2): it => [
  #set text(size: {h1_size}, weight: "bold")
  #v(0.6em)
  #it
  #v(0.2em)
]

#show heading.where(level: 3): it => [
  #set text(size: {h2_size}, weight: "bold")
  #v(0.5em)
  #it
  #v(0.2em)
]

#show raw.where(block: true): it => [
  #set text(size: {table_size})
  #block(
    fill: rgb("#f4f4f4"),
    inset: 10pt,
    radius: 4pt,
    width: 100%,
    it,
  )
]

{body}
"##;

// --- Typst World Implementation ---

/// Minimal Typst World for self-contained document rendering.
struct WorkbenchWorld {
    source: Source,
    source_dir: PathBuf,
    library: LazyHash<typst::Library>,
    book: LazyHash<FontBook>,
    fonts: Vec<FontSlot>,
}

/// Font slot with lazy loading (for system fonts) or pre-loaded (for bundled fonts).
struct FontSlot {
    path: Option<PathBuf>,
    index: u32,
    font: OnceLock<Option<Font>>,
}

impl FontSlot {
    fn get(&self) -> Option<Font> {
        self.font
            .get_or_init(|| {
                let path = self.path.as_ref()?;
                let data = std::fs::read(path).ok()?;
                Font::new(Bytes::new(data), self.index)
            })
            .clone()
    }
}

impl typst::World for WorkbenchWorld {
    fn library(&self) -> &LazyHash<typst::Library> {
        &self.library
    }

    fn book(&self) -> &LazyHash<FontBook> {
        &self.book
    }

    fn main(&self) -> FileId {
        self.source.id()
    }

    fn source(&self, id: FileId) -> FileResult<Source> {
        if id == self.source.id() {
            Ok(self.source.clone())
        } else {
            Err(FileError::NotFound(PathBuf::from("<not-available>")))
        }
    }

    fn file(&self, id: FileId) -> FileResult<Bytes> {
        let vpath = id.vpath();
        let path = vpath.as_rooted_path();
        let rel = path.strip_prefix("/").unwrap_or(path);
        let full_path = self.source_dir.join(rel);
        match std::fs::read(&full_path) {
            Ok(data) => Ok(Bytes::new(data)),
            Err(_) => Err(FileError::NotFound(full_path)),
        }
    }

    fn font(&self, index: usize) -> Option<Font> {
        self.fonts.get(index)?.get()
    }

    fn today(&self, _offset: Option<i64>) -> Option<Datetime> {
        // Use libc for local time (already a dependency)
        let now = unsafe {
            let mut t: libc::time_t = 0;
            libc::time(&mut t);
            let mut tm: libc::tm = std::mem::zeroed();
            #[cfg(unix)]
            libc::localtime_r(&t, &mut tm);
            #[cfg(windows)]
            libc::localtime_s(&mut tm, &t);
            tm
        };
        Datetime::from_ymd(
            (now.tm_year + 1900) as i32,
            (now.tm_mon + 1) as u8,
            now.tm_mday as u8,
        )
    }
}

/// Build font book and font slots from bundled + system fonts.
fn build_fonts(_doc: &DocumentConfig) -> (LazyHash<FontBook>, Vec<FontSlot>) {
    let mut book = FontBook::new();
    let mut fonts = Vec::new();

    // 1. Load bundled Carlito fonts (highest priority for fallback)
    let bundled: &[&[u8]] = &[
        CARLITO_REGULAR,
        CARLITO_BOLD,
        CARLITO_ITALIC,
        CARLITO_BOLD_ITALIC,
    ];
    for font_data in bundled {
        let buffer = Bytes::new(font_data.to_vec());
        for (i, font) in Font::iter(buffer.clone()).enumerate() {
            book.push(font.info().clone());
            fonts.push(FontSlot {
                path: None,
                index: i as u32,
                font: OnceLock::from(Some(font)),
            });
        }
    }

    // 2. Scan system fonts via typst-kit FontSearcher
    let system_fonts = typst_kit::fonts::FontSearcher::new().search();
    // Transfer font info from system_fonts.book to our book,
    // and create slots for each system font
    for (info_idx, slot) in system_fonts.fonts.into_iter().enumerate() {
        // Get FontInfo from the searcher's book
        if let Some(info) = system_fonts.book.info(info_idx) {
            book.push(info.clone());
            if let Some(font) = slot.get() {
                fonts.push(FontSlot {
                    path: slot.path().map(|p| p.to_path_buf()),
                    index: slot.index(),
                    font: OnceLock::from(Some(font)),
                });
            } else if let Some(path) = slot.path() {
                fonts.push(FontSlot {
                    path: Some(path.to_path_buf()),
                    index: slot.index(),
                    font: OnceLock::new(),
                });
            }
        }
    }

    (LazyHash::new(book), fonts)
}

// --- Markdown to Typst Renderer ---

/// Renders Markdown to Typst markup using pulldown-cmark events.
struct TypstRenderer {
    out: String,
    in_code_block: bool,
    code_lang: Option<String>,
    code_buf: String,
    in_table: bool,
    table_header_done: bool,
    table_columns: usize,
    table_cells: Vec<String>,
    cell_buf: String,
    list_depth: u32,
    ordered_list_stack: Vec<bool>,
    in_heading: bool,
    table_header_bg: String,
    table_border: String,
    table_size: String,
}

impl TypstRenderer {
    fn new(doc: &DocumentConfig) -> Self {
        Self {
            out: String::new(),
            in_code_block: false,
            code_lang: None,
            code_buf: String::new(),
            in_table: false,
            table_header_done: false,
            table_columns: 0,
            table_cells: Vec::new(),
            cell_buf: String::new(),
            list_depth: 0,
            ordered_list_stack: Vec::new(),
            in_heading: false,
            table_header_bg: doc.colors.table_header_bg.clone(),
            table_border: doc.colors.table_border.clone(),
            table_size: doc.sizes.table.clone(),
        }
    }

    /// Render Markdown string to Typst markup.
    fn render(md: &str, doc: &DocumentConfig) -> String {
        use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};

        let opts =
            Options::ENABLE_TABLES | Options::ENABLE_STRIKETHROUGH | Options::ENABLE_TASKLISTS;
        let parser = Parser::new_ext(md, opts);
        let mut r = TypstRenderer::new(doc);

        for event in parser {
            match event {
                // --- Headings ---
                Event::Start(Tag::Heading { level, .. }) => {
                    r.in_heading = true;
                    let prefix = "=".repeat(level as usize);
                    r.out.push_str(&format!("\n{} ", prefix));
                }
                Event::End(TagEnd::Heading(_)) => {
                    r.in_heading = false;
                    r.out.push('\n');
                }

                // --- Emphasis/Strong ---
                Event::Start(Tag::Emphasis) => r.push_to_active("_"),
                Event::End(TagEnd::Emphasis) => r.push_to_active("_"),
                Event::Start(Tag::Strong) => r.push_to_active("*"),
                Event::End(TagEnd::Strong) => r.push_to_active("*"),
                Event::Start(Tag::Strikethrough) => r.push_to_active("#strike["),
                Event::End(TagEnd::Strikethrough) => r.push_to_active("]"),

                // --- Paragraphs ---
                Event::Start(Tag::Paragraph) => {
                    if !r.in_table && !r.out.is_empty() && !r.out.ends_with('\n') {
                        r.out.push('\n');
                    }
                }
                Event::End(TagEnd::Paragraph) => {
                    if !r.in_table {
                        r.out.push('\n');
                    }
                }

                // --- Code blocks ---
                Event::Start(Tag::CodeBlock(kind)) => {
                    r.in_code_block = true;
                    r.code_buf.clear();
                    r.code_lang = match kind {
                        pulldown_cmark::CodeBlockKind::Fenced(lang) => {
                            let lang = lang.to_string();
                            if lang.is_empty() {
                                None
                            } else {
                                Some(lang)
                            }
                        }
                        pulldown_cmark::CodeBlockKind::Indented => None,
                    };
                }
                Event::End(TagEnd::CodeBlock) => {
                    r.in_code_block = false;
                    let lang = r.code_lang.take();
                    let code = r.code_buf.clone();
                    let code = code.trim_end_matches('\n');
                    if let Some(lang) = lang {
                        r.out.push_str(&format!("\n```{}\n{}\n```\n", lang, code));
                    } else {
                        r.out.push_str(&format!("\n```\n{}\n```\n", code));
                    }
                }

                // --- Inline code ---
                Event::Code(text) => {
                    r.push_to_active(&format!("`{}`", text));
                }

                // --- Links ---
                Event::Start(Tag::Link { dest_url, .. }) => {
                    r.push_to_active(&format!("#link(\"{}\")[", dest_url));
                }
                Event::End(TagEnd::Link) => {
                    r.push_to_active("]");
                }

                // --- Images ---
                Event::Start(Tag::Image { dest_url, .. }) => {
                    let url = dest_url.to_string();
                    if url.starts_with("http://") || url.starts_with("https://") {
                        // Remote images cannot be loaded by Typst — render as link
                        r.out.push_str(&format!(
                            "#text(fill: rgb(\"#666666\"), size: 9pt)[Image: #link(\"{}\")[{}]]",
                            url, url
                        ));
                    } else {
                        r.out.push_str(&format!("#image(\"{}\")", url));
                    }
                }
                Event::End(TagEnd::Image) => {}

                // --- Lists ---
                Event::Start(Tag::List(first)) => {
                    r.list_depth += 1;
                    r.ordered_list_stack.push(first.is_some());
                }
                Event::End(TagEnd::List(_)) => {
                    r.list_depth = r.list_depth.saturating_sub(1);
                    r.ordered_list_stack.pop();
                    if r.list_depth == 0 {
                        r.out.push('\n');
                    }
                }
                Event::Start(Tag::Item) => {
                    let indent = "  ".repeat((r.list_depth - 1) as usize);
                    let is_ordered = r.ordered_list_stack.last().copied().unwrap_or(false);
                    if is_ordered {
                        r.out.push_str(&format!("{}+ ", indent));
                    } else {
                        r.out.push_str(&format!("{}- ", indent));
                    }
                }
                Event::End(TagEnd::Item) => {
                    if !r.out.ends_with('\n') {
                        r.out.push('\n');
                    }
                }

                // --- Tables ---
                Event::Start(Tag::Table(alignments)) => {
                    r.in_table = true;
                    r.table_columns = alignments.len();
                    r.table_cells.clear();
                    r.table_header_done = false;
                    r.out.push_str(&format!(
                        "\n#set text(size: {})\n#table(\n  columns: {},\n  fill: (_, row) => if row == 0 {{ rgb(\"{}\") }} else {{ white }},\n  stroke: rgb(\"{}\"),\n  inset: 8pt,\n",
                        r.table_size, r.table_columns, r.table_header_bg, r.table_border,
                    ));
                }
                Event::End(TagEnd::Table) => {
                    r.in_table = false;
                    r.out.push_str(")\n\n");
                }
                Event::Start(Tag::TableHead) => {
                    r.table_cells.clear();
                    r.out.push_str("  table.header(\n");
                }
                Event::End(TagEnd::TableHead) => {
                    for cell in &r.table_cells {
                        r.out.push_str(&format!("    [*{}*],\n", cell));
                    }
                    r.table_cells.clear();
                    r.out.push_str("  ),\n");
                    r.table_header_done = true;
                }
                Event::Start(Tag::TableRow) => {
                    r.table_cells.clear();
                }
                Event::End(TagEnd::TableRow) => {
                    if r.table_header_done {
                        for cell in &r.table_cells {
                            r.out.push_str(&format!("  [{}],\n", cell));
                        }
                        r.table_cells.clear();
                    }
                }
                Event::Start(Tag::TableCell) => {
                    r.cell_buf.clear();
                }
                Event::End(TagEnd::TableCell) => {
                    r.table_cells.push(r.cell_buf.clone());
                    r.cell_buf.clear();
                }

                // --- Block quotes ---
                Event::Start(Tag::BlockQuote) => {
                    r.out.push_str("\n#quote(block: true)[\n");
                }
                Event::End(TagEnd::BlockQuote) => {
                    r.out.push_str("]\n");
                }

                // --- Horizontal rule ---
                Event::Rule => {
                    r.out.push_str("\n#line(length: 100%)\n");
                }

                // --- Text ---
                Event::Text(text) => {
                    if r.in_code_block {
                        r.code_buf.push_str(&text);
                    } else if r.in_table {
                        r.cell_buf.push_str(&typst_escape(&text));
                    } else {
                        r.push_to_active(&typst_escape(&text));
                    }
                }

                // --- Soft/Hard breaks ---
                Event::SoftBreak => {
                    if r.in_code_block {
                        r.code_buf.push('\n');
                    } else if r.in_table {
                        r.cell_buf.push(' ');
                    } else {
                        r.out.push('\n');
                    }
                }
                Event::HardBreak => {
                    if r.in_table {
                        r.cell_buf.push_str(" \\ ");
                    } else {
                        r.out.push_str(" \\\n");
                    }
                }

                // --- HTML (skip in Typst output) ---
                Event::Html(_) | Event::InlineHtml(_) => {}

                // --- Task list markers ---
                Event::TaskListMarker(checked) => {
                    if checked {
                        r.out.push_str("[x] ");
                    } else {
                        r.out.push_str("[ ] ");
                    }
                }

                // --- Footnotes ---
                Event::Start(Tag::FootnoteDefinition(_)) => {}
                Event::End(TagEnd::FootnoteDefinition) => {}
                Event::FootnoteReference(_) => {}

                // --- Metadata ---
                Event::Start(Tag::MetadataBlock(_)) => {}
                Event::End(TagEnd::MetadataBlock(_)) => {}

                // --- HTML blocks ---
                Event::Start(Tag::HtmlBlock) | Event::End(TagEnd::HtmlBlock) => {}
            }
        }

        r.out
    }

    /// Push text to the appropriate buffer (cell_buf if in table, out otherwise)
    fn push_to_active(&mut self, text: &str) {
        if self.in_table {
            self.cell_buf.push_str(text);
        } else {
            self.out.push_str(text);
        }
    }
}

/// Escape special Typst characters in literal text.
fn typst_escape(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '\\' => result.push_str("\\\\"),
            '#' => result.push_str("\\#"),
            '@' => result.push_str("\\@"),
            '<' => result.push_str("\\<"),
            '>' => result.push_str("\\>"),
            _ => result.push(c),
        }
    }
    result
}

/// Build the complete Typst document from template + body.
fn build_typst_document(body: &str, options: &ExportOptions, doc: &DocumentConfig) -> String {
    // Extract font family name (first entry before comma for Typst)
    let font_family = doc
        .fonts
        .body
        .split(',')
        .next()
        .unwrap_or("Calibri")
        .trim()
        .trim_matches('\'')
        .trim_matches('"');

    TYPST_TEMPLATE
        .replace("{page_size}", &doc.pdf.page_size.to_lowercase())
        .replace("{margin}", &doc.pdf.margin)
        .replace("{header_border}", &doc.colors.header_border)
        .replace("{footer_color}", &doc.colors.footer)
        .replace("{footer_size}", &doc.sizes.footer)
        .replace("{company_name}", &typst_escape(&doc.resolved_footer_text()))
        .replace("{date}", &options.date)
        .replace("{title}", &typst_escape(&options.title))
        .replace("{font_family}", font_family)
        .replace("{body_size}", &doc.sizes.body)
        .replace("{title_size}", &doc.sizes.title)
        .replace("{h1_size}", &doc.sizes.h1)
        .replace("{h2_size}", &doc.sizes.h2)
        .replace("{h3_size}", &doc.sizes.h3)
        .replace("{table_size}", &doc.sizes.table)
        .replace("{body}", body)
}

/// Export a Markdown file to PDF using native Typst rendering.
///
/// This is a pure-Rust pipeline with no external binary dependencies.
/// Bundled Carlito font provides Calibri-compatible rendering on all platforms.
pub fn export_markdown_to_pdf(
    md_source: &Path,
    target: &Path,
    options: &ExportOptions,
    doc: &DocumentConfig,
) -> Result<PathBuf> {
    // 1. Read markdown source
    let md = std::fs::read_to_string(md_source)?;

    // 2. Convert Markdown to Typst markup
    let body = TypstRenderer::render(&md, doc);

    // 3. Build complete Typst document
    let typ_source = build_typst_document(&body, options, doc);

    // 4. Build font resolver
    let (book, fonts) = build_fonts(doc);

    // 5. Create Typst source and world
    let source = Source::detached(&typ_source);
    let library = LazyHash::new(typst::Library::default());
    let source_dir = md_source
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."));
    let world = WorkbenchWorld {
        source,
        source_dir,
        library,
        book,
        fonts,
    };

    // 6. Compile the document
    let result = typst::compile::<typst::layout::PagedDocument>(&world);
    let document = result.output.map_err(|errors| {
        let messages: Vec<String> = errors
            .iter()
            .map(|e| format!("{:?}: {}", e.severity, e.message))
            .collect();
        anyhow::anyhow!("Typst PDF generation failed:\n{}", messages.join("\n"))
    })?;

    // 7. Export to PDF
    let pdf_options = typst_pdf::PdfOptions::default();
    let pdf_bytes = typst_pdf::pdf(&document, &pdf_options).map_err(|errors| {
        let messages: Vec<String> = errors
            .iter()
            .map(|e| format!("{:?}: {}", e.severity, e.message))
            .collect();
        anyhow::anyhow!("PDF export failed:\n{}", messages.join("\n"))
    })?;

    // 8. Write output file
    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(target, pdf_bytes.as_slice())?;

    Ok(target.to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_typst_escape() {
        assert_eq!(typst_escape("hello"), "hello");
        assert_eq!(typst_escape("#heading"), "\\#heading");
        assert_eq!(typst_escape("a@b"), "a\\@b");
        assert_eq!(typst_escape("a<b>c"), "a\\<b\\>c");
        assert_eq!(typst_escape("ä ö ü ß"), "ä ö ü ß");
    }

    #[test]
    fn test_markdown_to_typst_heading() {
        let doc = DocumentConfig::default();
        let result = TypstRenderer::render("# Hello World", &doc);
        assert!(result.contains("= Hello World"));
    }

    #[test]
    fn test_markdown_to_typst_bold_italic() {
        let doc = DocumentConfig::default();
        let result = TypstRenderer::render("**bold** and _italic_", &doc);
        assert!(result.contains("*bold*"));
        assert!(result.contains("_italic_"));
    }

    #[test]
    fn test_markdown_to_typst_code_block() {
        let doc = DocumentConfig::default();
        let result = TypstRenderer::render("```rust\nfn main() {}\n```", &doc);
        assert!(result.contains("```rust"));
        assert!(result.contains("fn main() {}"));
    }

    #[test]
    fn test_markdown_to_typst_list() {
        let doc = DocumentConfig::default();
        let result = TypstRenderer::render("- item 1\n- item 2\n", &doc);
        assert!(result.contains("- item 1"));
        assert!(result.contains("- item 2"));
    }

    #[test]
    fn test_markdown_to_typst_ordered_list() {
        let doc = DocumentConfig::default();
        let result = TypstRenderer::render("1. first\n2. second\n", &doc);
        assert!(result.contains("+ first"));
        assert!(result.contains("+ second"));
    }

    #[test]
    fn test_markdown_to_typst_link() {
        let doc = DocumentConfig::default();
        let result = TypstRenderer::render("[click here](https://example.com)", &doc);
        assert!(result.contains("#link(\"https://example.com\")[click here]"));
    }

    #[test]
    fn test_markdown_to_typst_table() {
        let doc = DocumentConfig::default();
        let result = TypstRenderer::render("| A | B |\n|---|---|\n| 1 | 2 |\n", &doc);
        assert!(result.contains("#table("));
        assert!(result.contains("table.header("));
        assert!(result.contains("[*A*]"));
        assert!(result.contains("[1]"));
    }

    #[test]
    fn test_markdown_to_typst_german_umlauts() {
        let doc = DocumentConfig::default();
        let result = TypstRenderer::render("Ärger mit Übung und größer", &doc);
        assert!(result.contains("Ärger"));
        assert!(result.contains("Übung"));
        assert!(result.contains("größer"));
    }

    #[test]
    fn test_build_typst_document() {
        let doc = DocumentConfig::default();
        let options = ExportOptions {
            title: "Test Document".to_string(),
            author: "Test Author".to_string(),
            date: "26.03.2026".to_string(),
            format: crate::browser::pdf_export::ExportFormat::Pdf,
        };
        let result = build_typst_document("Hello world", &options, &doc);
        assert!(result.contains("paper: \"a4\""));
        assert!(result.contains("Test Document"));
        assert!(result.contains("26.03.2026"));
        assert!(result.contains("Hello world"));
        assert!(result.contains("Carlito"));
    }
}
