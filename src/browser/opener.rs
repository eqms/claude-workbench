//! Platform-specific file opening utilities

use anyhow::Result;
use std::path::Path;

/// Opens a file with the system's default application
pub fn open_file(path: &Path) -> Result<()> {
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open").arg(path).spawn()?;
    }

    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open").arg(path).spawn()?;
    }

    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("cmd")
            .args(["/c", "start", ""])
            .arg(path)
            .spawn()?;
    }

    Ok(())
}

/// Opens a directory in the system file manager
pub fn open_in_file_manager(path: &Path) -> Result<()> {
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open").arg(path).spawn()?;
    }

    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open").arg(path).spawn()?;
    }

    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer").arg(path).spawn()?;
    }

    Ok(())
}

/// Check if file can be previewed in browser/external viewer
pub fn can_preview_in_browser(path: &Path) -> bool {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase());

    // Native browser/viewer types
    if matches!(
        ext.as_deref(),
        Some(
            "html"
                | "htm"
                | "md"
                | "markdown"
                | "mdown"
                | "mkd"
                | "pdf"
                | "png"
                | "jpg"
                | "jpeg"
                | "gif"
                | "svg"
                | "webp"
        )
    ) {
        return true;
    }

    // Text files that can be syntax-highlighted
    crate::browser::syntax::can_syntax_highlight(path)
}

/// Check if file is a markdown file
pub fn is_markdown(path: &Path) -> bool {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase());

    matches!(ext.as_deref(), Some("md" | "markdown" | "mdown" | "mkd"))
}
