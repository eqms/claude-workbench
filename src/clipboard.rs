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

use std::io::{Read, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::mpsc::{self, Sender};
use std::sync::{Mutex, OnceLock};
use std::thread;
use std::time::{Duration, Instant};

/// Maximum time we wait for any clipboard helper subprocess (xclip/xsel/wl-*)
/// to complete. Under XRDP, the X11 selection protocol can hang indefinitely
/// when the X-server's clipboard owner negotiation never completes — without
/// this timeout, every copy/paste call blocks the main event loop.
const SUBPROCESS_TIMEOUT: Duration = Duration::from_millis(500);

/// Poll-interval while waiting for a child to exit. Kept small so the
/// observed wait time stays close to `SUBPROCESS_TIMEOUT`.
const WAIT_POLL_INTERVAL: Duration = Duration::from_millis(20);

/// Outcome of a copy attempt — which backend succeeded, or why all failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClipboardOutcome {
    Arboard,
    Xclip,
    Xsel,
    WlCopy,
    Osc52,
    Failed(String),
    /// Copy job was queued for the background worker thread. The real
    /// outcome will appear later via `take_pending_outcome()`. Treated
    /// as success at the call site so the user sees an immediate flash.
    Submitted,
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
            ClipboardOutcome::Submitted => "submitted",
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
    /// Force OSC 52 only — skip every X11/Wayland helper. Used when the
    /// X-server's clipboard owner negotiation hangs (`CLAUDE_WORKBENCH_CLIPBOARD=osc52`).
    Osc52Only,
}

/// ENV override that takes precedence over auto-detection.
/// Set `CLAUDE_WORKBENCH_CLIPBOARD=osc52` to bypass xclip/xsel/wl-* entirely.
pub const STRATEGY_ENV: &str = "CLAUDE_WORKBENCH_CLIPBOARD";

static STRATEGY: OnceLock<ClipboardStrategy> = OnceLock::new();

fn strategy() -> ClipboardStrategy {
    *STRATEGY.get_or_init(detect_strategy)
}

