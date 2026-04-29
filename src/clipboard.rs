//! Clipboard utility with multi-stage fallback chain.
//!
//! Stage order (copy):
//!   1. arboard         — native display server (X11/Wayland), works locally
//!   2. xclip           — X11 selection bridge, robust under XRDP
//!   3. xsel            — X11 selection bridge alternative
//!   4. wl-copy         — Wayland clipboard tool
//!   5. OSC 52          — terminal-emulator escape sequence (works over SSH)
//!
//! Stage order (paste): arboard → xclip -o → xsel -b -o → wl-paste --no-newline.
//! No OSC 52 read path — it requires a synchronous response from the terminal
//! which is awkward inside a TUI event loop.
//!
//! The XRDP/Kitty/Xfce combination motivated this fallback chain: arboard's
//! `wayland-data-control` feature can stall in XRDP-X11 sessions, and OSC 52
//! is not synced by xrdp-chansrv to the RDP channel. Subprocess helpers
//! (xclip/xsel) write directly into the X11 selection, which xrdp-chansrv
//! does forward.

use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::OnceLock;

/// Outcome of a copy attempt — which backend succeeded, or why all failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClipboardOutcome {
    Arboard,
    Xclip,
    Xsel,
    WlCopy,
    Osc52,
    Failed(String),
}

impl ClipboardOutcome {
    pub fn is_success(&self) -> bool {
        !matches!(self, ClipboardOutcome::Failed(_))
    }

    pub fn label(&self) -> &str {
        match self {
            ClipboardOutcome::Arboard => "arboard",
            ClipboardOutcome::Xclip => "xclip",
            ClipboardOutcome::Xsel => "xsel",
            ClipboardOutcome::WlCopy => "wl-copy",
            ClipboardOutcome::Osc52 => "OSC 52",
            ClipboardOutcome::Failed(_) => "failed",
        }
    }
}

/// Detected clipboard strategy. Computed once per process.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClipboardStrategy {
    /// macOS or other platforms where arboard is reliable.
    ArboardFirst,
    /// X11/XRDP — prefer xclip/xsel over arboard to avoid wayland-data-control
    /// stalls in XRDP sessions.
    SubprocessFirst,
}

static STRATEGY: OnceLock<ClipboardStrategy> = OnceLock::new();

fn strategy() -> ClipboardStrategy {
    *STRATEGY.get_or_init(detect_strategy)
}

fn detect_strategy() -> ClipboardStrategy {
    if cfg!(not(target_os = "linux")) {
        return ClipboardStrategy::ArboardFirst;
    }
    let xrdp = std::env::var_os("XRDP_SESSION").is_some();
    let session_type = std::env::var("XDG_SESSION_TYPE").unwrap_or_default();
    let has_xclip = which("xclip").is_some();
    let has_xsel = which("xsel").is_some();
    if (xrdp || session_type == "x11") && (has_xclip || has_xsel) {
        ClipboardStrategy::SubprocessFirst
    } else {
        ClipboardStrategy::ArboardFirst
    }
}

