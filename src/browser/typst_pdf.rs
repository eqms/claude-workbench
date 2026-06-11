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
use crate::browser::slugify;
use crate::config::DocumentConfig;

// --- Bundled Carlito font files (SIL Open Font License) ---
static CARLITO_REGULAR: &[u8] = include_bytes!("../../assets/fonts/Carlito-Regular.ttf");
static CARLITO_BOLD: &[u8] = include_bytes!("../../assets/fonts/Carlito-Bold.ttf");
static CARLITO_ITALIC: &[u8] = include_bytes!("../../assets/fonts/Carlito-Italic.ttf");
static CARLITO_BOLD_ITALIC: &[u8] = include_bytes!("../../assets/fonts/Carlito-BoldItalic.ttf");

// --- Bundled DejaVu Sans font (DejaVu Fonts License) — Unicode symbol fallback ---
// Covers ☐ ⟨ ⟩ ✓ ✗ → and other symbols absent from Carlito/Liberation Sans.
static DEJAVU_SANS_REGULAR: &[u8] = include_bytes!("../../assets/fonts/DejaVuSans.ttf");

/// The Typst page template with placeholders for config values.
const TYPST_TEMPLATE: &str = r##"
#set page(
  paper: "{page_size}",
  margin: (left: {margin}, right: {margin}, top: {margin}, bottom: {margin}),
  header: [
    #set text(font: ("{font_family}", "Carlito", "Liberation Sans", "DejaVu Sans"), size: {header_size}, fill: rgb("{header_border}"))
    {title}
    #v(2pt)
    #line(length: 100%, stroke: 0.5pt + rgb("{header_border}"))
  ],
  footer: [
    #line(length: 100%, stroke: 0.5pt + rgb("{header_border}"))
    #v(2pt)
    #set text(font: ("{font_family}", "Carlito", "Liberation Sans", "DejaVu Sans"), size: {footer_size}, fill: rgb("{footer_color}"))
    #grid(
      columns: (1fr, 1fr, 1fr),
      align: (left, center, right),
      [{company_name}],
      [{date}],
      [Seite #context counter(page).display() von #context counter(page).final().first()],
    )
  ],
)

#set text(font: ("{font_family}", "Carlito", "Liberation Sans", "DejaVu Sans"), size: {body_size}, lang: "de")
#set par(justify: true, leading: 0.65em)
#set heading(numbering: none)

#show heading.where(level: 1): it => [
  #set text(size: {title_size}, weight: "bold")
  #v(0.8em)
  #it
  #v(0.2em)
  #line(length: 100%, stroke: 0.5pt + rgb("{heading_separator}"))
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
  #set text(font: ({code_font_list}), size: {code_size})
  #block(
    fill: rgb("{code_bg}"),
    inset: {code_block_inset},
    radius: 4pt,
    width: 100%,
    it,
  )
]

