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

use crate::ui::menu::MenuBar;
use crate::ui::dialog::Dialog;

pub struct App {
    pub config: Config,
    pub session: SessionState,
    pub should_quit: bool,
    pub terminals: HashMap<PaneId, PseudoTerminal>,
    pub active_pane: PaneId,
    pub file_browser: FileBrowserState,
    pub preview: PreviewState,
    pub show_help: bool,
    pub show_terminal: bool,
    pub show_lazygit: bool,
    pub last_refresh: std::time::Instant,
    pub menu: MenuBar,
    pub dialog: Dialog,
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
            active_pane: PaneId::FileBrowser,
            file_browser,
            preview: PreviewState::new(),
            show_help: false,
            show_terminal: false,
            show_lazygit: false,
            last_refresh: std::time::Instant::now(),
            menu: MenuBar::default(),
            dialog: Dialog::default(),
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
            // Auto-refresh file browser
            let refresh_interval = self.config.file_browser.auto_refresh_ms;
            if refresh_interval > 0 {
                let elapsed = self.last_refresh.elapsed().as_millis() as u64;
                if elapsed >= refresh_interval {
                    self.file_browser.refresh();
                    self.last_refresh = std::time::Instant::now();
                }
            }
            
            terminal.draw(|frame| self.draw(frame))?;