fn detect_strategy() -> ClipboardStrategy {
    // ENV override has highest priority — used as kill-switch when the
    // X-server hangs and subprocess timeouts still feel sluggish.
    if let Ok(val) = std::env::var(STRATEGY_ENV) {
        match val.trim().to_ascii_lowercase().as_str() {
            "osc52" | "osc-52" | "osc_52" => return ClipboardStrategy::Osc52Only,
            "arboard" => return ClipboardStrategy::ArboardFirst,
            "subprocess" | "xclip" | "xsel" => return ClipboardStrategy::SubprocessFirst,
            _ => {} // fall through to auto-detection on unknown values
        }
    }
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

/// Returns true if `path` has at least one executable bit set (Unix only).
///
/// Extracted as a standalone helper for testability. On non-Unix platforms
/// this function is not compiled in — `which()` falls back to `is_file()` alone.
#[cfg(unix)]
fn is_executable(path: &std::path::Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    path.metadata()
        .map(|m| m.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

/// Look up an executable in PATH. Returns the absolute path on first hit.
///
/// On Unix, only entries that are both a regular file **and** have at least one
/// executable bit set are returned. This prevents a non-executable file on PATH
/// (e.g. a 0644 stub) from being selected and causing a "Permission denied"
/// error at subprocess spawn time with no clear diagnostic.
pub fn which(name: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path) {
        let candidate = dir.join(name);
        if candidate.is_file() {
            #[cfg(unix)]
            if !is_executable(&candidate) {
                continue;
            }
            return Some(candidate);
        }
    }
    None
}

static IS_SSH: OnceLock<bool> = OnceLock::new();

/// Detect whether the current process runs inside an SSH session.
///
/// Checks `SSH_TTY` (set on every interactive SSH login) and
/// `SSH_CONNECTION` (set whenever sshd forwards a session). Result is
/// cached for the lifetime of the process — env-vars do not change at
/// runtime, and callers may invoke this on the hot path (per keystroke).
pub fn is_ssh_session() -> bool {
    *IS_SSH.get_or_init(|| {
        detect_ssh_session(
            std::env::var_os("SSH_TTY").as_deref(),
            std::env::var_os("SSH_CONNECTION").as_deref(),
        )
    })
}

/// Pure detection helper for testing — no env access, no caching.
fn detect_ssh_session(
    ssh_tty: Option<&std::ffi::OsStr>,
    ssh_connection: Option<&std::ffi::OsStr>,
) -> bool {
    // Empty strings count as unset (some shells export empty defaults).
    let nonempty = |v: Option<&std::ffi::OsStr>| v.map(|s| !s.is_empty()).unwrap_or(false);
    nonempty(ssh_tty) || nonempty(ssh_connection)
}

// =====================================================================
// Worker thread — runs all clipboard subprocess calls off the main loop.
// =====================================================================
//
// Even with the 500 ms subprocess timeout from v0.86.4, every copy call
// could still freeze the UI for half a second whenever the X-server
// clipboard hangs. The worker thread decouples the call site from the
// helper's wall-clock cost: the main loop returns immediately with
// `Submitted`, and the worker reports the real outcome (success or
// `Failed(reason)`) into a shared slot that the app polls per frame.
//
// Design notes:
// - One worker thread, single mpsc channel — clipboard jobs are
//   inherently serial (the system clipboard has one slot).
// - The worker is spawned lazily on first use and lives until process
//   exit. We do not implement explicit shutdown because joining on
//   exit would deadlock if a helper subprocess is still draining.
// - Only Copy is async. Paste stays synchronous because callers need
//   the pasted text immediately to inject into a PTY/editor; the
//   subprocess timeout is sufficient there.

enum ClipboardJob {
    Copy(String),
}

static WORKER_TX: OnceLock<Sender<ClipboardJob>> = OnceLock::new();
static OUTCOME_SLOT: OnceLock<Mutex<Option<ClipboardOutcome>>> = OnceLock::new();

fn outcome_slot() -> &'static Mutex<Option<ClipboardOutcome>> {
    OUTCOME_SLOT.get_or_init(|| Mutex::new(None))
}

fn ensure_worker() -> &'static Sender<ClipboardJob> {
    WORKER_TX.get_or_init(|| {
        let (tx, rx) = mpsc::channel::<ClipboardJob>();
        thread::Builder::new()
            .name("clipboard-worker".into())
            .spawn(move || {
                while let Ok(job) = rx.recv() {
                    match job {
                        ClipboardJob::Copy(text) => {
                            let outcome = copy_to_clipboard_sync(&text);
                            // Overwrite any prior pending outcome — the
                            // most recent copy is always the relevant one
                            // for the user's mental model.
                            if let Ok(mut slot) = outcome_slot().lock() {
                                *slot = Some(outcome);
                            }
                        }
                    }
                }
            })
            .expect("spawn clipboard worker thread");
        tx
    })
}

/// Take the most recent worker outcome, if any. Called once per frame
/// by the event loop to surface success/failure to the user (footer
/// flash for errors). Returns `None` if no copy has completed since
/// the last poll.
pub fn take_pending_outcome() -> Option<ClipboardOutcome> {
    outcome_slot().lock().ok().and_then(|mut g| g.take())
}

/// Copy text to clipboard via the background worker thread.
///
/// Returns immediately with `ClipboardOutcome::Submitted` — the real
/// outcome is available later via [`take_pending_outcome()`], which the
/// app event loop polls once per frame. This keeps the UI responsive
/// even when xclip/xsel hangs for the full subprocess timeout window.
pub fn copy_to_clipboard(text: &str) -> ClipboardOutcome {
    let tx = ensure_worker();
    if tx.send(ClipboardJob::Copy(text.to_owned())).is_err() {
        // Worker thread died — fall back to synchronous path so the user
        // still gets a real outcome (which will show as Failed if so).
        return copy_to_clipboard_sync(text);
    }
    ClipboardOutcome::Submitted
}

