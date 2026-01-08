use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::{layout::Rect, DefaultTerminal, Frame};
use std::collections::HashMap;

use crate::config::Config;
use crate::session::SessionState;
use crate::ui;
use crate::types::{EditorMode, PaneId, TerminalSelection, DragState, HelpState, MouseSelection};
use crate::terminal::PseudoTerminal;
use std::borrow::Cow;
use std::path::Path;
use shell_escape::escape;
use crate::ui::file_browser::FileBrowserState;
use crate::ui::preview::PreviewState;
use crate::ui::syntax::SyntaxManager;

use crate::ui::menu::MenuBar;
use crate::ui::dialog::Dialog;
use crate::ui::fuzzy_finder::FuzzyFinder;
use crate::ui::settings::SettingsState;
use crate::ui::about::AboutState;
use crate::setup::wizard::WizardState;
use crate::browser;

pub struct App {
    pub config: Config,
    pub session: SessionState,
    pub should_quit: bool,
    pub terminals: HashMap<PaneId, PseudoTerminal>,
    pub active_pane: PaneId,
    pub file_browser: FileBrowserState,
    pub preview: PreviewState,
    pub help: HelpState,
    pub show_terminal: bool,
    pub show_lazygit: bool,
    pub last_refresh: std::time::Instant,
    pub menu: MenuBar,
    pub dialog: Dialog,
    pub fuzzy_finder: FuzzyFinder,
    pub syntax_manager: SyntaxManager,
    pub wizard: WizardState,
    pub settings: SettingsState,
    pub about: AboutState,
    // Claude PTY error tracking
    pub claude_error: Option<String>,
    pub claude_command_used: String,
    // Startup prefix dialog
    pub claude_startup: ui::claude_startup::ClaudeStartupState,
    // Double-click tracking
    last_click_time: std::time::Instant,
    last_click_idx: Option<usize>,
    // Terminal line selection for copying to Claude
    pub terminal_selection: TerminalSelection,
    // Drag and drop state for file paths
    pub drag_state: DragState,
    // Mouse-based text selection in terminal panes
    pub mouse_selection: MouseSelection,
}

impl App {
    pub fn new(config: Config, session: SessionState) -> Self {
        let rows = 24; 
        let cols = 80;

        let file_browser = FileBrowserState::new(config.file_browser.show_hidden);
        let cwd = file_browser.current_dir.clone();

        let mut terminals = HashMap::new();
        let mut claude_error: Option<String> = None;

        // 1. Claude Pane - uses shell from pty config (same as terminal)
        // User starts claude manually when ready
        let claude_cmd = if config.pty.claude_command.is_empty() {
            // Default: use the same shell as Terminal pane
            let mut cmd = vec![config.terminal.shell_path.clone()];
            cmd.extend(config.terminal.shell_args.clone());
            cmd
        } else {
            config.pty.claude_command.clone()
        };
        let claude_command_str = claude_cmd.join(" ");
        match PseudoTerminal::new(&claude_cmd, rows, cols, &cwd) {
            Ok(pty) => {
                terminals.insert(PaneId::Claude, pty);
            }
            Err(e) => {
                claude_error = Some(format!(
                    "Failed to start shell\n\nCommand: {}\n\nError: {}",
                    claude_command_str, e
                ));
            }
        }

        // 2. LazyGit (from Config)
        let lazygit_cmd = if config.pty.lazygit_command.is_empty() {
            vec!["lazygit".to_string()]
        } else {
            config.pty.lazygit_command.clone()
        };
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

        let syntax_manager = SyntaxManager::new();

        // Check if wizard should open (first run)
        let should_open_wizard = !config.setup.wizard_completed;

        let mut app = Self {
            config,
            session,
            should_quit: false,
            terminals,
            active_pane: PaneId::FileBrowser,
            file_browser,
            preview: PreviewState::new(),
            help: HelpState::default(),
            show_terminal: false,
            show_lazygit: false,
            last_refresh: std::time::Instant::now(),
            menu: MenuBar::default(),
            dialog: Dialog::default(),
            fuzzy_finder: FuzzyFinder::default(),
            syntax_manager,
            wizard: WizardState::new(),
            settings: SettingsState::new(),
            about: AboutState::default(),
            claude_error,
            claude_command_used: claude_command_str,
            claude_startup: ui::claude_startup::ClaudeStartupState::default(),
            last_click_time: std::time::Instant::now(),
            last_click_idx: None,
            terminal_selection: TerminalSelection::default(),
            drag_state: DragState::default(),
            mouse_selection: MouseSelection::default(),
        };

        // Open wizard on first run
        if should_open_wizard {
            app.wizard.open();
        }
        
        
        app.update_preview();

        // Initial cd for Terminal only (Claude should not receive early commands)
        app.sync_terminals_initial();

        // Initial Clear - ONLY for Terminal pane (not Claude, which needs time to start)
        if let Some(pty) = app.terminals.get_mut(&PaneId::Terminal) {
            let _ = pty.write_input(b"\x0c");
        }

        app
    }

    fn update_preview(&mut self) {
        if let Some(path) = self.file_browser.selected_file() {
            if self.preview.current_file.as_ref() != Some(&path) {
                self.preview.load_file(path, &self.syntax_manager);
            }
        }
    }

