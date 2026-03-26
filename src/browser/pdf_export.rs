//! Markdown to PDF/Markdown export with metadata (date, author, page numbers).
//!
//! PDF generation uses native Typst rendering (pure Rust, no external binaries).

use anyhow::Result;
use std::path::{Path, PathBuf};

use crate::config::DocumentConfig;

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
pub fn export_markdown(
    source: &Path,
    target: &Path,
    options: &ExportOptions,
    doc: &DocumentConfig,
) -> Result<PathBuf> {
    match options.format {
        ExportFormat::Markdown => export_as_markdown(source, target),
        ExportFormat::Pdf => {
            crate::browser::typst_pdf::export_markdown_to_pdf(source, target, options, doc)
        }
    }
}

/// Simple Markdown copy export
fn export_as_markdown(source: &Path, target: &Path) -> Result<PathBuf> {
    std::fs::copy(source, target)?;
    Ok(target.to_path_buf())
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
