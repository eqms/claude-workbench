//! Markdown to HTML conversion for browser preview

use anyhow::Result;
use std::path::{Path, PathBuf};

use crate::browser::template::TemplateContext;
use crate::config::DocumentConfig;

/// Build HTML document dynamically from DocumentConfig
fn build_html_template(doc: &DocumentConfig) -> String {
    let ctx = TemplateContext::new(doc);
    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>{{title}}</title>
    <style>
        :root {{
            color-scheme: light dark;
        }}
        {base_body}
        {typography}
        {code}
        {blockquote}
        img {{ max-width: 100%; height: auto; }}
        {table}
        {link}
        ul, ol {{ padding-left: 2em; }}
        li {{ margin: 0.25em 0; }}
        hr {{ border: none; border-top: 1px solid #eee; margin: 2em 0; }}
        {footer}
        {dark_mode}
    </style>
</head>
<body>
{{content}}
<div class="footer">
    {footer_text}
</div>
</body>
</html>"#,
        base_body = ctx.base_body_css(),
        typography = ctx.typography_css(),
        code = ctx.code_css(),
        blockquote = ctx.blockquote_css(),
        table = ctx.table_css(),
        link = ctx.link_css(),
        footer = ctx.footer_css(),
        dark_mode = ctx.dark_mode_css(),
        footer_text = ctx.footer_text(),
    )
}

/// Convert markdown file to HTML and return temp file path.
/// Uses consistent naming convention: `{project}-{stem}-{date}.html`
pub fn markdown_to_html(
    md_path: &Path,
    doc: &DocumentConfig,
    project_name: &str,
) -> Result<PathBuf> {
    use pulldown_cmark::{html, Event, Options, Parser};

    let md_content = std::fs::read_to_string(md_path)?;
    let title = md_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Preview");

    // Get the directory of the markdown file for resolving relative paths
    let md_dir = md_path
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());

    // Enable all markdown extensions
    let options = Options::all();
    let parser = Parser::new_ext(&md_content, options);
    let events: Vec<Event> = parser.collect();
    let events = inject_heading_ids(events);

    let mut html_content = String::new();
    html::push_html(&mut html_content, events.into_iter());

    // Convert relative image paths to absolute file:// URLs
    let html_content = fix_image_paths(&html_content, &md_dir);

    let template = build_html_template(doc);
    let html = template
        .replace("{title}", title)
        .replace("{content}", &html_content);

    // Write to consistently named file in temp directory
    let temp_path = crate::browser::pdf_export::default_preview_filename(md_path, project_name);

    use std::io::Write;
    let mut file = std::fs::File::create(&temp_path)?;
    file.write_all(html.as_bytes())?;
    file.flush()?;
    Ok(temp_path)
}

/// Fix relative image paths in HTML by converting them to absolute file:// URLs
fn fix_image_paths(html: &str, base_dir: &Path) -> String {
    use regex::Regex;

    // Static regex: compiled once, cannot fail on this known-good pattern
    use std::sync::LazyLock;
    static IMG_RE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r#"<img\s+([^>]*?)src="([^"]+)"([^>]*)>"#).expect("valid regex")
    });
    let img_re = &*IMG_RE;

    img_re
        .replace_all(html, |caps: &regex::Captures| {
            let before = &caps[1];
            let src = &caps[2];
            let after = &caps[3];

            // Skip if already an absolute URL (http://, https://, file://, data:)
            if src.starts_with("http://")
                || src.starts_with("https://")
                || src.starts_with("file://")
                || src.starts_with("data:")
            {
                return caps[0].to_string();
            }

            // Resolve relative path to absolute with path traversal guard
            let abs_path = base_dir.join(src);
            if let Ok(resolved) = abs_path.canonicalize() {
                let canonical_base = base_dir
                    .canonicalize()
                    .unwrap_or_else(|_| base_dir.to_path_buf());
                if resolved.starts_with(&canonical_base) {
                    let file_url = format!("file://{}", resolved.display());
                    format!(r#"<img {}src="{}"{}>"#, before, file_url, after)
                } else {
                    caps[0].to_string()
                }
            } else {
                caps[0].to_string()
            }
        })
        .to_string()
}

