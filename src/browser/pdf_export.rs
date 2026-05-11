//! Markdown to PDF/Markdown export with metadata (date, author, page numbers).
//!
//! PDF generation uses native Typst rendering (pure Rust, no external binaries).

use anyhow::Result;
use std::path::{Path, PathBuf};
use tempfile::Builder;

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
            #[cfg(feature = "pdf-export")]
            {
                crate::browser::typst_pdf::export_markdown_to_pdf(source, target, options, doc)
            }
            #[cfg(not(feature = "pdf-export"))]
            {
                let _ = (source, target, doc);
                Err(anyhow::anyhow!(
                    "PDF export is not available: rebuild with `--features pdf-export`"
                ))
            }
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

/// Create a secure temp file for browser preview using O_EXCL (via tempfile::Builder).
///
/// The returned `NamedTempFile` must be kept alive until the browser has finished
/// reading the file; dropping it deletes the file automatically.
///
/// Replaces the former `default_preview_filename` (predictable path, symlink-attack
/// vector CR-03 / SEC-04) with an unpredictable, kernel-enforced exclusive open.
pub fn default_preview_file(
    source: &Path,
    project_name: &str,
) -> std::io::Result<tempfile::NamedTempFile> {
    let stem = source
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("preview");
    let prefix = if project_name.is_empty() {
        format!("{}-", stem)
    } else {
        format!("{}-{}-", project_name, stem)
    };
    Builder::new()
        .prefix(&prefix)
        .suffix(".html")
        .tempfile_in(std::env::temp_dir())
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
    fn test_preview_file_has_html_suffix() {
        let tmp = default_preview_file(Path::new("README.md"), "myproject").unwrap();
        let name = tmp.path().file_name().unwrap().to_string_lossy();
        assert!(name.ends_with(".html"), "expected .html suffix, got {name}");
        assert!(
            name.contains("myproject-README-"),
            "expected project-stem prefix, got {name}"
        );
    }

    #[test]
    fn test_preview_file_empty_project_name() {
        let tmp = default_preview_file(Path::new("doc.md"), "").unwrap();
        let name = tmp.path().file_name().unwrap().to_string_lossy();
        assert!(name.starts_with("doc-"), "expected stem prefix, got {name}");
        assert!(
            !name.starts_with('-'),
            "must not start with dash, got {name}"
        );
    }

    #[test]
    fn test_preview_files_are_unique() {
        let a = default_preview_file(Path::new("f.md"), "proj").unwrap();
        let b = default_preview_file(Path::new("f.md"), "proj").unwrap();
        assert_ne!(a.path(), b.path(), "two calls must produce different paths");
    }

    #[test]
    fn test_namedtempfile_deletes_on_drop() {
        let path = {
            let tmp = default_preview_file(Path::new("x.md"), "p").unwrap();
            tmp.path().to_path_buf()
        }; // tmp drops here
        assert!(
            !path.exists(),
            "file must be deleted after NamedTempFile drops"
        );
    }

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
