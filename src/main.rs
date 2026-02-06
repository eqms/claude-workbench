pub mod app;
pub mod browser;
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

    /// Fake current version for testing (e.g., "0.37.0")
    #[arg(long, env = "WORKBENCH_FAKE_VERSION")]
    fake_version: Option<String>,

    /// Update to a specific version (for testing/downgrade, e.g., "v0.38.5" or "0.38.5")
    #[arg(long)]
    update_to: Option<String>,
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
            println!("Check the log file for details: {}", update::LOG_FILE);
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

fn main() -> Result<()> {
    // Parse args early - before tokio runtime
    let args = Args::parse();

    // Handle --check-update CLI mode (exit without starting TUI or tokio)
    if args.check_update {
        return run_update_check_cli(args.fake_version);
    }

    // Handle --update-to CLI mode (update to specific version and exit)
    if let Some(target_version) = args.update_to {
        return run_update_to_version_cli(&target_version);
    }

    // Run the async main with tokio runtime
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("Failed to create tokio runtime")
        .block_on(async_main(args.fake_version))
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

    let config = load_config()?;
    let session = load_session();

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
