//! Markdown to HTML conversion for browser preview

use anyhow::Result;
use std::collections::HashMap;
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

/// Convert a single markdown file to an HTML string without writing to disk.
/// Returns (html_string, temp_output_path).
fn convert_single_md(
    md_path: &Path,
    doc: &DocumentConfig,
    project_name: &str,
) -> Result<(String, PathBuf)> {
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

    // Compute temp path (but don't write yet)
    let temp_path = crate::browser::pdf_export::default_preview_filename(md_path, project_name);

    Ok((html, temp_path))
}

/// Collect relative `.md` link hrefs from rendered HTML.
/// Returns raw href values like `"USAGE.md"`, `"./INSTALL.md#section"`.
fn collect_md_links(html: &str) -> Vec<String> {
    use regex::Regex;
    use std::sync::LazyLock;

    static LINK_RE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r#"<a\s[^>]*?href="([^"]*\.md(?:#[^"]*)?)"[^>]*>"#).expect("valid regex")
    });

    let mut links = Vec::new();
    for cap in LINK_RE.captures_iter(html) {
        let href = &cap[1];
        // Skip absolute URLs and absolute paths
        if href.starts_with("http://")
            || href.starts_with("https://")
            || href.starts_with("file://")
            || href.starts_with('/')
        {
            continue;
        }
        links.push(href.to_string());
    }
    links
}

/// Rewrite relative `.md` links in HTML to point to converted temp HTML files.
/// `link_map` maps normalized md filenames (e.g. `"USAGE.md"`) to their temp HTML paths.
fn fix_md_links(html: &str, link_map: &HashMap<String, PathBuf>) -> String {
    use regex::Regex;
    use std::sync::LazyLock;

    static LINK_RE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r#"(<a\s[^>]*?href=")([^"]*\.md)(#[^"]*)?("[^>]*>)"#).expect("valid regex")
    });

    LINK_RE
        .replace_all(html, |caps: &regex::Captures| {
            let prefix = &caps[1];
            let md_href = &caps[2];
            let fragment = caps.get(3).map_or("", |m| m.as_str());
            let suffix = &caps[4];

            // Skip absolute URLs
            if md_href.starts_with("http://")
                || md_href.starts_with("https://")
                || md_href.starts_with("file://")
                || md_href.starts_with('/')
            {
                return caps[0].to_string();
            }

            // Normalize: strip leading "./"
            let normalized = md_href.trim_start_matches("./");
            if let Some(html_path) = link_map.get(normalized) {
                let file_url = format!("file://{}", html_path.display());
                format!("{}{}{}{}", prefix, file_url, fragment, suffix)
            } else {
                caps[0].to_string()
            }
        })
        .to_string()
}

