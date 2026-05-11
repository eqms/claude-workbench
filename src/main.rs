pub mod app;
pub mod app_detector;
pub mod browser;
pub mod clipboard;
pub mod config;
pub mod filter;
pub mod git;
pub mod input;
pub mod session;
pub mod setup;
pub mod syntax_registry;
pub mod terminal;
pub mod types;
pub mod ui;
pub mod update;

use anyhow::Result;
use app::App;
use clap::Parser;
use config::load_config;
use session::load_session;
use std::io::Write;
use std::panic;
use std::path::PathBuf;
use update::{
    check_for_update_with_version, perform_update_to_version_sync, UpdateCheckResult, UpdateResult,
};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    config: Option<PathBuf>,

    #[arg(short, long)]
    session: Option<String>,

    /// Check for updates and exit (without starting TUI)
    #[arg(long)]
    check_update: bool,

    /// Fake current version for testing (e.g., "0.37.0").
    /// Only available in debug builds to prevent update-suppression attacks.
    #[cfg(debug_assertions)]
    #[arg(long, env = "WORKBENCH_FAKE_VERSION")]
    fake_version: Option<String>,

    /// Update to a specific version (for testing/downgrade, e.g., "v0.38.5" or "0.38.5").
    /// Only available in debug builds — release binaries do not expose this flag
    /// to prevent privilege-escalation via intentional downgrade to unsigned releases.
    #[cfg(debug_assertions)]
    #[arg(long)]
    update_to: Option<String>,

    /// Diagnose clipboard backends and exit (without starting TUI).
    /// Reports which fallback chain stage is active, which helper binaries
    /// (xclip/xsel/wl-copy/wl-paste) are present, relevant environment
    /// variables, and runs a copy/paste roundtrip test.
    #[arg(long)]
    clipboard_diag: bool,

    /// Diagnose SSH image-paste readiness and exit (without starting TUI).
    /// Reports SSH session state, presence of the `cc-clip` helper on
    /// `$PATH`, and TCP reachability of the cc-clip daemon port (9998).
    /// Use when image paste in the Claude pane fails over SSH from a Mac.
    #[arg(long)]
    ssh_paste_diag: bool,
}

/// Run update check from CLI and exit
fn run_update_check_cli(fake_version: Option<String>) -> Result<()> {
    let current = fake_version.as_deref().unwrap_or(update::CURRENT_VERSION);
    let is_fake = fake_version.is_some();

    println!(
        "Current version: {}{}",
        current,
        if is_fake { " (fake)" } else { "" }
    );
    println!("Checking GitHub releases...");
    println!();

    match check_for_update_with_version(current) {
        UpdateCheckResult::UpToDate => {
            println!("✅ Already up-to-date (v{})", current);
        }
        UpdateCheckResult::UpdateAvailable {
            version,
            release_notes,
        } => {
            println!("🔄 Update available: {}", version);
            if let Some(notes) = release_notes {
                println!();
                println!("── What's New ──────────────────────────────────────");
                for line in notes.lines().take(20) {
                    println!("  {}", line);
                }
                if notes.lines().count() > 20 {
                    println!("  ... (truncated)");
                }
            }
        }
        UpdateCheckResult::NoReleasesFound => {
            println!("⚠️  No releases found for this platform");
            println!(
                "   Platform: {}-{}",
                std::env::consts::ARCH,
                std::env::consts::OS
            );
        }
        UpdateCheckResult::Error(msg) => {
            println!("❌ Error checking for updates: {}", msg);
        }
    }

    Ok(())
}

/// Run update to a specific version from CLI and exit
fn run_update_to_version_cli(target_version: &str) -> Result<()> {
    println!("Current version: {}", update::CURRENT_VERSION);
    println!("Target version:  {}", target_version);
    println!();
    println!("Downloading and installing...");
    println!();

    match perform_update_to_version_sync(target_version) {
        UpdateResult::Success {
            old_version,
            new_version,
        } => {
            println!("✅ Update successful: {} -> {}", old_version, new_version);
            println!();
            println!("Please restart the application to use the new version.");
        }
        UpdateResult::Error(msg) => {
            println!("❌ Update failed: {}", msg);
            println!();
            println!(
                "Check the log file for details: {}",
                update::log_file_path().display()
            );
        }
    }

    Ok(())
}

/// Restore terminal to normal state - called on exit, panic, or signal
fn restore_terminal() {
    let _ = crossterm::execute!(
        std::io::stdout(),
        crossterm::event::DisableMouseCapture,
        crossterm::event::DisableBracketedPaste,
        crossterm::terminal::LeaveAlternateScreen,
        crossterm::cursor::Show
    );
    let _ = crossterm::terminal::disable_raw_mode();
    let _ = std::io::stdout().flush();
}

