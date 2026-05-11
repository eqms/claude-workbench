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

/// Validate that a program name contains only safe characters.
/// Accepts: ASCII alphanumerics, `_`, `-`, `.`, `/`, `+`.
/// Rejects: empty strings, spaces, shell metacharacters (`;`, `|`, `&`, `$`, `` ` ``, `(`, `)`, etc.).
fn validate_program(prog: &str) -> Result<()> {
    if prog.is_empty()
        || !prog
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.' | '/' | '+'))
    {
        anyhow::bail!("Unsafe program name in browser/editor config: {:?}", prog);
    }
    Ok(())
}

/// Opens a file with a specific browser, or falls back to system default
pub fn open_file_with_browser(path: &Path, browser: &str) -> Result<()> {
    if browser.is_empty() {
        open_file(path)
    } else {
        let tokens = shlex::split(browser).ok_or_else(|| {
            anyhow::anyhow!("Invalid shell quoting in browser config: {:?}", browser)
        })?;
        let program = tokens
            .first()
            .ok_or_else(|| anyhow::anyhow!("Empty browser command"))?;
        validate_program(program)?;
        std::process::Command::new(program)
            .args(&tokens[1..])
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
    let tokens = shlex::split(editor)
        .ok_or_else(|| anyhow::anyhow!("Invalid shell quoting in editor config: {:?}", editor))?;
    let program = tokens
        .first()
        .ok_or_else(|| anyhow::anyhow!("Empty editor command"))?;
    validate_program(program)?;
    std::process::Command::new(program)
        .args(&tokens[1..])
        .arg(path)
        .spawn()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_program_accepts_safe_names() {
        assert!(validate_program("firefox").is_ok());
        assert!(validate_program("open").is_ok());
        assert!(validate_program("/usr/bin/xdg-open").is_ok());
        assert!(validate_program("open-a-browser").is_ok());
        assert!(validate_program("g++").is_ok());
    }

    #[test]
    fn test_validate_program_rejects_metacharacters() {
        assert!(validate_program("").is_err(), "empty must be rejected");
        assert!(validate_program("fire;fox").is_err(), "semicolon");
        assert!(validate_program("$(rm -rf /)").is_err(), "subshell");
        assert!(validate_program("a b").is_err(), "space");
        assert!(validate_program("a|b").is_err(), "pipe");
        assert!(validate_program("a&b").is_err(), "ampersand");
        assert!(validate_program("a`b`").is_err(), "backtick");
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
