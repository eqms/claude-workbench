mod clipboard;
mod drawing;
mod file_ops;
mod git_ops;
mod keyboard;
mod mouse;
mod pty;
mod update;

use anyhow::Result;
use crossterm::event::{self, Event};
use ratatui::{layout::Rect, DefaultTerminal};
use std::collections::HashMap;

use crate::config::Config;
use crate::session::SessionState;
use crate::terminal::PseudoTerminal;
use crate::types::{
    BorderAreas, ClaudePermissionMode, DragState, GitRemoteCheckResult, GitRemoteState, HelpState,
    MouseSelection, PaneId, ResizeState, ScrollbarAreas, ScrollbarDragState, TerminalSelection,
};
use crate::ui;
use crate::ui::file_browser::FileBrowserState;
use crate::ui::preview::PreviewState;
use crate::ui::syntax::SyntaxManager;

use crate::setup::wizard::WizardState;
use crate::ui::about::AboutState;
use crate::ui::dialog::Dialog;
use crate::ui::fuzzy_finder::FuzzyFinder;
use crate::ui::menu::MenuBar;
use crate::ui::settings::SettingsState;
use crate::ui::update_dialog::{UpdateDialogAreas, UpdateDialogButton};
use crate::update::{UpdateCheckResult, UpdateResult, UpdateState};

/// Layout rectangles for all panes, computed once per event and passed to handlers.
pub(crate) struct LayoutRects {
    pub files: Rect,
    pub preview: Rect,
    pub claude: Rect,
    pub lazygit: Rect,
    pub terminal: Rect,
    pub footer: Rect,
}

/// Saved pane visibility state for preview maximize/restore
#[derive(Debug, Clone, Copy)]
pub struct SavedLayout {
    pub show_file_browser: bool,
    pub show_lazygit: bool,
    pub show_terminal: bool,
}

impl Default for SavedLayout {
    fn default() -> Self {
        Self {
            show_file_browser: true,
            show_lazygit: false,
            show_terminal: false,
        }
    }
}

pub struct App {
    pub config: Config,
    pub session: SessionState,
    pub should_quit: bool,
    pub should_restart: bool,
    pub terminals: HashMap<PaneId, PseudoTerminal>,
    pub active_pane: PaneId,
    pub file_browser: FileBrowserState,
    pub preview: PreviewState,
    pub help: HelpState,
    pub show_file_browser: bool,
    pub show_terminal: bool,
    pub show_lazygit: bool,
    pub show_preview: bool,
    // Preview maximize mode (F3 toggle)
    pub preview_maximized: bool,
    pub preview_saved_layout: SavedLayout,
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
    // Permission mode selection dialog
    pub permission_mode_dialog: ui::permission_mode::PermissionModeState,
    // Selected Claude permission mode
    pub claude_permission_mode: ClaudePermissionMode,
    // Whether Claude PTY is pending (waiting for permission mode selection)
    pub claude_pty_pending: bool,
    // Double-click tracking
    last_click_time: std::time::Instant,
    last_click_idx: Option<usize>,
    // Terminal line selection for copying to Claude
    pub terminal_selection: TerminalSelection,
    // Drag and drop state for file paths
    pub drag_state: DragState,
    // Mouse-based text selection in terminal panes
    pub mouse_selection: MouseSelection,
    // Git remote change detection state
    pub git_remote: GitRemoteState,
    // Receiver for async git remote check results
    pub git_check_receiver: Option<std::sync::mpsc::Receiver<GitRemoteCheckResult>>,
    // Self-update state
    pub update_state: UpdateState,
    // Receiver for async update check results
    pub update_check_receiver: Option<std::sync::mpsc::Receiver<UpdateCheckResult>>,
    // Receiver for async update results
    pub update_receiver: Option<std::sync::mpsc::Receiver<UpdateResult>>,
    // Update dialog button selection
    pub update_dialog_button: UpdateDialogButton,
    // Cached update dialog areas for mouse clicks
    pub update_dialog_areas: UpdateDialogAreas,
    // Fake version for testing updates (--fake-version CLI arg)
    pub fake_version: Option<String>,
    // Scrollbar drag support
    pub scrollbar_drag: ScrollbarDragState,
    pub scrollbar_areas: ScrollbarAreas,
    // Cached preview pane width for horizontal scroll calculations
    pub preview_width: u16,
    // Interactive pane resizing state
    pub resize_state: ResizeState,
    pub border_areas: BorderAreas,
    // Flash state for autosave "✓ SAVED" indicator (2s duration)
    pub last_autosave_time: Option<std::time::Instant>,
    // Flash state for F9 "✓ N Zeilen" copy indicator (2s duration)
    pub last_copy_time: Option<std::time::Instant>,
    pub copy_flash_lines: usize,
    // Pending /remote-control slash command (sent after 4s startup delay)
    pub remote_control_send_time: Option<std::time::Instant>,
    // Temp files created for browser previews (cleaned up on exit)
    pub temp_preview_files: Vec<std::path::PathBuf>,
    // Export format chooser (Ctrl+X on Markdown files)
    pub export_chooser: crate::types::ExportChooserState,
}

