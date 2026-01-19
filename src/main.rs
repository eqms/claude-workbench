pub mod app;
pub mod browser;
pub mod config;
pub mod filter;
pub mod git;
pub mod input;
pub mod session;
pub mod setup;
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

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    config: Option<PathBuf>,

    #[arg(short, long)]
    session: Option<String>,
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

#[tokio::main]
async fn main() -> Result<()> {
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

    let _args = Args::parse();
    let config = load_config()?;
    let session = load_session();

    let terminal = ratatui::init();
    crossterm::execute!(
        std::io::stdout(),
        crossterm::event::EnableMouseCapture,
        crossterm::event::EnableBracketedPaste
    )?;

    let app = App::new(config, session);

    let app_result = app.run(terminal);

    // Normal cleanup
    restore_terminal();

    app_result
}