/// Convert markdown file to HTML, including all directly referenced .md files.
/// Returns a Vec of temp file paths (primary file first, then dependencies).
/// All inter-document .md links are rewritten to point to the generated HTML files.
pub fn markdown_to_html(
    md_path: &Path,
    doc: &DocumentConfig,
    project_name: &str,
) -> Result<Vec<PathBuf>> {
    // Phase 1: Convert primary file
    let (primary_html, primary_path) = convert_single_md(md_path, doc, project_name)?;

    // Get source directory for resolving relative paths
    let md_dir = md_path
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());
    let canonical_md_dir = md_dir
        .canonicalize()
        .unwrap_or_else(|_| md_dir.to_path_buf());

    // Collect all referenced .md links from primary HTML
    let md_links = collect_md_links(&primary_html);

    // Build link map: normalized md filename -> temp HTML path
    // Also collect (html_string, temp_path) for each dependency
    let mut link_map: HashMap<String, PathBuf> = HashMap::new();
    let mut dep_files: Vec<(String, PathBuf)> = Vec::new();

    for href in &md_links {
        // Strip fragment for file resolution
        let bare_href = href.split('#').next().unwrap_or(href);
        let normalized = bare_href.trim_start_matches("./");

        // Skip if already processed
        if link_map.contains_key(normalized) {
            continue;
        }

        // Resolve relative path with security guard
        let abs_candidate = md_dir.join(normalized);
        let resolved = match abs_candidate.canonicalize() {
            Ok(r) => r,
            Err(_) => continue, // File doesn't exist, leave link unchanged
        };

        // Path traversal guard: must be under the source directory
        if !resolved.starts_with(&canonical_md_dir) {
            continue;
        }

        // Convert the referenced .md file
        match convert_single_md(&resolved, doc, project_name) {
            Ok((dep_html, dep_path)) => {
                link_map.insert(normalized.to_string(), dep_path.clone());
                dep_files.push((dep_html, dep_path));
            }
            Err(_) => continue, // Conversion failed, leave link unchanged
        }
    }

    // Phase 2: Rewrite .md links in ALL HTML files and write to disk
    use std::io::Write;

    // Write primary file
    let primary_html = fix_md_links(&primary_html, &link_map);
    let mut file = std::fs::File::create(&primary_path)?;
    file.write_all(primary_html.as_bytes())?;
    file.flush()?;

    // Collect all paths for cleanup tracking
    let mut all_paths = vec![primary_path];

    // Write dependency files
    for (dep_html, dep_path) in dep_files {
        let dep_html = fix_md_links(&dep_html, &link_map);
        let mut file = std::fs::File::create(&dep_path)?;
        file.write_all(dep_html.as_bytes())?;
        file.flush()?;
        all_paths.push(dep_path);
    }

    Ok(all_paths)
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

    // --- Tests for collect_md_links ---

    #[test]
    fn test_collect_md_links_simple() {
        let html = r#"<a href="USAGE.md">Usage</a>"#;
        let links = collect_md_links(html);
        assert_eq!(links, vec!["USAGE.md"]);
    }

    #[test]
    fn test_collect_md_links_with_fragment() {
        let html = r#"<a href="INSTALL.md#section">Install</a>"#;
        let links = collect_md_links(html);
        assert_eq!(links, vec!["INSTALL.md#section"]);
    }

    #[test]
    fn test_collect_md_links_ignores_absolute() {
        let html = r#"<a href="https://example.com/foo.md">ext</a>
                       <a href="/absolute.md">abs</a>
                       <a href="file:///tmp/local.md">file</a>"#;
        let links = collect_md_links(html);
        assert!(links.is_empty(), "got: {:?}", links);
    }

    #[test]
    fn test_collect_md_links_dotslash() {
        let html = r#"<a href="./USAGE.md">Usage</a>"#;
        let links = collect_md_links(html);
        assert_eq!(links, vec!["./USAGE.md"]);
    }

    // --- Tests for fix_md_links ---

    #[test]
    fn test_fix_md_links_rewrites() {
        let html = r#"<a href="USAGE.md">Usage</a>"#;
        let mut map = HashMap::new();
        map.insert(
            "USAGE.md".to_string(),
            PathBuf::from("/tmp/project-usage-01.01.2026.html"),
        );
        let result = fix_md_links(html, &map);
        assert!(
            result.contains("file:///tmp/project-usage-01.01.2026.html"),
            "got: {}",
            result
        );
    }

    #[test]
    fn test_fix_md_links_preserves_fragment() {
        let html = r#"<a href="USAGE.md#shortcuts">Shortcuts</a>"#;
        let mut map = HashMap::new();
        map.insert(
            "USAGE.md".to_string(),
            PathBuf::from("/tmp/project-usage-01.01.2026.html"),
        );
        let result = fix_md_links(html, &map);
        assert!(
            result.contains("file:///tmp/project-usage-01.01.2026.html#shortcuts"),
            "got: {}",
            result
        );
    }

    #[test]
    fn test_fix_md_links_leaves_unknown() {
        let html = r#"<a href="UNKNOWN.md">Unknown</a>"#;
        let map = HashMap::new();
        let result = fix_md_links(html, &map);
        assert_eq!(result, html);
    }

    #[test]
    fn test_fix_md_links_normalizes_dotslash() {
        let html = r#"<a href="./USAGE.md">Usage</a>"#;
        let mut map = HashMap::new();
        map.insert(
            "USAGE.md".to_string(),
            PathBuf::from("/tmp/project-usage-01.01.2026.html"),
        );
        let result = fix_md_links(html, &map);
        assert!(
            result.contains("file:///tmp/project-usage-01.01.2026.html"),
            "got: {}",
            result
        );
    }
}