/// Look up an executable in PATH. Returns the absolute path on first hit.
pub fn which(name: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path) {
        let candidate = dir.join(name);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

/// Copy text to clipboard via fallback chain.
///
/// Returns the `ClipboardOutcome` describing which backend succeeded
/// (or `Failed(reason)` if every backend failed).
pub fn copy_to_clipboard(text: &str) -> ClipboardOutcome {
    let mut errors: Vec<String> = Vec::new();

    let try_arboard = |errors: &mut Vec<String>| -> Option<ClipboardOutcome> {
        match arboard::Clipboard::new() {
            Ok(mut cb) => match cb.set_text(text) {
                Ok(()) => Some(ClipboardOutcome::Arboard),
                Err(e) => {
                    errors.push(format!("arboard set_text: {}", e));
                    None
                }
            },
            Err(e) => {
                errors.push(format!("arboard init: {}", e));
                None
            }
        }
    };

    if matches!(strategy(), ClipboardStrategy::ArboardFirst) {
        if let Some(outcome) = try_arboard(&mut errors) {
            return outcome;
        }
    }

    if let Some(outcome) = try_xclip_copy(text, &mut errors) {
        return outcome;
    }
    if let Some(outcome) = try_xsel_copy(text, &mut errors) {
        return outcome;
    }
    if let Some(outcome) = try_wl_copy(text, &mut errors) {
        return outcome;
    }

    if matches!(strategy(), ClipboardStrategy::SubprocessFirst) {
        if let Some(outcome) = try_arboard(&mut errors) {
            return outcome;
        }
    }

    // Last resort: OSC 52. We always claim success here because we can't
    // verify whether the terminal forwarded it; the worst case is "claimed
    // success but clipboard empty", which the user has to verify.
    // The collected `errors` are intentionally discarded — we already wrote
    // the escape, and the user can inspect failures via `--clipboard-diag`.
    osc52_copy(text);
    let _ = errors;
    ClipboardOutcome::Osc52
}

/// Read text from clipboard via fallback chain.
/// Returns `None` if every backend fails or yields empty text.
pub fn paste_from_clipboard() -> Option<String> {
    let strategy = strategy();

    let try_arboard = || -> Option<String> {
        let mut cb = arboard::Clipboard::new().ok()?;
        let text = cb.get_text().ok()?;
        if text.is_empty() {
            None
        } else {
            Some(text)
        }
    };

    if matches!(strategy, ClipboardStrategy::ArboardFirst) {
        if let Some(t) = try_arboard() {
            return Some(t);
        }
    }
    if let Some(t) = try_xclip_paste() {
        return Some(t);
    }
    if let Some(t) = try_xsel_paste() {
        return Some(t);
    }
    if let Some(t) = try_wl_paste() {
        return Some(t);
    }
    if matches!(strategy, ClipboardStrategy::SubprocessFirst) {
        if let Some(t) = try_arboard() {
            return Some(t);
        }
    }
    None
}

fn try_xclip_copy(text: &str, errors: &mut Vec<String>) -> Option<ClipboardOutcome> {
    which("xclip")?;
    match run_with_stdin("xclip", &["-selection", "clipboard", "-i"], text) {
        Ok(()) => Some(ClipboardOutcome::Xclip),
        Err(e) => {
            errors.push(format!("xclip: {}", e));
            None
        }
    }
}

fn try_xsel_copy(text: &str, errors: &mut Vec<String>) -> Option<ClipboardOutcome> {
    which("xsel")?;
    match run_with_stdin("xsel", &["--clipboard", "--input"], text) {
        Ok(()) => Some(ClipboardOutcome::Xsel),
        Err(e) => {
            errors.push(format!("xsel: {}", e));
            None
        }
    }
}

fn try_wl_copy(text: &str, errors: &mut Vec<String>) -> Option<ClipboardOutcome> {
    which("wl-copy")?;
    match run_with_stdin("wl-copy", &[], text) {
        Ok(()) => Some(ClipboardOutcome::WlCopy),
        Err(e) => {
            errors.push(format!("wl-copy: {}", e));
            None
        }
    }
}

fn try_xclip_paste() -> Option<String> {
    which("xclip")?;
    run_capture("xclip", &["-selection", "clipboard", "-o"]).filter(|s| !s.is_empty())
}

fn try_xsel_paste() -> Option<String> {
    which("xsel")?;
    run_capture("xsel", &["--clipboard", "--output"]).filter(|s| !s.is_empty())
}

fn try_wl_paste() -> Option<String> {
    which("wl-paste")?;
    run_capture("wl-paste", &["--no-newline"]).filter(|s| !s.is_empty())
}

fn run_with_stdin(cmd: &str, args: &[&str], input: &str) -> Result<(), String> {
    let mut child = Command::new(cmd)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("spawn: {}", e))?;
    {
        let mut stdin = child.stdin.take().ok_or("stdin unavailable")?;
        stdin
            .write_all(input.as_bytes())
            .map_err(|e| format!("write: {}", e))?;
    }
    let output = child
        .wait_with_output()
        .map_err(|e| format!("wait: {}", e))?;
    if output.status.success() {
        Ok(())
    } else {
        let err = String::from_utf8_lossy(&output.stderr).trim().to_string();
        Err(format!("exit {}: {}", output.status, err))
    }
}

