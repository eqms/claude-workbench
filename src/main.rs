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

use anyhow::Result;
use app::App;
use clap::Parser;
use config::load_config;
use session::load_session;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    config: Option<PathBuf>,

    #[arg(short, long)]
    session: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
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

    // Initialize PTYs (async or sync) logic could go here or inside App::new
    // For now App::new spawns them.

    let app_result = app.run(terminal);

    crossterm::execute!(
        std::io::stdout(),
        crossterm::event::DisableMouseCapture,
        crossterm::event::DisableBracketedPaste
    )?;
    ratatui::restore();

    app_result
}