impl App {
    pub fn new(config: Config, session: SessionState, fake_version: Option<String>) -> Self {
        let rows = 24;
        let cols = 80;

        let file_browser = FileBrowserState::new(config.file_browser.show_hidden);
        let cwd = file_browser.current_dir.clone();

        let mut terminals = HashMap::new();
        let mut claude_error: Option<String> = None;
        let mut claude_pty_pending = false;
        let mut permission_mode_dialog = ui::permission_mode::PermissionModeState::default();
        let mut claude_permission_mode = ClaudePermissionMode::Default;
        let mut remote_control_time: Option<std::time::Instant> = None;
        // Determine if we should show permission mode dialog
        // Don't show if wizard needs to run first (first-time setup)
        let should_show_permission_dialog =
            config.claude.show_permission_dialog && config.setup.wizard_completed;

        // 1. Claude Pane - delayed init if permission dialog should be shown
        let claude_command_str;
        if should_show_permission_dialog {
            // Delay Claude PTY creation until permission mode is selected
            claude_pty_pending = true;
            permission_mode_dialog.open_with_default(
                config.claude.default_permission_mode,
                config.claude.remote_control,
            );
            claude_command_str = String::new();
        } else {
            // Use configured default permission mode or Default
            claude_permission_mode = config
                .claude
                .default_permission_mode
                .unwrap_or(ClaudePermissionMode::Default);

            // Build Claude command with permission mode
            let claude_cmd = Self::build_claude_command(&config, claude_permission_mode);
            claude_command_str = claude_cmd.join(" ");

            match PseudoTerminal::new(&claude_cmd, rows, cols, &cwd) {
                Ok(pty) => {
                    terminals.insert(PaneId::Claude, pty);
                    if config.claude.remote_control {
                        remote_control_time = Some(std::time::Instant::now());
                    }
                }
                Err(e) => {
                    claude_error = Some(format!(
                        "Failed to start shell\n\nCommand: {}\n\nError: {}",
                        claude_command_str, e
                    ));
                }
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

        // Read pane visibility from config (before config is moved into struct)
        let show_file_browser = config.ui.show_file_browser;
        let show_terminal = config.ui.show_terminal;
        let show_lazygit = config.ui.show_lazygit;
        let show_preview = config.ui.show_preview;

        let mut app = Self {
            config,
            session,
            should_quit: false,
            should_restart: false,
            terminals,
            active_pane: PaneId::Claude,
            file_browser,
            preview: PreviewState::new(),
            help: HelpState::default(),
            show_file_browser,
            show_terminal,
            show_lazygit,
            show_preview,
            preview_maximized: false,
            preview_saved_layout: SavedLayout::default(),
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
            permission_mode_dialog,
            claude_permission_mode,
            claude_pty_pending,
            last_click_time: std::time::Instant::now(),
            last_click_idx: None,
            terminal_selection: TerminalSelection::default(),
            drag_state: DragState::default(),
            mouse_selection: MouseSelection::default(),
            git_remote: GitRemoteState::default(),
            git_check_receiver: None,
            update_state: UpdateState::new(),
            update_check_receiver: None,
            update_receiver: None,
            update_dialog_button: UpdateDialogButton::default(),
            update_dialog_areas: UpdateDialogAreas::default(),
            fake_version,
            scrollbar_drag: ScrollbarDragState::default(),
            scrollbar_areas: ScrollbarAreas::default(),
            preview_width: 80,
            resize_state: ResizeState::default(),
            border_areas: BorderAreas::default(),
            last_autosave_time: None,
            last_copy_time: None,
            copy_flash_lines: 0,
            remote_control_send_time: remote_control_time,
            temp_preview_files: Vec::new(),
            export_chooser: crate::types::ExportChooserState::default(),
        };

        // Open wizard on first run
        if should_open_wizard {
            app.wizard.open();
        }

        app.update_preview();

        // Initial Clear - ONLY for Terminal pane (not Claude, which needs time to start)
        // Note: No cd command sent here — PTY already starts in the correct cwd via cmd.cwd()
        // Sending cd would trigger Fish shell hooks (e.g. venv auto-activate) and change cwd
        if let Some(pty) = app.terminals.get_mut(&PaneId::Terminal) {
            let _ = pty.write_input(b"\x0c");
        }

        // Start background update check (non-blocking)
        app.start_update_check();

        app
    }

    /// Toggle preview maximize mode (F3): hides all panes except Preview, or restores previous layout
    pub(crate) fn toggle_preview_maximize(&mut self) {
        if self.preview_maximized {
            // Restore saved layout
            self.show_file_browser = self.preview_saved_layout.show_file_browser;
            self.show_lazygit = self.preview_saved_layout.show_lazygit;
            self.show_terminal = self.preview_saved_layout.show_terminal;
            self.preview_maximized = false;
        } else {
            // Ensure preview is visible
            if !self.show_preview {
                self.show_preview = true;
                self.config.ui.show_preview = true;
                let _ = crate::config::save_config(&self.config);
            }
            // Save current layout state
            self.preview_saved_layout = SavedLayout {
                show_file_browser: self.show_file_browser,
                show_lazygit: self.show_lazygit,
                show_terminal: self.show_terminal,
            };
            // Hide all other panes
            self.show_file_browser = false;
            self.show_lazygit = false;
            self.show_terminal = false;
            self.preview_maximized = true;
            self.active_pane = PaneId::Preview;
        }
    }

    fn update_preview(&mut self) {
        if let Some(path) = self.file_browser.selected_file() {
            if self.preview.current_file.as_ref() != Some(&path) {
                self.preview.load_file(path, &self.syntax_manager);
            }
        }
    }

    pub fn run(mut self, mut terminal: DefaultTerminal) -> Result<bool> {
        while !self.should_quit {
            // Check for exited PTYs and restart them with a shell
            self.check_and_restart_exited_ptys();

            // Auto-refresh file browser
            let refresh_interval = self.config.file_browser.auto_refresh_ms;
            if refresh_interval > 0 {
                let elapsed = self.last_refresh.elapsed().as_millis() as u64;
                if elapsed >= refresh_interval {
                    self.file_browser.refresh();
                    // Also check if preview file was modified externally
                    self.preview.reload_if_changed(&self.syntax_manager);
                    self.last_refresh = std::time::Instant::now();
                }
            }

            // Poll for async git remote check results
            self.poll_git_check();

            // Poll for async update check and update results
            self.poll_update_check();
            self.poll_update_result();
            // Send pending /remote-control command after delay
            self.poll_remote_control_send();

            terminal.draw(|frame| self.draw(frame))?;

            if event::poll(std::time::Duration::from_millis(16))? {
                match event::read()? {
                    Event::Mouse(mouse) => {
                        let size = terminal.size()?;
                        let area = Rect::new(0, 0, size.width, size.height);
                        let (files, preview, claude, lazygit, terminal_rect, footer) =
                            ui::layout::compute_layout(
                                area,
                                self.show_file_browser,
                                self.show_terminal,
                                self.show_lazygit,
                                self.show_preview,
                                self.preview_maximized,
                                &self.config.layout,
                            );
                        let rects = LayoutRects {
                            files,
                            preview,
                            claude,
                            lazygit,
                            terminal: terminal_rect,
                            footer,
                        };
                        self.handle_mouse_event(mouse, rects);
                    }
                    Event::Key(key) => {
                        self.handle_key_event(key);
                    }
                    Event::Paste(text) => {
                        self.handle_paste_event(text);
                    }

                    _ => {}
                } // End Match
            }
        }
        // Clean up temporary preview files
        self.cleanup_temp_files();

        Ok(self.should_restart)
    }
}
