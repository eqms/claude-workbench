pub mod app;
pub mod browser;
pub mod config;
pub mod filter;
pub mod git;
pub mod session;
pub mod setup;
pub mod types;
pub mod ui;
pub mod terminal;
pub mod input;

use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;
use app::App;
use config::load_config;
use session::load_session;

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
    crossterm::execute!(std::io::stdout(), crossterm::event::EnableMouseCapture)?;
    
    let app = App::new(config, session);
    
    // Initialize PTYs (async or sync) logic could go here or inside App::new
    // For now App::new spawns them.
    
    let app_result = app.run(terminal); 
    
    crossterm::execute!(std::io::stdout(), crossterm::event::DisableMouseCapture)?;
    ratatui::restore();
    
    app_result
}