/// Synchronous copy — used by `--clipboard-diag` and as the worker's
/// inner implementation. Blocks for up to ~500 ms per helper attempt.
pub fn copy_to_clipboard_sync(text: &str) -> ClipboardOutcome {
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

    let strat = strategy();

    // Osc52Only: skip every X11/Wayland helper (the user opted out because
    // the X-server hangs). Fall through to the OSC 52 emit below.
    if !matches!(strat, ClipboardStrategy::Osc52Only) {
        if matches!(strat, ClipboardStrategy::ArboardFirst) {
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

        if matches!(strat, ClipboardStrategy::SubprocessFirst) {
            if let Some(outcome) = try_arboard(&mut errors) {
                return outcome;
            }
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

    // Osc52Only has no read path — return None so callers can fall back to
    // their own behavior (e.g., refusing the F11 paste).
    if matches!(strategy, ClipboardStrategy::Osc52Only) {
        return None;
    }

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

/// Wait for `child` to exit, killing it if it does not finish within
/// `SUBPROCESS_TIMEOUT`. Returns `Ok(status)` on clean exit, `Err(reason)`
/// on timeout or wait error. After timeout, the child is killed and reaped
/// to avoid zombies.
fn wait_or_kill(child: &mut std::process::Child) -> Result<std::process::ExitStatus, String> {
    let deadline = Instant::now() + SUBPROCESS_TIMEOUT;
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return Ok(status),
            Ok(None) => {
                if Instant::now() >= deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(format!("timeout ({} ms)", SUBPROCESS_TIMEOUT.as_millis()));
                }
                std::thread::sleep(WAIT_POLL_INTERVAL);
            }
            Err(e) => return Err(format!("wait: {}", e)),
        }
    }
}

fn run_with_stdin(cmd: &str, args: &[&str], input: &str) -> Result<(), String> {
    let mut child = Command::new(cmd)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("spawn: {}", e))?;
    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(input.as_bytes())
            .map_err(|e| format!("write: {}", e))?;
        // stdin closed on drop here so the child sees EOF and proceeds
    }
    let status = wait_or_kill(&mut child)?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("exit {}", status))
    }
}

fn run_capture(cmd: &str, args: &[&str]) -> Option<String> {
    let mut child = Command::new(cmd)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    let status = wait_or_kill(&mut child).ok()?;
    if !status.success() {
        return None;
    }
    let mut buf = Vec::new();
    if let Some(mut stdout) = child.stdout.take() {
        let _ = stdout.read_to_end(&mut buf);
    }
    Some(String::from_utf8_lossy(&buf).into_owned())
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
    pub strategy_env: Option<String>,
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
            strategy_env: std::env::var(STRATEGY_ENV).ok(),
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

    #[test]
    fn test_detect_ssh_session_unset() {
        assert!(!detect_ssh_session(None, None));
    }

    #[test]
    fn test_detect_ssh_session_only_ssh_tty() {
        let tty = std::ffi::OsString::from("/dev/pts/3");
        assert!(detect_ssh_session(Some(&tty), None));
    }

    #[test]
    fn test_detect_ssh_session_only_ssh_connection() {
        let conn = std::ffi::OsString::from("10.0.0.1 51234 10.0.0.2 22");
        assert!(detect_ssh_session(None, Some(&conn)));
    }

    #[test]
    fn test_detect_ssh_session_empty_strings() {
        let empty = std::ffi::OsString::from("");
        assert!(!detect_ssh_session(Some(&empty), Some(&empty)));
    }

    #[test]
    fn test_is_ssh_session_does_not_panic() {
        // Cached result is whatever the test harness sees; just ensure
        // the call path is sound on every platform.
        let _ = is_ssh_session();
    }

    #[test]
    #[cfg(unix)]
    fn test_is_executable_respects_mode() {
        use std::os::unix::fs::PermissionsExt;
        let tmp = tempfile::NamedTempFile::new().unwrap();
        std::fs::set_permissions(
            tmp.path(),
            std::fs::Permissions::from_mode(0o644),
        )
        .unwrap();
        assert!(!is_executable(tmp.path()), "0o644 file must not be executable");
        std::fs::set_permissions(
            tmp.path(),
            std::fs::Permissions::from_mode(0o755),
        )
        .unwrap();
        assert!(is_executable(tmp.path()), "0o755 file must be executable");
    }
}
