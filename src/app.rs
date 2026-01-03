use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::{layout::Rect, DefaultTerminal, Frame};
use std::collections::HashMap;

use crate::config::Config;
use crate::session::SessionState;
use crate::ui;
use crate::types::PaneId;
use crate::terminal::PseudoTerminal;
use crate::ui::file_browser::FileBrowserState;
use crate::ui::preview::PreviewState;

pub struct App {
    pub config: Config,
    pub session: SessionState,
    pub should_quit: bool,
    pub terminals: HashMap<PaneId, PseudoTerminal>,
    pub active_pane: PaneId,
    pub file_browser: FileBrowserState,
    pub preview: PreviewState,
    pub show_help: bool,
}

impl App {
    pub fn new(config: Config, session: SessionState) -> Self {
        let rows = 24; 
        let cols = 80;

        let file_browser = FileBrowserState::new();
        let cwd = file_browser.current_dir.clone();

        let mut terminals = HashMap::new();

        // 1. Claude Code
        let claude_cmd = vec!["/bin/bash".to_string(), "-c".to_string(), "echo 'Claude Code PTY'; exec bash".to_string()];
        if let Ok(pty) = PseudoTerminal::new(&claude_cmd, rows, cols, &cwd) {
             terminals.insert(PaneId::Claude, pty);
        }

        // 2. LazyGit
        let lazygit_cmd = vec!["lazygit".to_string()];
        if let Ok(pty) = PseudoTerminal::new(&lazygit_cmd, rows, cols, &cwd) {
             terminals.insert(PaneId::LazyGit, pty);
        }

        // 3. User Terminal (from Config)
        let shell = &config.terminal.shell_path;
        let args = &config.terminal.shell_args;
        let mut cmd = vec![shell.clone()];
        cmd.extend(args.clone());
        
        if let Ok(pty) = PseudoTerminal::new(&cmd, rows, cols, &cwd) {
             terminals.insert(PaneId::Terminal, pty);
        }

        let mut app = Self {
            config,
            session,
            should_quit: false,
            terminals,
            active_pane: PaneId::Terminal,
            file_browser,
            preview: PreviewState::new(),
            show_help: false,
        };
        
        
        app.update_preview();
        app.sync_terminals();
        
        // Initial Clear
        for id in [PaneId::Terminal, PaneId::Claude] {
            if let Some(pty) = app.terminals.get_mut(&id) {
                 // Use Form Feed \x0c (Ctrl+L) to clear screen in most shells
                let _ = pty.write_input(b"\x0c");
            }
        }
        
        app
    }

    fn update_preview(&mut self) {
        if let Some(path) = self.file_browser.selected_file() {
            if self.preview.current_file.as_ref() != Some(&path) {
                self.preview.load_file(path);
            }
        }
    }

