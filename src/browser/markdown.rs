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
    use pulldown_cmark::{html, Options, Parser};

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

    let mut html_content = String::new();
    html::push_html(&mut html_content, parser);

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