#show raw.where(block: false): it => box(
  fill: rgb("{code_bg}"),
  inset: (x: 3pt, y: 0pt),
  outset: (y: 3pt),
  radius: 2pt,
  text(font: ({code_font_list}), size: 0.92em)[#it],
)

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

    // 1. Load bundled fonts: Carlito (Latin/Calibri-compatible) + DejaVu Sans (Unicode symbols)
    let bundled: &[&[u8]] = &[
        CARLITO_REGULAR,
        CARLITO_BOLD,
        CARLITO_ITALIC,
        CARLITO_BOLD_ITALIC,
        DEJAVU_SANS_REGULAR,
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
    heading_buf: String,
    table_header_bg: String,
    table_border: String,
    table_size: String,
    table_cell_inset: String,
    code_font_list: String,
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
            heading_buf: String::new(),
            table_header_bg: doc.colors.table_header_bg.clone(),
            table_border: doc.colors.table_border.clone(),
            table_size: doc.sizes.table.clone(),
            table_cell_inset: doc.sizes.table_cell_inset.clone(),
            code_font_list: build_code_font_list(&doc.fonts.code),
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
                    r.heading_buf.clear();
                    let prefix = "=".repeat(level as usize);
                    r.out.push_str(&format!("\n{} ", prefix));
                }
                Event::End(TagEnd::Heading(_)) => {
                    r.in_heading = false;
                    // Generate a label from heading text for internal link targets
                    let slug = slugify(&r.heading_buf);
                    if !slug.is_empty() {
                        r.out.push_str(&format!("\n#label(\"{}\")", slug));
                    }
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
                    let url = dest_url.to_string();
                    if let Some(anchor) = url.strip_prefix('#') {
                        // Internal anchor link → Typst label reference
                        let slug = slugify(anchor);
                        r.push_to_active(&format!("#link(label(\"{}\"))[", slug));
                    } else {
                        r.push_to_active(&format!("#link(\"{}\")[", url));
                    }
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
                        "\n#block[\n#set text(font: ({}), size: {})\n#set par(justify: false)\n#table(\n  columns: {},\n  fill: (_, row) => if row == 0 {{ rgb(\"{}\") }} else {{ white }},\n  stroke: rgb(\"{}\"),\n  inset: {},\n",
                        r.code_font_list, r.table_size, r.table_columns, r.table_header_bg, r.table_border, r.table_cell_inset,
                    ));
                }
                Event::End(TagEnd::Table) => {
                    r.in_table = false;
                    r.out.push_str(")\n]\n\n");
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
                // pulldown-cmark 0.13: BlockQuote variants now carry an optional kind.
                Event::Start(Tag::BlockQuote(_)) => {
                    r.out.push_str("\n#quote(block: true)[\n");
                }
                Event::End(TagEnd::BlockQuote(_)) => {
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
                        // Add break opportunities so long identifiers wrap inside
                        // narrow table columns instead of overflowing.
                        r.cell_buf
                            .push_str(&typst_escape(&insert_break_opportunities(&text)));
                    } else {
                        if r.in_heading {
                            r.heading_buf.push_str(&text);
                        }
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

                // --- Math (pulldown-cmark 0.13) ---
                // Render LaTeX-like math as escaped raw text — no math typesetting.
                Event::InlineMath(text) | Event::DisplayMath(text) => {
                    r.push_to_active(&typst_escape(&text));
                }

                // pulldown-cmark 0.13 added DefinitionList* and Superscript/Subscript
                // tags. Treat unmatched Start/End tags as no-ops; their inner Text
                // events still flow through and produce reasonable output.
                _ => {}
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
            // Typst-significant characters that appear literally in prose/tables.
            // Curly braces enter Typst code mode and are the most common hard
            // compile error (e.g. `search_x_by_{a,b}` in a table cell).
            '{' => result.push_str("\\{"),
            '}' => result.push_str("\\}"),
            // Emphasis / strong markers — structural emphasis is emitted separately
            // via push_to_active(), so escaping these here only affects literal text.
            '_' => result.push_str("\\_"),
            '*' => result.push_str("\\*"),
            // Content-block brackets — link/strike markup is emitted separately,
            // so these only guard literal brackets inside text.
            '[' => result.push_str("\\["),
            ']' => result.push_str("\\]"),
            '~' => result.push_str("\\~"),
            '$' => result.push_str("\\$"),
            '`' => result.push_str("\\`"),
            _ => result.push(c),
        }
    }
    result
}

/// Insert zero-width-space break opportunities (U+200B) into table-cell text.
///
/// Typst does not break a "word" without a break opportunity, so long
/// `snake_case` identifiers (e.g. `get_product_template_by_ref`) and
/// comma-joined lists without spaces (e.g. `{name,email,city,vat}`) overflow a
/// narrow table column and overlap the neighbouring cell. Inserting a ZWSP after
/// `_`, `,` and `/` lets Typst wrap such tokens inside the cell. ZWSP is invisible
/// and survives `typst_escape` unchanged, so this is applied to the raw text
/// before escaping.
fn insert_break_opportunities(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    for c in s.chars() {
        result.push(c);
        if matches!(c, '_' | ',' | '/') {
            result.push('\u{200B}');
        }
    }
    result
}

