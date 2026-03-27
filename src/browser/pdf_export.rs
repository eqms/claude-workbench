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

/// Generate current date in German format (dd.mm.yyyy)
pub(crate) fn date_now_dmy() -> String {
    use std::time::SystemTime;
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let time_t = now as libc::time_t;
    let mut tm: libc::tm = unsafe { std::mem::zeroed() };
    #[cfg(not(target_os = "windows"))]
    {
        unsafe {
            libc::localtime_r(&time_t, &mut tm);
        }
    }
    #[cfg(target_os = "windows")]
    {
        unsafe {
            libc::localtime_s(&mut tm, &time_t);
        }
    }
    format!(
        "{:02}.{:02}.{}",
        tm.tm_mday,
        tm.tm_mon + 1,
        tm.tm_year + 1900
    )
}

/// Generate a default export filename based on source file, format and project name.
/// Format: `{project}-{stem}-{dd.mm.yyyy}.{ext}`
pub fn default_export_filename(source: &Path, format: ExportFormat, project_name: &str) -> String {
    let stem = source
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("export");
    let ext = match format {
        ExportFormat::Markdown => "md",
        ExportFormat::Pdf => "pdf",
    };
    let date = date_now_dmy();
    if project_name.is_empty() {
        format!("{}-{}.{}", stem, date, ext)
    } else {
        format!("{}-{}-{}.{}", project_name, stem, date, ext)
    }
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
        let name = default_export_filename(
            Path::new("/path/to/my-plan.md"),
            ExportFormat::Pdf,
            "myproject",
        );
        assert!(name.starts_with("myproject-my-plan-"));
        assert!(name.ends_with(".pdf"));

        let name =
            default_export_filename(Path::new("/path/to/notes.md"), ExportFormat::Markdown, "");
        assert!(name.starts_with("notes-"));
        assert!(name.ends_with(".md"));
    }
}