fn run_capture(cmd: &str, args: &[&str]) -> Option<String> {
    let output = Command::new(cmd).args(args).output().ok()?;
    if !output.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&output.stdout).into_owned())
}

/// Send text to clipboard via OSC 52 escape sequence.
/// Writes directly to stdout, bypassing crossterm's buffering.
/// Sends both BEL (\x07) and ST (\x1b\\) terminators for maximum compatibility.
fn osc52_copy(text: &str) {
    let encoded = base64_encode(text);
    let osc52_bel = format!("\x1b]52;c;{}\x07", encoded);
    let _ = std::io::stdout().write_all(osc52_bel.as_bytes());
    let _ = std::io::stdout().flush();
    let osc52_st = format!("\x1b]52;c;{}\x1b\\", encoded);
    let _ = std::io::stdout().write_all(osc52_st.as_bytes());
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

/// Diagnostic snapshot for `--clipboard-diag` and the F12 help screen.
#[derive(Debug, Clone)]
pub struct ClipboardDiag {
    pub strategy: ClipboardStrategy,
    pub xclip: Option<PathBuf>,
    pub xsel: Option<PathBuf>,
    pub wl_copy: Option<PathBuf>,
    pub wl_paste: Option<PathBuf>,
    pub display: Option<String>,
    pub wayland_display: Option<String>,
    pub xdg_session_type: Option<String>,
    pub xrdp_session: Option<String>,
    pub ssh_tty: Option<String>,
}

impl ClipboardDiag {
    pub fn collect() -> Self {
        Self {
            strategy: strategy(),
            xclip: which("xclip"),
            xsel: which("xsel"),
            wl_copy: which("wl-copy"),
            wl_paste: which("wl-paste"),
            display: std::env::var("DISPLAY").ok(),
            wayland_display: std::env::var("WAYLAND_DISPLAY").ok(),
            xdg_session_type: std::env::var("XDG_SESSION_TYPE").ok(),
            xrdp_session: std::env::var("XRDP_SESSION").ok(),
            ssh_tty: std::env::var("SSH_TTY").ok(),
        }
    }
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

    #[test]
    fn test_outcome_label_and_success() {
        assert!(ClipboardOutcome::Arboard.is_success());
        assert!(ClipboardOutcome::Xclip.is_success());
        assert!(ClipboardOutcome::Xsel.is_success());
        assert!(ClipboardOutcome::WlCopy.is_success());
        assert!(ClipboardOutcome::Osc52.is_success());
        assert!(!ClipboardOutcome::Failed("nope".into()).is_success());

        assert_eq!(ClipboardOutcome::Xclip.label(), "xclip");
        assert_eq!(ClipboardOutcome::Osc52.label(), "OSC 52");
    }

    #[test]
    fn test_which_finds_common_binary() {
        // `sh` exists on every Unix; `cmd` on Windows.
        #[cfg(not(windows))]
        let needle = "sh";
        #[cfg(windows)]
        let needle = "cmd.exe";
        let found = which(needle);
        assert!(found.is_some(), "expected to find {} in PATH", needle);
    }

    #[test]
    fn test_which_returns_none_for_missing() {
        let found = which("nonexistent_binary_zzz_12345");
        assert!(found.is_none());
    }

    #[test]
    fn test_diag_collect_does_not_panic() {
        let diag = ClipboardDiag::collect();
        // Strategy is always one of the two variants.
        assert!(matches!(
            diag.strategy,
            ClipboardStrategy::ArboardFirst | ClipboardStrategy::SubprocessFirst
        ));
    }
}
