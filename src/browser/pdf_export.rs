//! Markdown to PDF/Markdown export with metadata (date, author, page numbers).

use anyhow::Result;
use std::path::{Path, PathBuf};

/// Export format selection
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportFormat {
    Markdown,
    Pdf,
}

/// Options for the export operation
#[derive(Debug, Clone)]
pub struct ExportOptions {
    pub title: String,
    pub author: String,
    pub date: String,
    pub format: ExportFormat,
}

/// Export a Markdown file to the target path in the specified format.
pub fn export_markdown(source: &Path, target: &Path, options: &ExportOptions) -> Result<PathBuf> {
    match options.format {
        ExportFormat::Markdown => export_as_markdown(source, target),
        ExportFormat::Pdf => export_as_pdf(source, target, options),
    }
}

/// Simple Markdown copy export
fn export_as_markdown(source: &Path, target: &Path) -> Result<PathBuf> {
    std::fs::copy(source, target)?;
    Ok(target.to_path_buf())
}

/// Export Markdown as PDF via Chrome headless or wkhtmltopdf
fn export_as_pdf(source: &Path, target: &Path, options: &ExportOptions) -> Result<PathBuf> {
    // Step 1: Convert Markdown to HTML with print-optimized template
    let html_path = markdown_to_pdf_html(source, options)?;

    // Step 2: Find a PDF renderer
    let renderer = crate::app_detector::find_pdf_renderer().ok_or_else(|| {
        anyhow::anyhow!("No PDF renderer found. Install Google Chrome, Chromium, or wkhtmltopdf.")
    })?;

    // Step 3: Convert HTML to PDF
    let result = if renderer == "wkhtmltopdf" {
        render_with_wkhtmltopdf(&html_path, target)
    } else {
        render_with_chrome(&renderer, &html_path, target)
    };

    // Clean up temp HTML file
    let _ = std::fs::remove_file(&html_path);

    result
}

/// Convert Markdown to a print-optimized HTML file (temp)
fn markdown_to_pdf_html(md_path: &Path, options: &ExportOptions) -> Result<PathBuf> {
    use pulldown_cmark::{html, Options, Parser};

    let md_content = std::fs::read_to_string(md_path)?;

    let parser_options = Options::all();
    let parser = Parser::new_ext(&md_content, parser_options);

    let mut html_content = String::new();
    html::push_html(&mut html_content, parser);

    let html = PDF_HTML_TEMPLATE
        .replace("{title}", &options.title)
        .replace("{author}", &options.author)
        .replace("{date}", &options.date)
        .replace("{content}", &html_content);

    let named_temp = tempfile::Builder::new()
        .prefix("cwb-pdf-export-")
        .suffix(".html")
        .tempfile()?;

    use std::io::Write;
    let (mut file, temp_path) = named_temp.keep()?;
    file.write_all(html.as_bytes())?;
    file.flush()?;
    Ok(temp_path)
}

/// Render PDF using Chrome/Chromium headless
fn render_with_chrome(chrome_bin: &str, html_path: &Path, output: &Path) -> Result<PathBuf> {
    let file_url = format!("file://{}", html_path.display());
    let print_flag = format!("--print-to-pdf={}", output.display());

    let mut cmd = std::process::Command::new(chrome_bin);
    cmd.args(["--headless", "--disable-gpu"]);
    // --no-sandbox only needed on Linux (containerized environments)
    #[cfg(target_os = "linux")]
    cmd.arg("--no-sandbox");
    cmd.args([
        "--run-all-compositor-stages-before-draw",
        &print_flag,
        &file_url,
    ]);
    let status = cmd
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()?;

    if status.success() {
        Ok(output.to_path_buf())
    } else {
        anyhow::bail!(
            "Chrome PDF generation failed (exit code: {:?})",
            status.code()
        )
    }
}

/// Render PDF using wkhtmltopdf
fn render_with_wkhtmltopdf(html_path: &Path, output: &Path) -> Result<PathBuf> {
    let status = std::process::Command::new("wkhtmltopdf")
        .args([
            "--enable-local-file-access",
            "--page-size",
            "A4",
            "--margin-top",
            "20mm",
            "--margin-bottom",
            "20mm",
            "--margin-left",
            "15mm",
            "--margin-right",
            "15mm",
        ])
        .arg(html_path)
        .arg(output)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()?;

    if status.success() {
        Ok(output.to_path_buf())
    } else {
        anyhow::bail!(
            "wkhtmltopdf PDF generation failed (exit code: {:?})",
            status.code()
        )
    }
}

