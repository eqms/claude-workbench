//! Clipboard utility with arboard primary and OSC 52 fallback.
//!
//! arboard requires a display server (X11/Wayland). When running over SSH
//! or on headless Linux, it will fail. OSC 52 escape sequences allow
//! setting the clipboard through the terminal emulator itself, which
//! works over SSH as long as the terminal supports it.

use std::io::Write;

/// Copy text to clipboard.
/// Tries arboard first (local display server), falls back to OSC 52 (terminal emulator).
/// Returns true if arboard succeeded, false if OSC 52 fallback was used.
pub fn copy_to_clipboard(text: &str) -> bool {
    // Try arboard first (works locally with display server)
    if let Ok(mut cb) = arboard::Clipboard::new() {
        if cb.set_text(text).is_ok() {
            return true;
        }
    }

    // Fallback: OSC 52 (works over SSH through terminal emulator)
    osc52_copy(text);
    false
}

/// Read text from clipboard (arboard only — OSC 52 has no read path).
pub fn paste_from_clipboard() -> Option<String> {
    let mut cb = arboard::Clipboard::new().ok()?;
    let text = cb.get_text().ok()?;
    if text.is_empty() {
        None
    } else {
        Some(text)
    }
}

/// Send text to clipboard via OSC 52 escape sequence.
/// This writes directly to stdout, bypassing crossterm's buffering.
fn osc52_copy(text: &str) {
    let encoded = base64_encode(text);
    let osc52 = format!("\x1b]52;c;{}\x07", encoded);
    let _ = std::io::stdout().write_all(osc52.as_bytes());
    let _ = std::io::stdout().flush();
}

fn base64_encode(input: &str) -> String {
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let bytes = input.as_bytes();
    let mut result = String::new();

    for chunk in bytes.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = chunk.get(1).map(|&b| b as u32).unwrap_or(0);
        let b2 = chunk.get(2).map(|&b| b as u32).unwrap_or(0);

        let n = (b0 << 16) | (b1 << 8) | b2;

        result.push(CHARSET[((n >> 18) & 0x3F) as usize] as char);
        result.push(CHARSET[((n >> 12) & 0x3F) as usize] as char);

        if chunk.len() > 1 {
            result.push(CHARSET[((n >> 6) & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }

        if chunk.len() > 2 {
            result.push(CHARSET[(n & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_base64_encode() {
        assert_eq!(base64_encode("Hello"), "SGVsbG8=");
        assert_eq!(base64_encode("Hello, World!"), "SGVsbG8sIFdvcmxkIQ==");
        assert_eq!(base64_encode(""), "");
        assert_eq!(base64_encode("a"), "YQ==");
        assert_eq!(base64_encode("ab"), "YWI=");
        assert_eq!(base64_encode("abc"), "YWJj");
    }
}