/// Run clipboard diagnostic from CLI and exit.
fn run_clipboard_diag_cli() -> Result<()> {
    use clipboard::{ClipboardDiag, ClipboardOutcome};

    println!(
        "claude-workbench v{} — clipboard diagnostic",
        env!("CARGO_PKG_VERSION")
    );
    println!();

    let diag = ClipboardDiag::collect();
    println!("Strategy:           {:?}", diag.strategy);
    match &diag.strategy_env {
        Some(v) if !v.is_empty() => {
            println!("  ENV override:     {}={}", clipboard::STRATEGY_ENV, v)
        }
        _ => println!(
            "  ENV override:     (unset — set {}=osc52 to bypass xclip/xsel)",
            clipboard::STRATEGY_ENV
        ),
    }
    println!();
    println!("Helper binaries:");
    fn show(name: &str, path: &Option<std::path::PathBuf>) {
        match path {
            Some(p) => println!("  {:<10} ✓ {}", name, p.display()),
            None => println!("  {:<10} ✗ not found", name),
        }
    }
    show("xclip", &diag.xclip);
    show("xsel", &diag.xsel);
    show("wl-copy", &diag.wl_copy);
    show("wl-paste", &diag.wl_paste);
    println!();

    println!("Environment:");
    fn show_env(name: &str, val: &Option<String>) {
        match val {
            Some(v) if !v.is_empty() => println!("  {:<18} = {}", name, v),
            _ => println!("  {:<18} = (unset)", name),
        }
    }
    show_env("DISPLAY", &diag.display);
    show_env("WAYLAND_DISPLAY", &diag.wayland_display);
    show_env("XDG_SESSION_TYPE", &diag.xdg_session_type);
    show_env("XRDP_SESSION", &diag.xrdp_session);
    show_env("SSH_TTY", &diag.ssh_tty);
    println!();

    let test_marker = format!("workbench-diag-{}", std::process::id());
    println!("Roundtrip test (marker: {}):", test_marker);
    // Diag uses the synchronous path so the reported outcome is the
    // real backend result — not the worker's `Submitted` placeholder.
    let outcome = clipboard::copy_to_clipboard_sync(&test_marker);
    println!("  Copy backend:     {} ({:?})", outcome.label(), outcome);
    if matches!(outcome, ClipboardOutcome::Osc52) {
        println!("  Note: OSC 52 has no read path, skipping paste verification.");
    } else {
        match clipboard::paste_from_clipboard() {
            Some(text) if text == test_marker => {
                println!("  Paste roundtrip:  ✓ matches");
            }
            Some(text) => {
                println!(
                    "  Paste roundtrip:  ✗ mismatch (read back: {:?})",
                    text.chars().take(40).collect::<String>()
                );
            }
            None => {
                println!("  Paste roundtrip:  ✗ paste returned None");
            }
        }
    }
    println!();
    println!("F11 in the TUI uses the same fallback chain to inject paste");
    println!("into the active pane — useful when Kitty's bracketed-paste");
    println!("forwarding is broken (e.g., under XRDP).");

    Ok(())
}

/// Run SSH-image-paste diagnostic from CLI and exit.
///
/// Three checks:
///  1. SSH session detection (`SSH_TTY` / `SSH_CONNECTION`).
///  2. `cc-clip` binary on `$PATH`.
///  3. TCP reachability of the cc-clip daemon on `127.0.0.1:9998` —
///     when set up correctly the user runs `ssh -R 9998:localhost:9998`
///     so the remote port forwards to the Mac-side daemon.
fn run_ssh_paste_diag_cli() -> Result<()> {
    use std::net::{SocketAddr, TcpStream};
    use std::time::Duration;

    println!(
        "claude-workbench v{} — SSH image-paste diagnostic",
        env!("CARGO_PKG_VERSION")
    );
    println!();

    // 1. SSH session detection
    let in_ssh = clipboard::is_ssh_session();
    println!("SSH session:");
    if in_ssh {
        println!("  ✓ detected (SSH_TTY or SSH_CONNECTION set)");
    } else {
        println!("  ✗ not detected — these settings only matter when running over SSH");
    }
    if let Ok(v) = std::env::var("SSH_TTY") {
        println!("    SSH_TTY        = {}", v);
    }
    if let Ok(v) = std::env::var("SSH_CONNECTION") {
        println!("    SSH_CONNECTION = {}", v);
    }
    println!();

    // 2. cc-clip on PATH
    println!("cc-clip helper:");
    match clipboard::which("cc-clip") {
        Some(p) => println!("  ✓ found: {}", p.display()),
        None => {
            println!("  ✗ not on $PATH");
            println!("    Install on this host:  cargo install cc-clip");
            println!("    Project page:           https://github.com/ShunmeiCho/cc-clip");
        }
    }
    println!();

    // 3. cc-clip daemon port reachability (the daemon runs on the Mac;
    //    `ssh -R 9998:localhost:9998` exposes it on this host).
    println!("Daemon reachability (127.0.0.1:9998):");
    let addr: SocketAddr = "127.0.0.1:9998".parse().expect("hardcoded address parses");
    match TcpStream::connect_timeout(&addr, Duration::from_millis(500)) {
        Ok(_) => println!("  ✓ port 9998 reachable — daemon or reverse-tunnel is up"),
        Err(e) => {
            println!("  ✗ port 9998 unreachable: {}", e);
            println!("    On your Mac:    start the cc-clip daemon");
            println!("    ~/.ssh/config:  RemoteForward 9998 localhost:9998");
        }
    }
    println!();
    println!("If all three checks pass, image paste in the Claude pane");
    println!("(Ctrl+V) will route through cc-clip and inject the image path.");

    Ok(())
}