            if event::poll(std::time::Duration::from_millis(16))? {
                match event::read()? {
                    Event::Mouse(mouse) => {
                         let size = terminal.size()?;
                         let area = Rect::new(0, 0, size.width, size.height);
                         let (files, preview, claude, lazygit, term, footer_area) = ui::layout::compute_layout(area, self.show_terminal, self.show_lazygit);
                         
                         let x = mouse.column;
                         let y = mouse.row;
                         
                         // Helper closure for hit testing
                         let is_inside = |r: Rect, x: u16, y: u16| -> bool {
                             x >= r.x && x < r.x + r.width && y >= r.y && y < r.y + r.height
                         };

                         match mouse.kind {
                            crossterm::event::MouseEventKind::Down(crossterm::event::MouseButton::Left) => {
                                if is_inside(files, x, y) { 
                                    self.active_pane = PaneId::FileBrowser;
                                    // Calculate which item was clicked (account for border)
                                    let relative_y = y.saturating_sub(files.y + 1); // +1 for border
                                    let idx = relative_y as usize;
                                    if idx < self.file_browser.entries.len() {
                                        self.file_browser.list_state.select(Some(idx));
                                        self.update_preview();
                                    }
                                }
                                else if is_inside(preview, x, y) { self.active_pane = PaneId::Preview; }
                                else if is_inside(claude, x, y) { self.active_pane = PaneId::Claude; }
                                else if is_inside(lazygit, x, y) { self.active_pane = PaneId::LazyGit; }
                                else if is_inside(term, x, y) { self.active_pane = PaneId::Terminal; }
                                else if is_inside(footer_area, x, y) {
                                    // Use precise button positions
                                    let footer_x = x.saturating_sub(footer_area.x);
                                    let positions = ui::footer::get_button_positions();
                                    
                                    for (start, end, idx) in positions {
                                        if footer_x >= start && footer_x < end {
                                            match idx {
                                                0 => self.active_pane = PaneId::FileBrowser,  // F1 Files
                                                1 => self.active_pane = PaneId::Preview,       // F2 Preview
                                                2 => { self.file_browser.refresh(); self.update_preview(); } // F3 Refresh
                                                3 => self.active_pane = PaneId::Claude,        // F4 Claude
                                                4 => { self.show_lazygit = !self.show_lazygit; if self.show_lazygit { self.active_pane = PaneId::LazyGit; } } // F5 Git
                                                5 => { self.show_terminal = !self.show_terminal; if self.show_terminal { self.active_pane = PaneId::Terminal; } } // F6 Term
                                                6 => self.menu.toggle(),  // F9 Menu
                                                7 => self.show_help = true, // ? Help
                                                _ => {}
                                            }
                                            break;
                                        }
                                    }
                                }
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
                        
                        // Dialog handling (highest priority)
                        if self.dialog.is_active() {
                            match &mut self.dialog.dialog_type {
                                ui::dialog::DialogType::Input { value, action, .. } => {
                                    match key.code {
                                        KeyCode::Esc => self.dialog.close(),
                                        KeyCode::Enter => {
                                            let val = value.clone();
                                            let act = action.clone();
                                            self.dialog.close();
                                            self.execute_dialog_action(act, Some(val));
                                        }
                                        KeyCode::Backspace => { value.pop(); }
                                        KeyCode::Char(c) => { value.push(c); }
                                        _ => {}
                                    }
                                }
                                ui::dialog::DialogType::Confirm { action, .. } => {
                                    match key.code {
                                        KeyCode::Esc | KeyCode::Char('n') | KeyCode::Char('N') => self.dialog.close(),
                                        KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => {
                                            let act = action.clone();
                                            self.dialog.close();
                                            self.execute_dialog_action(act, None);
                                        }
                                        _ => {}
                                    }
                                }
                                ui::dialog::DialogType::None => {}
                            }
                            continue;
                        }
                        
                        // Menu handling
                        if self.menu.visible {
                            match key.code {
                                KeyCode::Esc => self.menu.visible = false,
                                KeyCode::Up | KeyCode::Char('k') => self.menu.prev(),
                                KeyCode::Down | KeyCode::Char('j') => self.menu.next(),
                                KeyCode::Enter => {
                                    let action = self.menu.action();
                                    self.menu.visible = false;
                                    self.handle_menu_action(action);
                                }
                                KeyCode::Char('n') => { self.menu.visible = false; self.handle_menu_action(ui::menu::MenuAction::NewFile); }
                                KeyCode::Char('r') => { self.menu.visible = false; self.handle_menu_action(ui::menu::MenuAction::RenameFile); }
                                KeyCode::Char('d') => { self.menu.visible = false; self.handle_menu_action(ui::menu::MenuAction::DeleteFile); }
                                KeyCode::Char('y') => { self.menu.visible = false; self.handle_menu_action(ui::menu::MenuAction::CopyAbsolutePath); }
                                KeyCode::Char('Y') => { self.menu.visible = false; self.handle_menu_action(ui::menu::MenuAction::CopyRelativePath); }
                                _ => {}
                            }
                            continue;
                        }
                        
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
                        
                        if key.code == KeyCode::F(9) {
                            self.menu.toggle();
                            continue;
                        }

                        // Global Focus Switching
                        match key.code {
                            KeyCode::F(1) => self.active_pane = PaneId::FileBrowser,
                            KeyCode::F(2) => self.active_pane = PaneId::Preview,
                            KeyCode::F(3) => { self.file_browser.refresh(); self.update_preview(); }
                            KeyCode::F(4) => self.active_pane = PaneId::Claude,
                            KeyCode::F(5) => {
                                self.show_lazygit = !self.show_lazygit;
                                if self.show_lazygit {
                                    self.active_pane = PaneId::LazyGit;
                                } else if self.active_pane == PaneId::LazyGit {
                                    self.active_pane = PaneId::Preview;
                                }
                            }
                            KeyCode::F(6) => {
                                self.show_terminal = !self.show_terminal;
                                if self.show_terminal {
                                    self.active_pane = PaneId::Terminal;
                                } else if self.active_pane == PaneId::Terminal {
                                    self.active_pane = PaneId::Preview;
                                }
                            }
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
        let (files, preview, claude, lazygit, terminal, footer) = ui::layout::compute_layout(area, self.show_terminal, self.show_lazygit);

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
        
        if self.menu.visible {
            ui::menu::render(frame, area, &self.menu);
        }
        
        if self.dialog.is_active() {
            ui::dialog::render(frame, area, &self.dialog);
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
    
    fn handle_menu_action(&mut self, action: ui::menu::MenuAction) {
        use ui::menu::MenuAction;
        use ui::dialog::{DialogType, DialogAction};
        
        match action {
            MenuAction::NewFile => {
                self.dialog.dialog_type = DialogType::Input {
                    title: "New File".to_string(),
                    value: String::new(),
                    action: DialogAction::NewFile,
                };
            }
            MenuAction::RenameFile => {
                if let Some(selected) = self.file_browser.selected_file() {
                    let name = selected.file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_default();
                    self.dialog.dialog_type = DialogType::Input {
                        title: "Rename".to_string(),
                        value: name,
                        action: DialogAction::RenameFile { old_path: selected },
                    };
                }
            }
            MenuAction::DeleteFile => {
                if let Some(selected) = self.file_browser.selected_file() {
                    if selected.file_name().map(|n| n.to_string_lossy()) != Some("..".into()) {
                        let name = selected.file_name()
                            .map(|n| n.to_string_lossy().to_string())
                            .unwrap_or_default();
                        self.dialog.dialog_type = DialogType::Confirm {
                            title: "Delete".to_string(),
                            message: format!("Delete '{}'?", name),
                            action: DialogAction::DeleteFile { path: selected },
                        };
                    }
                }
            }
            MenuAction::CopyAbsolutePath => {
                if let Some(selected) = self.file_browser.selected_file() {
                    let path = selected.to_string_lossy().to_string();
                    let encoded = base64_encode(&path);
                    print!("\x1b]52;c;{}\x07", encoded);
                }
            }
            MenuAction::CopyRelativePath => {
                if let Some(selected) = self.file_browser.selected_file() {
                    if let Ok(rel) = selected.strip_prefix(&self.file_browser.current_dir) {
                        let path = rel.to_string_lossy().to_string();
                        let encoded = base64_encode(&path);
                        print!("\x1b]52;c;{}\x07", encoded);
                    }
                }
            }
            MenuAction::None => {}
        }
    }
    
    fn execute_dialog_action(&mut self, action: ui::dialog::DialogAction, value: Option<String>) {
        use ui::dialog::DialogAction;
        
        match action {
            DialogAction::NewFile => {
                if let Some(name) = value {
                    if !name.is_empty() {
                        let new_path = self.file_browser.current_dir.join(&name);
                        let _ = std::fs::write(&new_path, "");
                        self.file_browser.refresh();
                    }
                }
            }
            DialogAction::RenameFile { old_path } => {
                if let Some(new_name) = value {
                    if !new_name.is_empty() {
                        let new_path = old_path.parent()
                            .map(|p| p.join(&new_name))
                            .unwrap_or_else(|| std::path::PathBuf::from(&new_name));
                        let _ = std::fs::rename(&old_path, &new_path);
                        self.file_browser.refresh();
                    }
                }
            }
            DialogAction::DeleteFile { path } => {
                if path.is_file() {
                    let _ = std::fs::remove_file(&path);
                } else if path.is_dir() {
                    let _ = std::fs::remove_dir_all(&path);
                }
                self.file_browser.refresh();
            }
        }
    }
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