/// Build a Typst-compatible font list from a CSS font-family string.
///
/// Parses entries like `'SF Mono', Monaco, 'Cascadia Code', Consolas, monospace`
/// into `"SF Mono", "Monaco", "Cascadia Code", "Consolas"` (Typst format).
/// Filters out generic CSS values like `monospace` and appends reliable fallbacks.
fn build_code_font_list(css_fonts: &str) -> String {
    let mut fonts: Vec<String> = css_fonts
        .split(',')
        .map(|s| s.trim().trim_matches('\'').trim_matches('"').to_string())
        .filter(|s| !s.is_empty() && s != "monospace" && s != "serif" && s != "sans-serif")
        .map(|s| format!("\"{}\"", s))
        .collect();

    // Append reliable fallbacks if not already present
    for fallback in &["DejaVu Sans Mono", "DejaVu Sans", "Liberation Mono"] {
        let quoted = format!("\"{}\"", fallback);
        if !fonts.iter().any(|f| f == &quoted) {
            fonts.push(quoted);
        }
    }

    fonts.join(", ")
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

    let code_font_list = build_code_font_list(&doc.fonts.code);

    TYPST_TEMPLATE
        .replace("{page_size}", &doc.pdf.page_size.to_lowercase())
        .replace("{margin}", &doc.pdf.margin)
        .replace("{header_border}", &doc.colors.header_border)
        .replace("{header_size}", &doc.sizes.header)
        .replace("{footer_color}", &doc.colors.footer)
        .replace("{footer_size}", &doc.sizes.footer)
        .replace("{company_name}", &typst_escape(&doc.resolved_footer_text()))
        .replace("{date}", &options.date)
        .replace("{title}", &typst_escape(&options.title))
        .replace("{font_family}", font_family)
        .replace("{code_font_list}", &code_font_list)
        .replace("{body_size}", &doc.sizes.body)
        .replace("{title_size}", &doc.sizes.title)
        .replace("{h1_size}", &doc.sizes.h1)
        .replace("{h2_size}", &doc.sizes.h2)
        .replace("{h3_size}", &doc.sizes.h3)
        .replace("{table_size}", &doc.sizes.table)
        .replace("{code_size}", &doc.sizes.code)
        .replace("{heading_separator}", &doc.colors.heading_separator)
        .replace("{code_bg}", &doc.colors.code_bg)
        .replace("{code_block_inset}", &doc.sizes.code_block_inset)
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
    fn test_typst_escape_code_mode_chars() {
        // Curly braces are the most common hard compile error: in Typst content
        // mode `{` enters code mode. Literal text containing them must be escaped.
        assert_eq!(
            typst_escape("search_x_by_{a,b}"),
            "search\\_x\\_by\\_\\{a,b\\}"
        );
        assert_eq!(typst_escape("a*b_c"), "a\\*b\\_c");
        assert_eq!(typst_escape("[x]"), "\\[x\\]");
        assert_eq!(typst_escape("~$`"), "\\~\\$\\`");
    }

    #[test]
    fn test_table_cell_with_braces_does_not_emit_raw_brace() {
        // Regression: a table cell like `search_x_by_{a,b}` previously emitted an
        // unescaped `{` into the Typst output, which broke compilation.
        let doc = DocumentConfig::default();
        let md = "| Tool | Rolle |\n|---|---|\n| search_x_by_{a,b}, **create_partner** | Lesen |\n";
        let result = TypstRenderer::render(md, &doc);
        // Inside the rendered cell content, every literal brace must be escaped
        // (no unescaped `{` reaches Typst). Note table cells also receive ZWSP
        // break opportunities after `_` and `,`, so braces are not adjacent to
        // their content verbatim.
        assert!(!result.contains("by_{a,b}"));
        assert!(result.contains("\\{"));
        assert!(result.contains("\\}"));
        assert!(result.contains("\\_"));
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

    #[test]
    fn test_table_cell_long_identifier_gets_break_opportunities() {
        // Long snake_case identifiers and comma-joined lists in a narrow table
        // column must receive zero-width-space break opportunities so they wrap
        // instead of overflowing into the neighbouring cell.
        let doc = DocumentConfig::default();
        let md = "| Tools |\n|---|\n| get_product_template_by_ref |\n";
        let result = TypstRenderer::render(md, &doc);
        assert!(
            result.contains('\u{200B}'),
            "expected ZWSP break opportunities"
        );
        // ZWSP must follow each underscore separator.
        assert!(result.contains("get\\_\u{200B}product"));
    }

    /// Verify that every character in the required Unicode symbol set is covered
    /// by at least one of the 5 bundled font byte slices.
    ///
    /// This test fails if DejaVuSans.ttf is absent, corrupt, or missing expected glyphs,
    /// and also acts as the T-m4v-01 threat-model mitigating integrity check.
    #[test]
    fn bundled_fonts_cover_all_required_codepoints() {
        use ttf_parser::Face;

        let required: &[char] = &[
            '☐', '⟨', '⟩', '✓', '✗', '→', 'ü', 'ä', 'ö', 'ß', 'Ü', 'Ä', 'Ö', '–', '„', '…', '·',
            '≤', '≈',
        ];

        let font_slices: &[&[u8]] = &[
            CARLITO_REGULAR,
            CARLITO_BOLD,
            CARLITO_ITALIC,
            CARLITO_BOLD_ITALIC,
            DEJAVU_SANS_REGULAR,
        ];

        for &c in required {
            let any_has_glyph = font_slices.iter().any(|data| {
                Face::parse(data, 0)
                    .map(|face| face.glyph_index(c).is_some())
                    .unwrap_or(false)
            });
            assert!(
                any_has_glyph,
                "no bundled font covers codepoint U+{:04X} ('{}')",
                c as u32, c
            );
        }
    }

    /// Integration test: export a Markdown snippet containing Unicode symbols to PDF
    /// and verify that DejaVu Sans is embedded in the resulting PDF byte stream.
    ///
    /// Typst only embeds a font if it actually uses glyphs from it — if DejaVu is
    /// absent or unused, the string b"DejaVuSans" would not appear in the PDF.
    #[test]
    #[cfg(feature = "pdf-export")]
    fn integration_export_produces_dejavu_embedded() {
        use crate::browser::pdf_export::{ExportFormat, ExportOptions};
        use std::path::Path;

        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path().join("test.md");
        std::fs::write(&src, "☐ ⟨test⟩ ✓ ✗ → normal text\n").unwrap();
        let pdf_path = tmp.path().join("test.pdf");

        let options = ExportOptions {
            title: "DejaVu test".to_string(),
            author: "test".to_string(),
            date: "11.06.2026".to_string(),
            format: ExportFormat::Pdf,
        };
        let doc = crate::config::DocumentConfig::default();

        export_markdown_to_pdf(Path::new(&src), Path::new(&pdf_path), &options, &doc)
            .expect("PDF export must succeed for integration test");

        let pdf_bytes = std::fs::read(&pdf_path).expect("PDF file must exist after export");

        // Typst embeds fonts with a 6-char subset prefix (e.g. "AUBSHA+DejaVuSans").
        // We search for the invariant "DejaVu" prefix — present in any subset variant.
        let has_dejavu = pdf_bytes.windows(6).any(|w| w == b"DejaVu");
        assert!(
            has_dejavu,
            "DejaVu must be embedded in the PDF — font not found in output bytes"
        );
    }
}