    pub fn run(mut self, mut terminal: DefaultTerminal) -> Result<()> {
        while !self.should_quit {
            // Check for exited PTYs and restart them with a shell
            self.check_and_restart_exited_ptys();

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
                                // Block all background interaction when any modal is open
                                // About dialog
                                if self.about.visible {
                                    if let Some(popup) = self.about.popup_area {
                                        if is_inside(popup, x, y) {
                                            self.about.handle_click(x, y);
                                        } else {
                                            self.about.close();
                                        }
                                    }
                                    continue;
                                }

                                // Help popup - click inside to interact, outside to close
                                if self.help.visible {
                                    if !self.help.contains(x, y) {
                                        self.help.close();
                                    }
                                    continue;
                                }

                                // Settings menu - click outside closes it
                                if self.settings.visible {
                                    self.settings.close();
                                    continue;
                                }

                                // Dialog (Confirm/Input) - handle button clicks
                                if self.dialog.is_active() {
                                    // Check if Yes or No button was clicked
                                    if let Some(result) = self.dialog.check_button_click(x, y) {
                                        match result {
                                            ui::dialog::ConfirmResult::Yes => {
                                                if let Some(action) = self.dialog.get_action() {
                                                    self.dialog.close();
                                                    self.execute_dialog_action(action, None);
                                                }
                                            }
                                            ui::dialog::ConfirmResult::No => {
                                                self.dialog.close();
                                            }
                                        }
                                    }
                                    // Block all other clicks when dialog is active
                                    continue;
                                }

                                // Fuzzy finder - click outside closes it
                                if self.fuzzy_finder.visible {
                                    self.fuzzy_finder.close();
                                    continue;
                                }

                                // Claude startup dialog - click outside closes and focuses Claude
                                if self.claude_startup.visible {
                                    self.claude_startup.close();
                                    self.active_pane = PaneId::Claude;
                                    continue;
                                }

                                // Setup wizard - block all background clicks
                                if self.wizard.visible {
                                    continue;
                                }

                                // Menu popup - click outside closes it
                                if self.menu.visible {
                                    self.menu.visible = false;
                                    continue;
                                }

                                if is_inside(files, x, y) {
                                    self.active_pane = PaneId::FileBrowser;
                                    // Calculate which item was clicked (account for border)
                                    let relative_y = y.saturating_sub(files.y + 1); // +1 for border
                                    let idx = relative_y as usize;
                                    if idx < self.file_browser.entries.len() {
                                        // Start drag with the file path
                                        let entry = &self.file_browser.entries[idx];
                                        self.drag_state.start(entry.path.clone(), x, y);
                                        // Check for double-click (same item within 300ms)
                                        let now = std::time::Instant::now();
                                        let is_double_click = self.last_click_idx == Some(idx)
                                            && now.duration_since(self.last_click_time).as_millis() < 300;

                                        // Update tracking for next click
                                        self.last_click_time = now;
                                        self.last_click_idx = Some(idx);

                                        // Check if editor has unsaved changes before switching
                                        let has_unsaved = self.preview.mode == EditorMode::Edit && self.preview.is_modified();

                                        if is_double_click {
                                            // Double-click: enter directory or open file
                                            let is_dir = self.file_browser.entries.get(idx).map(|e| e.is_dir).unwrap_or(false);
                                            if is_dir {
                                                if has_unsaved {
                                                    self.dialog.dialog_type = ui::dialog::DialogType::Confirm {
                                                        title: "Unsaved Changes".to_string(),
                                                        message: "Discard changes and enter directory?".to_string(),
                                                        action: ui::dialog::DialogAction::EnterDirectory { target_idx: idx },
                                                    };
                                                } else {
                                                    self.file_browser.list_state.select(Some(idx));
                                                    self.file_browser.enter_selected();
                                                    self.update_preview();
                                                    self.sync_terminals();
                                                }
                                            }
                                        } else {
                                            // Single click: just select (but check for unsaved changes)
                                            if has_unsaved {
                                                self.dialog.dialog_type = ui::dialog::DialogType::Confirm {
                                                    title: "Unsaved Changes".to_string(),
                                                    message: "Discard changes and switch file?".to_string(),
                                                    action: ui::dialog::DialogAction::SwitchFile { target_idx: idx },
                                                };
                                            } else {
                                                self.file_browser.list_state.select(Some(idx));
                                                self.update_preview();
                                            }
                                        }
                                    }
                                }
                                else if is_inside(preview, x, y) {
                                    // Alt+Click starts selection in Preview (Read-Only mode only)
                                    if mouse.modifiers.contains(crossterm::event::KeyModifiers::ALT)
                                       && self.preview.mode == crate::types::EditorMode::ReadOnly {
                                        self.mouse_selection.start(PaneId::Preview, y, preview);
                                    }
                                    // Click-to-position cursor in Edit mode
                                    else if self.preview.mode == crate::types::EditorMode::Edit {
                                        self.position_preview_cursor(preview, x, y);
                                    }
                                    self.active_pane = PaneId::Preview;
                                }
                                else if is_inside(claude, x, y) {
                                    // Show startup dialog if prefixes configured and not yet shown
                                    if !self.claude_startup.shown_this_session && !self.config.claude.startup_prefixes.is_empty() {
                                        self.claude_startup.open(self.config.claude.startup_prefixes.clone());
                                    } else {
                                        // Alt+Click starts mouse text selection
                                        if mouse.modifiers.contains(crossterm::event::KeyModifiers::ALT) {
                                            self.mouse_selection.start(PaneId::Claude, y, claude);
                                        }
                                        self.active_pane = PaneId::Claude;
                                    }
                                }
                                else if is_inside(lazygit, x, y) {
                                    // Alt+Click starts mouse text selection
                                    if mouse.modifiers.contains(crossterm::event::KeyModifiers::ALT) {
                                        self.mouse_selection.start(PaneId::LazyGit, y, lazygit);
                                    }
                                    self.active_pane = PaneId::LazyGit;
                                }
                                else if is_inside(term, x, y) {
                                    // Alt+Click starts mouse text selection
                                    if mouse.modifiers.contains(crossterm::event::KeyModifiers::ALT) {
                                        self.mouse_selection.start(PaneId::Terminal, y, term);
                                    }
                                    self.active_pane = PaneId::Terminal;
                                }
                                else if is_inside(footer_area, x, y) {
                                    // Use context-aware button positions
                                    let footer_x = x.saturating_sub(footer_area.x);
                                    let is_selection_mode = self.terminal_selection.active;
                                    let positions = ui::footer::get_context_button_positions(
                                        self.active_pane,
                                        self.preview.mode,
                                        is_selection_mode,
                                    );

                                    for (start, end, action) in positions {
                                        if footer_x >= start && footer_x < end {
                                            use ui::footer::FooterAction;
                                            match action {
                                                FooterAction::FocusFiles => self.active_pane = PaneId::FileBrowser,
                                                FooterAction::FocusPreview => self.active_pane = PaneId::Preview,
                                                FooterAction::Refresh => { self.file_browser.refresh(); self.update_preview(); }
                                                FooterAction::FocusClaude => {
                                                    if !self.claude_startup.shown_this_session && !self.config.claude.startup_prefixes.is_empty() {
                                                        self.claude_startup.open(self.config.claude.startup_prefixes.clone());
                                                    } else {
                                                        self.active_pane = PaneId::Claude;
                                                    }
                                                }
                                                FooterAction::ToggleGit => { self.show_lazygit = !self.show_lazygit; if self.show_lazygit { self.active_pane = PaneId::LazyGit; } }
                                                FooterAction::ToggleTerm => { self.show_terminal = !self.show_terminal; if self.show_terminal { self.active_pane = PaneId::Terminal; } }
                                                FooterAction::FuzzyFind => { self.fuzzy_finder.open(&self.file_browser.current_dir); }
                                                FooterAction::OpenFile => {
                                                    if let Some(path) = self.file_browser.selected_file() {
                                                        if browser::can_preview_in_browser(&path) {
                                                            let preview_path = if browser::is_markdown(&path) {
                                                                browser::markdown_to_html(&path).unwrap_or(path)
                                                            } else {
                                                                path
                                                            };
                                                            let _ = browser::open_file(&preview_path);
                                                        }
                                                    }
                                                }
                                                FooterAction::OpenFinder => { let _ = browser::open_in_file_manager(&self.file_browser.current_dir); }
                                                FooterAction::Settings => { let cfg = self.config.clone(); self.settings.open(&cfg); }
                                                FooterAction::About => self.about.open(),
                                                FooterAction::Help => self.help.open(),
                                                FooterAction::Edit => {
                                                    // Enter edit mode in Preview
                                                    if self.active_pane == PaneId::Preview {
                                                        self.preview.mode = crate::types::EditorMode::Edit;
                                                    }
                                                }
                                                FooterAction::StartSelect => {
                                                    // Start selection mode in current pane
                                                    if self.active_pane == PaneId::Preview {
                                                        self.terminal_selection.start(self.preview.scroll as usize, PaneId::Preview);
                                                    } else if matches!(self.active_pane, PaneId::Claude | PaneId::LazyGit | PaneId::Terminal) {
                                                        self.terminal_selection.start(0, self.active_pane);
                                                    }
                                                }
                                                FooterAction::Save => {
                                                    // Save in edit mode
                                                    if self.active_pane == PaneId::Preview && self.preview.mode == crate::types::EditorMode::Edit {
                                                        let _ = self.preview.save();
                                                    }
                                                }
                                                FooterAction::ExitEdit => {
                                                    // Exit edit mode
                                                    if self.active_pane == PaneId::Preview {
                                                        self.preview.mode = crate::types::EditorMode::ReadOnly;
                                                    }
                                                }
                                                FooterAction::Undo | FooterAction::Redo => {
                                                    // Undo/Redo handled by keyboard only
                                                }
                                                FooterAction::SelectDown | FooterAction::SelectUp => {
                                                    // Selection navigation not clickable
                                                }
                                                FooterAction::SelectCopy => {
                                                    // Copy selection
                                                    if self.terminal_selection.active {
                                                        self.copy_selection_to_claude();
                                                    }
                                                }
                                                FooterAction::SelectCancel => {
                                                    // Cancel selection
                                                    self.terminal_selection.active = false;
                                                }
                                                FooterAction::None => {}
                                            }
                                            break;
                                        }
                                    }
                                }
                            }
                            // Handle drag movement
                            crossterm::event::MouseEventKind::Drag(crossterm::event::MouseButton::Left) => {
                                // Block drag when any modal is open
                                if self.about.visible || self.help.visible || self.settings.visible
                                    || self.dialog.is_active() || self.fuzzy_finder.visible
                                    || self.claude_startup.visible || self.wizard.visible || self.menu.visible {
                                    continue;
                                }
                                // Handle mouse text selection in terminal panes
                                if self.mouse_selection.selecting {
                                    self.mouse_selection.update(y);
                                } else if self.drag_state.dragging {
                                    self.drag_state.update_position(x, y);
                                }
                            }
                            // Handle drag drop and mouse selection finish
                            crossterm::event::MouseEventKind::Up(crossterm::event::MouseButton::Left) => {
                                // Block drop when any modal is open
                                if self.about.visible || self.help.visible || self.settings.visible
                                    || self.dialog.is_active() || self.fuzzy_finder.visible
                                    || self.claude_startup.visible || self.wizard.visible || self.menu.visible {
                                    continue;
                                }
                                // Handle mouse text selection finish - convert to terminal selection
                                if self.mouse_selection.selecting {
                                    if let Some((start, end, pane)) = self.mouse_selection.finish() {
                                        // Enter keyboard selection mode with the mouse-selected range
                                        self.terminal_selection.active = true;
                                        self.terminal_selection.start_line = Some(start);
                                        self.terminal_selection.end_line = Some(end);
                                        self.terminal_selection.source_pane = Some(pane);
                                        self.active_pane = pane;
                                    }
                                } else if self.drag_state.dragging {
                                    // Determine drop target
                                    let drop_target = if is_inside(claude, x, y) {
                                        Some(PaneId::Claude)
                                    } else if is_inside(term, x, y) {
                                        Some(PaneId::Terminal)
                                    } else if is_inside(lazygit, x, y) {
                                        Some(PaneId::LazyGit)
                                    } else {
                                        None
                                    };

                                    if let Some(target_pane) = drop_target {
                                        if let Some(path) = self.drag_state.finish() {
                                            self.insert_path_at_cursor(target_pane, &path);
                                        }
                                    } else {
                                        self.drag_state.clear();
                                    }
                                }
                            }
                            crossterm::event::MouseEventKind::ScrollDown => {
                                // Block all background scroll when any modal is open
                                if self.about.visible {
                                    if let Some(popup) = self.about.popup_area {
                                        if is_inside(popup, x, y) {
                                            self.about.scroll_down();
                                        }
                                    }
                                    continue;
                                }

                                // Help popup scroll handling
                                if self.help.visible {
                                    if self.help.contains(x, y) {
                                        self.help.scroll_down(1);
                                    }
                                    continue;
                                }

                                // Block scroll for all other modals
                                if self.settings.visible || self.dialog.is_active()
                                    || self.fuzzy_finder.visible || self.claude_startup.visible
                                    || self.wizard.visible || self.menu.visible {
                                    continue;
                                }

                                if is_inside(files, x, y) {
                                    self.file_browser.down();
                                    self.update_preview();
                                }
                                else if is_inside(preview, x, y) {
                                    // In Edit mode, move TextArea cursor; in ReadOnly mode, scroll
                                    if self.preview.mode == crate::types::EditorMode::Edit {
                                        if let Some(editor) = &mut self.preview.editor {
                                            for _ in 0..3 {
                                                editor.move_cursor(tui_textarea::CursorMove::Down);
                                            }
                                        }
                                    } else {
                                        self.preview.scroll_down();
                                    }
                                }
                                else if is_inside(claude, x, y) { if let Some(pty) = self.terminals.get_mut(&PaneId::Claude) { pty.scroll_down(3); } }
                                else if is_inside(lazygit, x, y) { if let Some(pty) = self.terminals.get_mut(&PaneId::LazyGit) { pty.scroll_down(3); } }
                                else if is_inside(term, x, y) { if let Some(pty) = self.terminals.get_mut(&PaneId::Terminal) { pty.scroll_down(3); } }
                            }
                            crossterm::event::MouseEventKind::ScrollUp => {
                                // Block all background scroll when any modal is open
                                if self.about.visible {
                                    if let Some(popup) = self.about.popup_area {
                                        if is_inside(popup, x, y) {
                                            self.about.scroll_up();
                                        }
                                    }
                                    continue;
                                }

                                // Help popup scroll handling
                                if self.help.visible {
                                    if self.help.contains(x, y) {
                                        self.help.scroll_up(1);
                                    }
                                    continue;
                                }

                                // Block scroll for all other modals
                                if self.settings.visible || self.dialog.is_active()
                                    || self.fuzzy_finder.visible || self.claude_startup.visible
                                    || self.wizard.visible || self.menu.visible {
                                    continue;
                                }

                                if is_inside(files, x, y) {
                                    self.file_browser.up();
                                    self.update_preview();
                                }
                                else if is_inside(preview, x, y) {
                                    // In Edit mode, move TextArea cursor; in ReadOnly mode, scroll
                                    if self.preview.mode == crate::types::EditorMode::Edit {
                                        if let Some(editor) = &mut self.preview.editor {
                                            for _ in 0..3 {
                                                editor.move_cursor(tui_textarea::CursorMove::Up);
                                            }
                                        }
                                    } else {
                                        self.preview.scroll_up();
                                    }
                                }
                                else if is_inside(claude, x, y) { if let Some(pty) = self.terminals.get_mut(&PaneId::Claude) { pty.scroll_up(3); } }
                                else if is_inside(lazygit, x, y) { if let Some(pty) = self.terminals.get_mut(&PaneId::LazyGit) { pty.scroll_up(3); } }
                                else if is_inside(term, x, y) { if let Some(pty) = self.terminals.get_mut(&PaneId::Terminal) { pty.scroll_up(3); } }
                            }
                            _ => {}
                         }
                    }
                    Event::Key(key) => {
                        if key.kind == KeyEventKind::Press {
                        
                        // Fuzzy finder handling (highest priority)
                        if self.fuzzy_finder.visible {
                            match key.code {
                                KeyCode::Esc => self.fuzzy_finder.close(),
                                KeyCode::Enter => {
                                    if let Some(selected) = self.fuzzy_finder.selected() {
                                        let full_path = self.fuzzy_finder.base_dir.join(&selected);
                                        // Navigate to file's directory and select it
                                        if let Some(parent) = full_path.parent() {
                                            self.file_browser.current_dir = parent.to_path_buf();
                                            self.file_browser.load_directory();
                                            // Try to select the file
                                            let file_name = full_path.file_name().map(|n| n.to_string_lossy().to_string());
                                            if let Some(name) = file_name {
                                                for (i, entry) in self.file_browser.entries.iter().enumerate() {
                                                    if entry.name == name {
                                                        self.file_browser.list_state.select(Some(i));
                                                        break;
                                                    }
                                                }
                                            }
                                            self.update_preview();
                                            self.sync_terminals();
                                        }
                                        self.fuzzy_finder.close();
                                    }
                                }
                                KeyCode::Up => self.fuzzy_finder.prev(),
                                KeyCode::Down => self.fuzzy_finder.next(),
                                KeyCode::Backspace => self.fuzzy_finder.pop_char(),
                                KeyCode::Char(c) => self.fuzzy_finder.push_char(c),
                                _ => {}
                            }
                            continue;
                        }

                        // Wizard handling (high priority)
                        if self.wizard.visible {
                            self.handle_wizard_input(key.code, key.modifiers);
                            continue;
                        }

                        // Settings handling (high priority)
                        if self.settings.visible {
                            self.handle_settings_input(key.code, key.modifiers);
                            continue;
                        }

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
                        
                        // About dialog handling
                        if self.about.visible {
                            match key.code {
                                KeyCode::Esc | KeyCode::F(10) | KeyCode::Char('q') => self.about.close(),
                                KeyCode::Up | KeyCode::Char('k') => self.about.scroll_up(),
                                KeyCode::Down | KeyCode::Char('j') => self.about.scroll_down(),
                                _ => {}
                            }
                            continue;
                        }

                        if self.help.visible {
                            match key.code {
                                KeyCode::Esc | KeyCode::F(12) | KeyCode::Char('q') => self.help.close(),
                                KeyCode::Up | KeyCode::Char('k') => self.help.scroll_up(1),
                                KeyCode::Down | KeyCode::Char('j') => self.help.scroll_down(1),
                                KeyCode::PageUp => self.help.page_up(),
                                KeyCode::PageDown => self.help.page_down(),
                                KeyCode::Home | KeyCode::Char('g') => self.help.scroll_to_top(),
                                KeyCode::End | KeyCode::Char('G') => self.help.scroll_to_bottom(),
                                _ => {}
                            }
                            // Consume all keys while help is open
                            continue;
                        }

                        // Global Keys - F10/F12 work everywhere
                        if key.code == KeyCode::F(12) {
                            self.help.open();
                            continue;
                        }

                        if key.code == KeyCode::F(10) {
                            self.about.open();
                            continue;
                        }

                        // Context-specific shortcuts (only in non-terminal panes)
                        // '?' for help - only in FileBrowser or Preview (read-only)
                        if key.code == KeyCode::Char('?')
                            && matches!(self.active_pane, PaneId::FileBrowser | PaneId::Preview)
                            && self.preview.mode != EditorMode::Edit {
                            self.help.open();
                            continue;
                        }

                        // 'i' for about - only in FileBrowser (not Preview, as 'i' is common text)
                        if key.code == KeyCode::Char('i')
                            && self.active_pane == PaneId::FileBrowser {
                            self.about.open();
                            continue;
                        }
                        
                        if key.code == KeyCode::F(9) {
                            self.menu.toggle();
                            continue;
                        }
                        
                        // Ctrl+P: Open fuzzy finder
                        if key.code == KeyCode::Char('p') && key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) {
                            self.fuzzy_finder.open(&self.file_browser.current_dir);
                            continue;
                        }

                        // Ctrl+,: Open settings
                        if key.code == KeyCode::Char(',') && key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) {
                            self.settings.open(&self.config);
                            continue;
                        }

                        // Ctrl+Shift+W: Re-run setup wizard
                        if key.code == KeyCode::Char('W') && key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL | crossterm::event::KeyModifiers::SHIFT) {
                            self.wizard.open();
                            continue;
                        }

                        // Claude startup dialog handling (high priority)
                        if self.claude_startup.visible {
                            match key.code {
                                KeyCode::Esc => {
                                    self.claude_startup.close();
                                    self.active_pane = PaneId::Claude;
                                }
                                KeyCode::Enter => {
                                    if let Some(prefix) = self.claude_startup.selected_prefix() {
                                        if !prefix.is_empty() {
                                            if let Some(pty) = self.terminals.get_mut(&PaneId::Claude) {
                                                let cmd = format!("{}\n", prefix);
                                                let _ = pty.write_input(cmd.as_bytes());
                                            }
                                        }
                                    }
                                    self.claude_startup.close();
                                    self.active_pane = PaneId::Claude;
                                }
                                KeyCode::Up | KeyCode::Char('k') => self.claude_startup.prev(),
                                KeyCode::Down | KeyCode::Char('j') => self.claude_startup.next(),
                                _ => {}
                            }
                            continue;
                        }

                        // Global Focus Switching
                        match key.code {
                            KeyCode::F(1) => self.active_pane = PaneId::FileBrowser,
                            KeyCode::F(2) => self.active_pane = PaneId::Preview,
                            KeyCode::F(3) => { self.file_browser.refresh(); self.update_preview(); }
                            KeyCode::F(4) => {
                                // Show startup dialog if prefixes configured and not yet shown
                                if !self.claude_startup.shown_this_session && !self.config.claude.startup_prefixes.is_empty() {
                                    self.claude_startup.open(self.config.claude.startup_prefixes.clone());
                                } else {
                                    self.active_pane = PaneId::Claude;
                                }
                            }
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
                                            // Open file in browser/external viewer
                                            KeyCode::Char('o') => {
                                                if let Some(path) = self.file_browser.selected_file() {
                                                    if browser::can_preview_in_browser(&path) {
                                                        let preview_path = if browser::is_markdown(&path) {
                                                            browser::markdown_to_html(&path).unwrap_or(path)
                                                        } else {
                                                            path
                                                        };
                                                        let _ = browser::open_file(&preview_path);
                                                    }
                                                }
                                            }
                                            // Open current directory in file manager
                                            KeyCode::Char('O') => {
                                                let _ = browser::open_in_file_manager(&self.file_browser.current_dir);
                                            }
                                            // Allow single q to quit if in browser
                                            KeyCode::Char('q') => {
                                                 self.should_quit = true;
                                            }
                                            // Toggle hidden files visibility
                                            KeyCode::Char('.') => {
                                                self.file_browser.show_hidden = !self.file_browser.show_hidden;
                                                self.file_browser.refresh();
                                                self.update_preview();
                                            }
                                            _ => {}
                                        }
                                    }

                                    PaneId::Preview => {
                                        // Edit mode handling
                                        if self.preview.mode == EditorMode::Edit {
                                            // Check for Ctrl+S (save) - handle both modifier and control char
                                            let is_ctrl_s = (key.code == KeyCode::Char('s') && key.modifiers.contains(KeyModifiers::CONTROL))
                                                || key.code == KeyCode::Char('\x13'); // Ctrl+S as control char

                                            if key.code == KeyCode::Esc {
                                                if self.preview.is_modified() {
                                                    // Show discard dialog
                                                    self.dialog.dialog_type = ui::dialog::DialogType::Confirm {
                                                        title: "Unsaved Changes".to_string(),
                                                        message: "Discard changes?".to_string(),
                                                        action: ui::dialog::DialogAction::DiscardEditorChanges,
                                                    };
                                                } else {
                                                    self.preview.exit_edit_mode(true);
                                                }
                                            } else if is_ctrl_s {
                                                if let Err(_e) = self.preview.save() {
                                                    // Could show error dialog here
                                                } else {
                                                    // Refresh highlighting after save
                                                    self.preview.refresh_highlighting(&self.syntax_manager);
                                                }
                                            } else {
                                                // Handle scrolling keys specially (TextArea moves cursor, not view)
                                                match key.code {
                                                    KeyCode::PageUp => {
                                                        if let Some(editor) = &mut self.preview.editor {
                                                            // Move cursor up by ~20 lines to simulate page scroll
                                                            for _ in 0..20 {
                                                                editor.move_cursor(tui_textarea::CursorMove::Up);
                                                            }
                                                        }
                                                    }
                                                    KeyCode::PageDown => {
                                                        if let Some(editor) = &mut self.preview.editor {
                                                            for _ in 0..20 {
                                                                editor.move_cursor(tui_textarea::CursorMove::Down);
                                                            }
                                                        }
                                                    }
                                                    _ => {
                                                        // Forward other keys to TextArea (handles Ctrl+Z, Ctrl+Y, etc.)
                                                        if let Some(editor) = &mut self.preview.editor {
                                                            editor.input(Event::Key(key));
                                                            self.preview.update_modified();
                                                            // Update syntax highlighting for edit mode
                                                            self.preview.update_edit_highlighting(&self.syntax_manager);
                                                        }
                                                    }
                                                }
                                            }
                                        } else {
                                            // Read-only mode - check for selection mode first
                                            if self.terminal_selection.active && self.terminal_selection.source_pane == Some(PaneId::Preview) {
                                                // Selection mode active in Preview
                                                match key.code {
                                                    KeyCode::Up | KeyCode::Char('k') => {
                                                        if let Some(end) = self.terminal_selection.end_line {
                                                            self.terminal_selection.extend(end.saturating_sub(1));
                                                        }
                                                        continue;
                                                    }
                                                    KeyCode::Down | KeyCode::Char('j') => {
                                                        if let Some(end) = self.terminal_selection.end_line {
                                                            self.terminal_selection.extend(end + 1);
                                                        }
                                                        continue;
                                                    }
                                                    KeyCode::Enter | KeyCode::Char('y') => {
                                                        self.copy_selection_to_claude();
                                                        self.terminal_selection.clear();
                                                        continue;
                                                    }
                                                    KeyCode::Esc => {
                                                        self.terminal_selection.clear();
                                                        continue;
                                                    }
                                                    _ => {}
                                                }
                                            } else {
                                                // Normal read-only mode (no selection)
                                                // Check for Ctrl+S to start selection mode
                                                let is_ctrl_s = (key.code == KeyCode::Char('s') && key.modifiers.contains(KeyModifiers::CONTROL))
                                                    || key.code == KeyCode::Char('\x13');
                                                if is_ctrl_s {
                                                    // Start keyboard selection at current scroll position
                                                    self.terminal_selection.start(self.preview.scroll as usize, PaneId::Preview);
                                                    continue;
                                                }

                                                match key.code {
                                                    KeyCode::Down | KeyCode::Char('j') => self.preview.scroll_down(),
                                                    KeyCode::Up | KeyCode::Char('k') => self.preview.scroll_up(),
                                                    KeyCode::PageDown => {
                                                        for _ in 0..10 { self.preview.scroll_down(); }
                                                    }
                                                    KeyCode::PageUp => {
                                                        for _ in 0..10 { self.preview.scroll_up(); }
                                                    }
                                                    KeyCode::Home => { self.preview.scroll = 0; }
                                                    KeyCode::End => {
                                                        let max = self.preview.highlighted_lines.len().saturating_sub(1) as u16;
                                                        self.preview.scroll = max;
                                                    }
                                                    KeyCode::Char('e') | KeyCode::Char('E') => {
                                                        self.preview.enter_edit_mode();
                                                    }
                                                    _ => {}
                                                }
                                            }
                                        }
                                    }
                                    PaneId::Terminal | PaneId::Claude | PaneId::LazyGit => {
                                        // Terminal selection mode handling
                                        if self.terminal_selection.active && self.terminal_selection.source_pane == Some(self.active_pane) {
                                            match key.code {
                                                KeyCode::Up | KeyCode::Char('k') => {
                                                    if let Some(end) = self.terminal_selection.end_line {
                                                        self.terminal_selection.extend(end.saturating_sub(1));
                                                    }
                                                    continue;
                                                }
                                                KeyCode::Down | KeyCode::Char('j') => {
                                                    if let Some(end) = self.terminal_selection.end_line {
                                                        self.terminal_selection.extend(end + 1);
                                                    }
                                                    continue;
                                                }
                                                KeyCode::Enter | KeyCode::Char('y') => {
                                                    self.copy_selection_to_claude();
                                                    self.terminal_selection.clear();
                                                    continue;
                                                }
                                                KeyCode::Esc => {
                                                    self.terminal_selection.clear();
                                                    continue;
                                                }
                                                _ => {
                                                    // Let other keys (like Ctrl+C) pass through to PTY
                                                }
                                            }
                                        }

                                        // Ctrl+S: Start terminal selection mode
                                        if key.code == KeyCode::Char('s') && key.modifiers.contains(KeyModifiers::CONTROL) {
                                            if let Some(pty) = self.terminals.get(&self.active_pane) {
                                                let cursor_row = pty.cursor_row() as usize;
                                                self.terminal_selection.start(cursor_row, self.active_pane);
                                            }
                                            continue;
                                        }

                                        if let Some(pty) = self.terminals.get(&self.active_pane) {
                                            // Check if PTY has exited and auto_restart is disabled
                                            if pty.has_exited() && !self.config.pty.auto_restart {
                                                // Manual restart on Enter
                                                if key.code == KeyCode::Enter {
                                                    self.restart_single_pty(self.active_pane);
                                                }
                                                continue;
                                            }
                                        }

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
                // Handle paste events from clipboard (VipeCoding, etc.)
                // Use bracketed paste mode to signal terminal that this is pasted text
                // This prevents shells from interpreting newlines as immediate commands
                Event::Paste(text) => {
                    match self.active_pane {
                        PaneId::Claude | PaneId::LazyGit | PaneId::Terminal => {
                            if let Some(pty) = self.terminals.get_mut(&self.active_pane) {
                                // Wrap in bracketed paste escape sequences
                                // \x1b[200~ = start paste, \x1b[201~ = end paste
                                let bracketed = format!("\x1b[200~{}\x1b[201~", text);
                                let _ = pty.write_input(bracketed.as_bytes());
                            }
                        }
                        PaneId::Preview => {
                            // Forward paste to editor in edit mode
                            if self.preview.mode == EditorMode::Edit {
                                if let Some(editor) = &mut self.preview.editor {
                                    editor.insert_str(&text);
                                    self.preview.update_modified();
                                    self.preview.update_edit_highlighting(&self.syntax_manager);
                                }
                            }
                        }
                        PaneId::FileBrowser => {
                            // Ignore paste in file browser
                        }
                    }
                }
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

        // Calculate Preview selection range (keyboard or mouse selection)
        let preview_selection_range = if self.terminal_selection.active
            && self.terminal_selection.source_pane == Some(PaneId::Preview) {
            self.terminal_selection.line_range()
        } else if self.mouse_selection.is_selecting_in(PaneId::Preview) {
            self.mouse_selection.line_range()
        } else {
            None
        };
        ui::preview::render(frame, preview, &self.preview, self.active_pane == PaneId::Preview, preview_selection_range);
        
        ui::terminal_pane::render(frame, claude, PaneId::Claude, self);
        ui::terminal_pane::render(frame, lazygit, PaneId::LazyGit, self);
        ui::terminal_pane::render(frame, terminal, PaneId::Terminal, self);

        let footer_widget = ui::footer::Footer {
            active_pane: self.active_pane,
            editor_mode: self.preview.mode,
            editor_modified: self.preview.modified,
            selection_mode: self.terminal_selection.active,
        };
        frame.render_widget(footer_widget, footer);

        if self.help.visible {
            ui::help::render(frame, &mut self.help);
        }

        if self.about.visible {
            ui::about::render(frame, area, &mut self.about);
        }

        if self.menu.visible {
            ui::menu::render(frame, area, &self.menu);
        }

        if self.dialog.is_active() {
            ui::dialog::render(frame, area, &mut self.dialog);
        }
        
        if self.fuzzy_finder.visible {
            ui::fuzzy_finder::render(frame, area, &mut self.fuzzy_finder);
        }

        if self.wizard.visible {
            ui::wizard_ui::render(frame, area, &self.wizard);
        }

        if self.settings.visible {
            ui::settings::render(frame, area, &self.settings);
        }

        if self.claude_startup.visible {
            ui::claude_startup::render(frame, area, &self.claude_startup);
        }

        // Render drag ghost on top of everything
        ui::drag_ghost::render(frame, &self.drag_state);
    }

    /// Initial sync: send cd to Terminal only (Claude should not receive early commands)
    fn sync_terminals_initial(&mut self) {
        let path_str = self.file_browser.current_dir.to_string_lossy();
        let escaped = escape(Cow::Borrowed(&path_str));
        let cmd = format!("cd {}\r", escaped);

        // Only sync to Terminal, NOT Claude (Claude needs time to start)
        if let Some(pty) = self.terminals.get_mut(&PaneId::Terminal) {
            let _ = pty.write_input(cmd.as_bytes());
        }
    }

    /// Sync directory to Terminal pane only (not Claude - Claude only gets cd at startup)
    fn sync_terminals(&mut self) {
        let path_str = self.file_browser.current_dir.to_string_lossy();
        let escaped = escape(Cow::Borrowed(&path_str));
        let cmd = format!("cd {}\r", escaped);

        // Only sync to Terminal, not Claude (Claude should keep its initial directory)
        if let Some(pty) = self.terminals.get_mut(&PaneId::Terminal) {
            let _ = pty.write_input(cmd.as_bytes());
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
            DialogAction::DiscardEditorChanges => {
                self.preview.exit_edit_mode(true); // true = discard changes
                self.preview.refresh_highlighting(&self.syntax_manager);
            }
            DialogAction::SwitchFile { target_idx } => {
                // Discard changes and switch to clicked file
                self.preview.exit_edit_mode(true);
                self.preview.refresh_highlighting(&self.syntax_manager);
                self.file_browser.list_state.select(Some(target_idx));
                self.update_preview();
            }
            DialogAction::EnterDirectory { target_idx } => {
                // Discard changes and enter the clicked directory
                self.preview.exit_edit_mode(true);
                self.preview.refresh_highlighting(&self.syntax_manager);
                self.file_browser.list_state.select(Some(target_idx));
                self.file_browser.enter_selected();
                self.update_preview();
                self.sync_terminals();
            }
        }
    }

    /// Handle wizard input
    fn handle_wizard_input(&mut self, code: KeyCode, _modifiers: KeyModifiers) {
        use crate::setup::wizard::WizardStep;

        // If editing a field
        if self.wizard.editing_field.is_some() {
            match code {
                KeyCode::Esc => self.wizard.cancel_editing(),
                KeyCode::Enter => self.wizard.finish_editing(),
                KeyCode::Backspace => { self.wizard.input_buffer.pop(); }
                KeyCode::Char(c) => self.wizard.input_buffer.push(c),
                _ => {}
            }
            return;
        }

        match code {
            KeyCode::Esc => {
                // On Welcome, Esc closes wizard; on other steps, go back
                if self.wizard.step == WizardStep::Welcome {
                    self.wizard.close();
                } else {
                    self.wizard.prev_step();
                }
            }
            KeyCode::Enter => {
                if self.wizard.step == WizardStep::Confirmation {
                    // Save config immediately when confirming
                    let new_config = self.wizard.generate_config();
                    self.config = new_config;
                    if let Err(e) = crate::config::save_config(&self.config) {
                        eprintln!("Failed to save config: {}", e);
                    }
                    self.wizard.next_step(); // Go to Complete
                } else if self.wizard.step == WizardStep::Complete {
                    // Just close on Complete step
                    self.wizard.close();
                } else if self.wizard.can_proceed() {
                    self.wizard.next_step();
                }
            }
            KeyCode::Tab | KeyCode::Right => {
                if self.wizard.can_proceed() {
                    self.wizard.next_step();
                }
            }
            KeyCode::BackTab | KeyCode::Left => {
                self.wizard.prev_step();
            }
            KeyCode::Up | KeyCode::Char('k') => {
                match self.wizard.step {
                    WizardStep::ShellSelection => {
                        if self.wizard.selected_shell_idx > 0 {
                            self.wizard.selected_shell_idx -= 1;
                        }
                    }
                    WizardStep::ClaudeConfig => {
                        if self.wizard.focused_field > 0 {
                            self.wizard.focused_field -= 1;
                        }
                    }
                    _ => {}
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                match self.wizard.step {
                    WizardStep::ShellSelection => {
                        if self.wizard.selected_shell_idx < self.wizard.available_shells.len().saturating_sub(1) {
                            self.wizard.selected_shell_idx += 1;
                        }
                    }
                    WizardStep::ClaudeConfig => {
                        if self.wizard.focused_field < 1 {
                            self.wizard.focused_field += 1;
                        }
                    }
                    _ => {}
                }
            }
            KeyCode::Char('e') | KeyCode::Char('E') => {
                // Edit in ClaudeConfig step
                if self.wizard.step == WizardStep::ClaudeConfig {
                    use crate::setup::wizard::WizardField;
                    let field = if self.wizard.focused_field == 0 {
                        WizardField::ClaudePath
                    } else {
                        WizardField::LazygitPath
                    };
                    self.wizard.start_editing(field);
                }
            }
            _ => {}
        }
    }

    /// Handle settings input
    fn handle_settings_input(&mut self, code: KeyCode, _modifiers: KeyModifiers) {
        // If editing a field
        if self.settings.editing.is_some() {
            match code {
                KeyCode::Esc => self.settings.cancel_editing(),
                KeyCode::Enter => self.settings.finish_editing(),
                KeyCode::Backspace => { self.settings.input_buffer.pop(); }
                KeyCode::Char(c) => self.settings.input_buffer.push(c),
                _ => {}
            }
            return;
        }

        match code {
            KeyCode::Esc => {
                self.settings.close();
            }
            KeyCode::Tab => {
                self.settings.next_category();
            }
            KeyCode::BackTab => {
                self.settings.prev_category();
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.settings.move_up();
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.settings.move_down();
            }
            KeyCode::Enter => {
                self.settings.toggle_or_select();
            }
            KeyCode::Char(' ') => {
                self.settings.toggle_or_select();
            }
            KeyCode::Char('s') | KeyCode::Char('S') => {
                // Save and close
                if self.settings.has_changes {
                    self.settings.apply_to_config(&mut self.config);
                    if let Err(e) = crate::config::save_config(&self.config) {
                        eprintln!("Failed to save config: {}", e);
                    }
                }
                self.settings.close();
            }
            _ => {}
        }
    }

    /// Copy selected lines to Claude pane as a code block (from terminal or preview)
    fn copy_selection_to_claude(&mut self) {
        use crate::filter::{filter_lines, FilterOptions};

        let Some((start, end)) = self.terminal_selection.line_range() else {
            return;
        };

        let Some(source_pane) = self.terminal_selection.source_pane else {
            return;
        };

        // Extract lines from source (terminal or preview)
        let lines = if source_pane == PaneId::Preview {
            // Extract lines from preview content
            let content_lines: Vec<String> = self.preview.content.lines().map(String::from).collect();
            if start > content_lines.len() || end > content_lines.len() {
                return;
            }
            content_lines[start..=end.min(content_lines.len().saturating_sub(1))].to_vec()
        } else if let Some(pty) = self.terminals.get(&source_pane) {
            pty.extract_lines(start, end)
        } else {
            return;
        };

        if lines.is_empty() {
            return;
        }

        // For preview, use syntax from file; for terminal, detect syntax
        let (formatted_lines, syntax_hint) = if source_pane == PaneId::Preview {
            // Use file extension for syntax hint
            let syntax = self.preview.syntax_name.as_deref().unwrap_or("");
            (lines, Some(syntax.to_lowercase()))
        } else {
            // Apply intelligent filtering for terminal output
            let filtered = filter_lines(lines, &FilterOptions::default());
            if filtered.lines.is_empty() {
                return;
            }
            (filtered.lines, filtered.syntax_hint)
        };

        // Format for Claude - wrap in markdown code block with syntax hint
        let syntax = syntax_hint.as_deref().unwrap_or("");
        let formatted = format!(
            "```{}\n{}\n```\n",
            syntax,
            formatted_lines.join("\n")
        );

        // Send to Claude PTY
        if let Some(claude_pty) = self.terminals.get_mut(&PaneId::Claude) {
            let _ = claude_pty.write_input(formatted.as_bytes());
        }
    }

    /// Position preview editor cursor based on mouse click coordinates
    fn position_preview_cursor(&mut self, area: Rect, click_x: u16, click_y: u16) {
        use tui_textarea::CursorMove;

        let Some(editor) = &mut self.preview.editor else { return };

        // Account for block border (1px on each side)
        let inner_x = area.x + 1;
        let inner_y = area.y + 1;
        let inner_width = area.width.saturating_sub(2);
        let inner_height = area.height.saturating_sub(2);

        // Check if click is within inner content area
        if click_x < inner_x || click_x >= inner_x + inner_width {
            return;
        }
        if click_y < inner_y || click_y >= inner_y + inner_height {
            return;
        }

        // Calculate relative position within content area
        let rel_x = (click_x - inner_x) as usize;
        let rel_y = (click_y - inner_y) as usize;

        // Calculate scroll offset based on current cursor position
        let (cursor_row, _) = editor.cursor();
        let visible_height = inner_height as usize;
        let scroll_offset = if cursor_row >= visible_height {
            cursor_row.saturating_sub(visible_height / 2)
        } else {
            0
        };

        // Calculate target row (accounting for scroll)
        let target_row = (rel_y + scroll_offset) as u16;
        let target_col = rel_x as u16;

        // Clamp to valid range
        let max_row = editor.lines().len().saturating_sub(1) as u16;
        let clamped_row = target_row.min(max_row);

        let line_len = editor.lines()
            .get(clamped_row as usize)
            .map(|l| l.len())
            .unwrap_or(0) as u16;
        let clamped_col = target_col.min(line_len);

        // Jump to calculated position
        editor.move_cursor(CursorMove::Jump(clamped_row, clamped_col));
    }

    /// Check for exited PTYs and restart them with a fresh shell
    fn check_and_restart_exited_ptys(&mut self) {
        // Skip if auto-restart is disabled
        if !self.config.pty.auto_restart {
            return;
        }

        let cwd = self.file_browser.current_dir.clone();
        let rows = 24;
        let cols = 80;

        // Check each terminal PTY
        let panes_to_restart: Vec<PaneId> = self.terminals
            .iter()
            .filter(|(_, pty)| pty.has_exited())
            .map(|(id, _)| *id)
            .collect();

        for pane_id in panes_to_restart {
            // Remove the old PTY
            self.terminals.remove(&pane_id);

            // Determine the command to restart based on pane type
            let cmd = match pane_id {
                PaneId::Claude => {
                    if self.config.pty.claude_command.is_empty() {
                        let mut cmd = vec![self.config.terminal.shell_path.clone()];
                        cmd.extend(self.config.terminal.shell_args.clone());
                        cmd
                    } else {
                        self.config.pty.claude_command.clone()
                    }
                }
                PaneId::LazyGit => {
                    if self.config.pty.lazygit_command.is_empty() {
                        vec!["lazygit".to_string()]
                    } else {
                        self.config.pty.lazygit_command.clone()
                    }
                }
                PaneId::Terminal => {
                    let mut cmd = vec![self.config.terminal.shell_path.clone()];
                    cmd.extend(self.config.terminal.shell_args.clone());
                    cmd
                }
                _ => continue, // Skip non-terminal panes
            };

            // Start a fresh shell/process
            if let Ok(new_pty) = PseudoTerminal::new(&cmd, rows, cols, &cwd) {
                self.terminals.insert(pane_id, new_pty);
            }
        }
    }

    /// Restart a single PTY (manual restart when auto_restart is disabled)
    fn restart_single_pty(&mut self, pane_id: PaneId) {
        let cwd = self.file_browser.current_dir.clone();
        let rows = 24;
        let cols = 80;

        // Remove the old PTY
        self.terminals.remove(&pane_id);

        // Determine the command to restart based on pane type
        let cmd = match pane_id {
            PaneId::Claude => {
                if self.config.pty.claude_command.is_empty() {
                    let mut cmd = vec![self.config.terminal.shell_path.clone()];
                    cmd.extend(self.config.terminal.shell_args.clone());
                    cmd
                } else {
                    self.config.pty.claude_command.clone()
                }
            }
            PaneId::LazyGit => {
                if self.config.pty.lazygit_command.is_empty() {
                    vec!["lazygit".to_string()]
                } else {
                    self.config.pty.lazygit_command.clone()
                }
            }
            PaneId::Terminal => {
                let mut cmd = vec![self.config.terminal.shell_path.clone()];
                cmd.extend(self.config.terminal.shell_args.clone());
                cmd
            }
            _ => return, // Skip non-terminal panes
        };

        // Start a fresh shell/process
        if let Ok(new_pty) = PseudoTerminal::new(&cmd, rows, cols, &cwd) {
            self.terminals.insert(pane_id, new_pty);
        }
    }

    /// Insert file path at cursor in target terminal pane
    fn insert_path_at_cursor(&mut self, target: PaneId, path: &Path) {
        if let Some(pty) = self.terminals.get_mut(&target) {
            let path_str = path.to_string_lossy();
            // Use shell-escape crate for proper escaping of special characters
            let escaped = escape(Cow::Borrowed(&path_str));

            // Write to PTY (no newline - just insert the path)
            let _ = pty.write_input(escaped.as_bytes());
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