fn main() -> Result<()> {
    // Parse args early - before tokio runtime
    let args = Args::parse();

    // Extract fake_version (only available in debug builds)
    #[cfg(debug_assertions)]
    let fake_version = args.fake_version;
    #[cfg(not(debug_assertions))]
    let fake_version: Option<String> = None;

    // Handle --check-update CLI mode (exit without starting TUI or tokio)
    if args.check_update {
        return run_update_check_cli(fake_version);
    }

    // Handle --update-to CLI mode (update to specific version and exit)
    // Only available in debug builds (field is cfg-gated in Args struct)
    #[cfg(debug_assertions)]
    if let Some(target_version) = args.update_to {
        return run_update_to_version_cli(&target_version);
    }

    // Handle --clipboard-diag CLI mode (exit without starting TUI)
    if args.clipboard_diag {
        return run_clipboard_diag_cli();
    }

    // Handle --ssh-paste-diag CLI mode (exit without starting TUI)
    if args.ssh_paste_diag {
        return run_ssh_paste_diag_cli();
    }

    // Run the async main with tokio runtime
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("Failed to create tokio runtime")
        .block_on(async_main(fake_version))
}

async fn async_main(fake_version: Option<String>) -> Result<()> {
    // Set up panic hook to restore terminal on crash
    let original_hook = panic::take_hook();
    panic::set_hook(Box::new(move |panic_info| {
        restore_terminal();
        original_hook(panic_info);
    }));

    // Ignore SIGTSTP (Ctrl+Z) to prevent suspend with broken terminal state
    // User can still quit with Ctrl+Q or Ctrl+C
    #[cfg(unix)]
    unsafe {
        libc::signal(libc::SIGTSTP, libc::SIG_IGN);
    }

    // Startup-Indikator: zeilenweise auf Stderr — sichtbar bevor ratatui::init()
    // den Alternate-Screen auf Stdout zieht. Stderr bleibt im normalen Buffer
    // und stoert die TUI-Ausgabe nicht. Auf Windows mit ConPTY ist der
    // Spawn-Pfad spuerbar langsamer, daher dort das groesste UX-Plus.
    let t0 = std::time::Instant::now();
    {
        let mut err = std::io::stderr();
        let _ = writeln!(
            err,
            "claude-workbench v{} starting...",
            env!("CARGO_PKG_VERSION")
        );
    }

    let config = load_config()?;
    {
        let mut err = std::io::stderr();
        let _ = writeln!(err, "  config loaded ({} ms)", t0.elapsed().as_millis());
    }

    let session = load_session();

    {
        let mut err = std::io::stderr();
        let _ = writeln!(err, "  spawning panes...");
    }

    let terminal = ratatui::init();
    crossterm::execute!(
        std::io::stdout(),
        crossterm::event::EnableMouseCapture,
        crossterm::event::EnableBracketedPaste
    )?;

    let app = App::new(config, session, fake_version);

    let restart_requested = app.run(terminal);

    // Normal cleanup
    restore_terminal();

    // Check if restart was requested (after update)
    match restart_requested {
        Ok(true) => {
            println!("Restarting application...");
            if let Err(e) = update::restart_application() {
                eprintln!("Restart failed: {}", e);
                eprintln!("Please restart manually.");
                return Err(anyhow::anyhow!("Restart failed: {}", e));
            }
            // exec() on Unix replaces the process, so this is only reached on Windows
            Ok(())
        }
        Ok(false) => Ok(()),
        Err(e) => Err(e),
    }
}