    pub fn run(mut self, mut terminal: DefaultTerminal) -> Result<()> {
        while !self.should_quit {
            terminal.draw(|frame| self.draw(frame))?;

            if event::poll(std::time::Duration::from_millis(16))? {
                match event::read()? {
                    Event::Mouse(mouse) => {
                         let size = terminal.size()?;
                         let area = Rect::new(0, 0, size.width, size.height);
                         let (files, preview, claude, lazygit, term, _footer) = ui::layout::compute_layout(area);
                         
                         let x = mouse.column;
                         let y = mouse.row;
                         
                         // Helper closure for hit testing
                         let is_inside = |r: Rect, x: u16, y: u16| -> bool {
                             x >= r.x && x < r.x + r.width && y >= r.y && y < r.y + r.height
                         };

                         match mouse.kind {
                            crossterm::event::MouseEventKind::Down(crossterm::event::MouseButton::Left) => {
                                if is_inside(files, x, y) { self.active_pane = PaneId::FileBrowser; }
                                else if is_inside(preview, x, y) { self.active_pane = PaneId::Preview; }
                                else if is_inside(claude, x, y) { self.active_pane = PaneId::Claude; }
                                else if is_inside(lazygit, x, y) { self.active_pane = PaneId::LazyGit; }
                                else if is_inside(term, x, y) { self.active_pane = PaneId::Terminal; }
                            }
                            crossterm::event::MouseEventKind::ScrollDown => {
                                if is_inside(files, x, y) { 
                                    self.file_browser.down(); 
                                    self.update_preview(); 
                                }
                                else if is_inside(preview, x, y) { self.preview.scroll_down(); }
                                else if is_inside(claude, x, y) { if let Some(pty) = self.terminals.get_mut(&PaneId::Claude) { pty.scroll_down(3); } }
                                else if is_inside(lazygit, x, y) { if let Some(pty) = self.terminals.get_mut(&PaneId::LazyGit) { pty.scroll_down(3); } }
                                else if is_inside(term, x, y) { if let Some(pty) = self.terminals.get_mut(&PaneId::Terminal) { pty.scroll_down(3); } }
                            }
                            crossterm::event::MouseEventKind::ScrollUp => {
                                if is_inside(files, x, y) { 
                                    self.file_browser.up(); 
                                    self.update_preview(); 
                                }
                                else if is_inside(preview, x, y) { self.preview.scroll_up(); }
                                else if is_inside(claude, x, y) { if let Some(pty) = self.terminals.get_mut(&PaneId::Claude) { pty.scroll_up(3); } }
                                else if is_inside(lazygit, x, y) { if let Some(pty) = self.terminals.get_mut(&PaneId::LazyGit) { pty.scroll_up(3); } }
                                else if is_inside(term, x, y) { if let Some(pty) = self.terminals.get_mut(&PaneId::Terminal) { pty.scroll_up(3); } }
                            }
                            _ => {}
                         }
                    }
                    Event::Key(key) => {
                        if key.kind == KeyEventKind::Press {
                        if self.show_help {
                            match key.code {
                                KeyCode::Esc | KeyCode::Char('?') | KeyCode::Char('q') => self.show_help = false,
                                _ => {}
                            }
                            // Consume all keys while help is open
                            continue;
                        }

                        // Global Keys
                        if key.code == KeyCode::Char('?') {
                            self.show_help = true;
                            continue;
                        }

                        // Global Focus Switching
                        match key.code {
                            KeyCode::F(1) => self.active_pane = PaneId::FileBrowser,
                            KeyCode::F(2) => self.active_pane = PaneId::Preview,
                            KeyCode::F(4) => self.active_pane = PaneId::Claude,
                            KeyCode::F(5) => self.active_pane = PaneId::LazyGit,
                            KeyCode::F(6) => self.active_pane = PaneId::Terminal,
                            // QUIT: Ctrl+Q
                            KeyCode::Char('q') if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) => {
                                self.should_quit = true;
                            }
                             _ => {
                                // Pane specific handling
                                match self.active_pane {
                                    PaneId::FileBrowser => {
                                        match key.code {
                                            KeyCode::Down | KeyCode::Char('j') => {
                                                self.file_browser.down();
                                                self.update_preview();
                                            }
                                            KeyCode::Up | KeyCode::Char('k') => {
                                                self.file_browser.up();
                                                self.update_preview();
                                            }
                                            KeyCode::Enter | KeyCode::Right | KeyCode::Char('l') => {
                                                if let Some(_path) = self.file_browser.enter_selected() {
                                                    // File opened
                                                } else {
                                                    self.update_preview();
                                                    self.sync_terminals();
                                                }
                                            }
                                            KeyCode::Backspace | KeyCode::Left | KeyCode::Char('h') => {
                                                self.file_browser.go_parent();
                                                self.update_preview();
                                                self.sync_terminals();
                                            }
                                            // Allow single q to quit if in browser
                                            KeyCode::Char('q') => {
                                                 self.should_quit = true; 
                                            }
                                            _ => {}
                                        }
                                    }

                                    PaneId::Preview => {
                                        match key.code {
                                            KeyCode::Down | KeyCode::Char('j') => self.preview.scroll_down(),
                                            KeyCode::Up | KeyCode::Char('k') => self.preview.scroll_up(),
                                            _ => {}
                                        }
                                    }
                                    PaneId::Terminal | PaneId::Claude | PaneId::LazyGit => {
                                        if let Some(pty) = self.terminals.get_mut(&self.active_pane) {
                                            // Scroll Handling
                                            if key.modifiers.contains(crossterm::event::KeyModifiers::SHIFT) {
                                                match key.code {
                                                    KeyCode::PageUp => { pty.scroll_up(10); continue; }
                                                    KeyCode::PageDown => { pty.scroll_down(10); continue; }
                                                    KeyCode::Up => { pty.scroll_up(1); continue; }
                                                    KeyCode::Down => { pty.scroll_down(1); continue; }
                                                    _ => {}
                                                }
                                            }

                                            if let Some(bytes) = crate::input::map_key_to_pty(key) {
                                                let _ = pty.write_input(&bytes);
                                            }
                                        }
                                    }
        
                                }
                            }
                        }
                    }
                } // End Key
                _ => {}
            } // End Match
            }
        }
        Ok(())
    }

    fn draw(&mut self, frame: &mut Frame) {
        let area = frame.area();
        let (files, preview, claude, lazygit, terminal, footer) = ui::layout::compute_layout(area);

        // Helper to resize PTY
        // We need to account for borders (1px each side => -2)
        // Ensure strictly positive
        let resize_pty = |terminals: &mut HashMap<PaneId, PseudoTerminal>, id: PaneId, rect: Rect| {
            if let Some(pty) = terminals.get_mut(&id) {
                let w = rect.width.saturating_sub(2);
                let h = rect.height.saturating_sub(2);
                if w > 0 && h > 0 {
                    let _ = pty.resize(h, w);
                }
            }
        };

        resize_pty(&mut self.terminals, PaneId::Claude, claude);
        resize_pty(&mut self.terminals, PaneId::LazyGit, lazygit);
        resize_pty(&mut self.terminals, PaneId::Terminal, terminal);

        ui::file_browser::render(frame, files, &mut self.file_browser, self.active_pane == PaneId::FileBrowser);
        ui::preview::render(frame, preview, &self.preview, self.active_pane == PaneId::Preview);
        
        ui::terminal_pane::render(frame, claude, PaneId::Claude, self);
        ui::terminal_pane::render(frame, lazygit, PaneId::LazyGit, self);
        ui::terminal_pane::render(frame, terminal, PaneId::Terminal, self);

        frame.render_widget(ui::footer::Footer, footer);

        if self.show_help {
            ui::help::render(frame);
        }
    }

    fn sync_terminals(&mut self) {
        let path_str = self.file_browser.current_dir.to_string_lossy().to_string();
        let esc_path = path_str.replace("\"", "\\\"");
        let cmd = format!("cd \"{}\"\r", esc_path);

        for id in [PaneId::Terminal, PaneId::Claude] {
            if let Some(pty) = self.terminals.get_mut(&id) {
                let _ = pty.write_input(cmd.as_bytes());
            }
        }
    }
}