/// Inject `id` attributes into heading events so that internal anchor links work.
///
/// pulldown-cmark does not auto-generate `id` on headings — it only sets `id`
/// when the Markdown source uses explicit `{ #id }` attribute syntax.
/// This function walks the event stream, collects heading text, slugifies it,
/// and injects the `id` into the `Tag::Heading` start event.
fn inject_heading_ids(mut events: Vec<pulldown_cmark::Event>) -> Vec<pulldown_cmark::Event> {
    use pulldown_cmark::{CowStr, Event, Tag, TagEnd};

    let mut i = 0;
    while i < events.len() {
        if let Event::Start(Tag::Heading { .. }) = &events[i] {
            // Collect heading text from subsequent events up to the End tag
            let mut text_buf = String::new();
            let mut j = i + 1;
            while j < events.len() {
                match &events[j] {
                    Event::Text(t) | Event::Code(t) => text_buf.push_str(t),
                    Event::End(TagEnd::Heading(_)) => break,
                    _ => {}
                }
                j += 1;
            }

            let slug = crate::browser::slugify(&text_buf);
            if !slug.is_empty() {
                // Re-match to extract fields, then replace the event
                if let Event::Start(Tag::Heading {
                    level,
                    classes,
                    attrs,
                    ..
                }) = &events[i]
                {
                    let level = *level;
                    let classes = classes.clone();
                    let attrs = attrs.clone();
                    events[i] = Event::Start(Tag::Heading {
                        level,
                        id: Some(CowStr::from(slug)),
                        classes,
                        attrs,
                    });
                }
            }
        }
        i += 1;
    }
    events
}

#[cfg(test)]
mod tests {
    use super::*;

    fn md_to_html_fragment(md: &str) -> String {
        use pulldown_cmark::{html, Event, Options, Parser};

        let options = Options::all();
        let parser = Parser::new_ext(md, options);
        let events: Vec<Event> = parser.collect();
        let events = inject_heading_ids(events);
        let mut out = String::new();
        html::push_html(&mut out, events.into_iter());
        out
    }

    #[test]
    fn test_heading_id_injected() {
        let html = md_to_html_fragment("## Overview");
        assert!(html.contains("id=\"overview\""), "got: {}", html);
    }

    #[test]
    fn test_heading_id_slug_spaces() {
        let html = md_to_html_fragment("## Hello World");
        assert!(html.contains("id=\"hello-world\""), "got: {}", html);
    }

    #[test]
    fn test_heading_id_drops_special_chars() {
        let html = md_to_html_fragment("## C++ & Rust");
        assert!(html.contains("id=\"c-rust\""), "got: {}", html);
    }

    #[test]
    fn test_anchor_link_has_target() {
        let md = "## Overview\n\n[go to overview](#overview)";
        let html = md_to_html_fragment(md);
        assert!(html.contains("id=\"overview\""), "missing id: {}", html);
        assert!(
            html.contains("href=\"#overview\""),
            "missing href: {}",
            html
        );
    }

    #[test]
    fn test_multiple_heading_levels() {
        let md = "# Title\n## Section\n### Subsection";
        let html = md_to_html_fragment(md);
        assert!(html.contains("id=\"title\""), "got: {}", html);
        assert!(html.contains("id=\"section\""), "got: {}", html);
        assert!(html.contains("id=\"subsection\""), "got: {}", html);
    }

    #[test]
    fn test_heading_with_inline_code() {
        let html = md_to_html_fragment("## Use `fmt::Display`");
        assert!(html.contains("id=\"use-fmtdisplay\""), "got: {}", html);
    }
}