/// Resolve the effective export directory
pub fn resolve_export_dir(configured: &str) -> PathBuf {
    if configured.is_empty() {
        dirs::download_dir()
            .or_else(|| dirs::home_dir().map(|h| h.join("Downloads")))
            .unwrap_or_else(|| PathBuf::from("."))
    } else {
        let expanded = if configured.starts_with('~') {
            if let Some(home) = dirs::home_dir() {
                home.join(configured.trim_start_matches("~/"))
            } else {
                PathBuf::from(configured)
            }
        } else {
            PathBuf::from(configured)
        };
        expanded
    }
}

/// Generate a default export filename based on source file and format
pub fn default_export_filename(source: &Path, format: ExportFormat) -> String {
    let stem = source
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("export");
    let ext = match format {
        ExportFormat::Markdown => "md",
        ExportFormat::Pdf => "pdf",
    };
    format!("{}.{}", stem, ext)
}

// ── Print-optimized HTML template ──────────────────────────────────────

const PDF_HTML_TEMPLATE: &str = r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>{title}</title>
    <style>
        body {
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Oxygen, Ubuntu, sans-serif;
            max-width: 100%;
            margin: 0;
            padding: 0;
            line-height: 1.6;
            color: #333;
            background: #fff;
            font-size: 11pt;
        }
        .document-header {
            border-bottom: 2px solid #333;
            padding-bottom: 0.5em;
            margin-bottom: 1.5em;
        }
        .document-header h1 {
            margin: 0 0 0.2em 0;
            font-size: 18pt;
            border: none;
            padding: 0;
        }
        .document-meta {
            font-size: 9pt;
            color: #666;
        }
        h1, h2, h3, h4, h5, h6 { margin-top: 1.5em; margin-bottom: 0.5em; }
        h1 { border-bottom: 2px solid #eee; padding-bottom: 0.3em; font-size: 16pt; }
        h2 { border-bottom: 1px solid #eee; padding-bottom: 0.3em; font-size: 14pt; }
        h3 { font-size: 12pt; }
        code {
            font-family: 'SF Mono', Monaco, 'Cascadia Code', Consolas, monospace;
            background: #f4f4f4;
            color: #333;
            padding: 0.2em 0.4em;
            border-radius: 3px;
            font-size: 0.9em;
        }
        pre {
            background: #f4f4f4;
            color: #333;
            padding: 0.8em;
            overflow-x: auto;
            border-radius: 4px;
            font-size: 9pt;
            border: 1px solid #ddd;
        }
        pre code {
            background: none;
            color: inherit;
            padding: 0;
        }
        blockquote {
            border-left: 4px solid #ddd;
            margin: 1em 0;
            padding-left: 1rem;
            color: #666;
        }
        img { max-width: 100%; height: auto; }
        table {
            border-collapse: collapse;
            width: 100%;
            margin: 1em 0;
            font-size: 10pt;
        }
        th, td {
            border: 1px solid #ccc;
            padding: 6px 10px;
            text-align: left;
        }
        th { background: #e8e8e8; color: #1a1a1a; font-weight: 600; }
        td { background: #fafafa; }
        a { color: #0366d6; text-decoration: none; }
        ul, ol { padding-left: 2em; }
        li { margin: 0.25em 0; }
        hr { border: none; border-top: 1px solid #eee; margin: 2em 0; }
        .document-footer {
            margin-top: 3rem;
            padding-top: 0.5em;
            border-top: 1px solid #ccc;
            font-size: 8pt;
            color: #999;
            text-align: center;
        }

        @media print {
            body { margin: 0; padding: 0; }
            pre { page-break-inside: avoid; }
            table { page-break-inside: avoid; }
            h1, h2, h3 { page-break-after: avoid; }
            @page {
                size: A4;
                margin: 2cm;
            }
        }
    </style>
</head>
<body>
<div class="document-header">
    <h1>{title}</h1>
    <div class="document-meta">{author} &mdash; {date}</div>
</div>
{content}
<div class="document-footer">
    Generated by Claude Workbench &mdash; {date}
</div>
</body>
</html>"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_export_dir_empty() {
        let dir = resolve_export_dir("");
        // Should resolve to Downloads or current dir
        assert!(dir.to_str().is_some());
    }

    #[test]
    fn test_resolve_export_dir_custom() {
        let dir = resolve_export_dir("/tmp/exports");
        assert_eq!(dir, PathBuf::from("/tmp/exports"));
    }

    #[test]
    fn test_default_export_filename() {
        let name = default_export_filename(Path::new("/path/to/my-plan.md"), ExportFormat::Pdf);
        assert_eq!(name, "my-plan.pdf");

        let name = default_export_filename(Path::new("/path/to/notes.md"), ExportFormat::Markdown);
        assert_eq!(name, "notes.md");
    }
}
