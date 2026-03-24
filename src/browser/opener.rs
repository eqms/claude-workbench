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

/// Opens a file with a specific browser, or falls back to system default
pub fn open_file_with_browser(path: &Path, browser: &str) -> Result<()> {
    if browser.is_empty() {
        open_file(path)
    } else {
        let (program, args) = split_command(browser);
        std::process::Command::new(&program)
            .args(&args)
            .arg(path)
            .spawn()?;
        Ok(())
    }
}

/// Opens a file with an external GUI editor
pub fn open_file_with_editor(path: &Path, editor: &str) -> Result<()> {
    if editor.is_empty() {
        anyhow::bail!("No external editor configured");
    }
    let (program, args) = split_command(editor);
    std::process::Command::new(&program)
        .args(&args)
        .arg(path)
        .spawn()?;
    Ok(())
}

/// Split a command string into program and arguments with quote-aware parsing.
/// Handles patterns like: "firefox", "open -a Firefox", "open -a \"Brave Browser\""
fn split_command(cmd: &str) -> (String, Vec<String>) {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    let chars = cmd.chars().peekable();

    for c in chars {
        match c {
            '"' => in_quotes = !in_quotes,
            ' ' if !in_quotes => {
                if !current.is_empty() {
                    tokens.push(std::mem::take(&mut current));
                }
            }
            _ => current.push(c),
        }
    }
    if !current.is_empty() {
        tokens.push(current);
    }

    if tokens.is_empty() {
        (cmd.to_string(), Vec::new())
    } else {
        let program = tokens.remove(0);
        (program, tokens)
    }
}

/// Check if file is a markdown file
pub fn is_markdown(path: &Path) -> bool {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase());

    matches!(ext.as_deref(), Some("md" | "markdown" | "mdown" | "mkd"))
}
