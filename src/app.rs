use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::{layout::Rect, DefaultTerminal, Frame};
use std::collections::HashMap;

use crate::config::Config;
use crate::session::SessionState;
use crate::terminal::PseudoTerminal;
use crate::types::{
    BorderAreas, ClaudePermissionMode, DragState, EditorMode, GitRemoteCheckResult, GitRemoteState,
    HelpState, MouseSelection, PaneId, ResizeBorder, ResizeState, ScrollbarAreas, ScrollbarAxis,
    ScrollbarDragState, SearchMode, TerminalSelection,
};
use crate::ui;
use crate::ui::file_browser::FileBrowserState;
use crate::ui::preview::PreviewState;
use crate::ui::syntax::SyntaxManager;
use shell_escape::escape;
use std::borrow::Cow;
use std::path::Path;

use crate::browser;
use crate::git;
use crate::setup::wizard::WizardState;
use crate::ui::about::AboutState;
use crate::ui::dialog::Dialog;
use crate::ui::fuzzy_finder::FuzzyFinder;
use crate::ui::menu::MenuBar;
use crate::ui::settings::SettingsState;
use crate::ui::update_dialog::{UpdateDialogAreas, UpdateDialogButton};
use crate::update::{self, UpdateCheckResult, UpdateResult, UpdateState};

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

        // Determine if we should show permission mode dialog
        // Don't show if wizard needs to run first (first-time setup)
        let should_show_permission_dialog =
            config.claude.show_permission_dialog && config.setup.wizard_completed;

        // 1. Claude Pane - delayed init if permission dialog should be shown
        let claude_command_str;
        if should_show_permission_dialog {
            // Delay Claude PTY creation until permission mode is selected
            claude_pty_pending = true;
            permission_mode_dialog.open_with_default(config.claude.default_permission_mode);
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
            show_file_browser: true,
            show_terminal: false,
            show_lazygit: false,
            show_preview: true,
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

        // Start background update check (non-blocking)
        app.start_update_check();

        app
    }

    /// Start async update check
    fn start_update_check(&mut self) {
        self.update_state.start_check();
        self.update_check_receiver = Some(update::check_for_update_async_with_version(
            self.fake_version.clone(),
        ));
    }

    /// Poll for async update check results
    fn poll_update_check(&mut self) {
        if let Some(ref receiver) = self.update_check_receiver {
            if let Ok(result) = receiver.try_recv() {
                match result {
                    UpdateCheckResult::UpToDate => {
                        self.update_state.set_up_to_date();
                        // For manual checks, show "up to date" dialog
                        if self.update_state.manual_check {
                            self.update_state.show_dialog = true;
                        }
                    }
                    UpdateCheckResult::UpdateAvailable {
                        version,
                        release_notes,
                    } => {
                        self.update_state.set_available(version, release_notes);
                    }
                    UpdateCheckResult::NoReleasesFound => {
                        // No releases found - treat as up-to-date for auto checks
                        // but show info for manual checks
                        if self.update_state.manual_check {
                            self.update_state.set_error(
                                "No releases found for your platform.\nCheck GitHub for available downloads.".to_string()
                            );
                        } else {
                            self.update_state.set_up_to_date();
                            self.update_state.show_dialog = false;
                        }
                    }
                    UpdateCheckResult::Error(msg) => {
                        self.update_state.set_error(msg);
                        // Show error dialog only for manual checks, silent fail on startup
                        if !self.update_state.manual_check {
                            self.update_state.show_dialog = false;
                        }
                    }
                }
                self.update_check_receiver = None;
            }
        }
    }

    /// Poll for async update results
    fn poll_update_result(&mut self) {
        if let Some(ref receiver) = self.update_receiver {
            update::log_update("[app] poll_update_result: checking receiver...");
            match receiver.try_recv() {
                Ok(result) => {
                    update::log_update(&format!(
                        "[app] poll_update_result: GOT RESULT {:?}",
                        result
                    ));
                    match result {
                        UpdateResult::Success { new_version, .. } => {
                            update::log_update(&format!("[app] SUCCESS: {}", new_version));
                            // Set success state - shows dedicated success screen
                            self.update_state.set_success(new_version);
                            // Set button to Restart (primary action after update)
                            self.update_dialog_button = UpdateDialogButton::Restart;
                        }
                        UpdateResult::Error(msg) => {
                            update::log_update(&format!("[app] ERROR: {}", msg));
                            self.update_state.set_error(msg.clone());
                            self.update_state.updating = false;
                            self.update_state.show_dialog = true;
                        }
                    }
                    self.update_receiver = None;
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => {
                    // No result yet, keep waiting
                }
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    update::log_update("[app] poll_update_result: CHANNEL DISCONNECTED!");
                    self.update_state
                        .set_error("Update channel disconnected unexpectedly".to_string());
                    self.update_state.updating = false;
                    self.update_state.show_dialog = true;
                    self.update_receiver = None;
                }
            }
        }
    }

    /// Start the actual update process
    fn start_update(&mut self) {
        update::log_update("[app] start_update() CALLED");
        self.update_state.start_update();

        // If fake_version is set, simulate the update instead of downloading
        if self.fake_version.is_some() {
            update::log_update("[app] Using FAKE update (simulated)");
            // Simulate update with a short delay
            let (tx, rx) = std::sync::mpsc::channel();
            std::thread::spawn(move || {
                std::thread::sleep(std::time::Duration::from_secs(2));
                let _ = tx.send(update::UpdateResult::Success {
                    old_version: "simulated".to_string(),
                    new_version: "simulated".to_string(),
                });
            });
            self.update_receiver = Some(rx);
        } else {
            update::log_update("[app] Calling perform_update_async()...");
            self.update_receiver = Some(update::perform_update_async());
            update::log_update("[app] update_receiver is now Some");
        }
    }

    /// Build Claude command with permission mode flags
    fn build_claude_command(config: &Config, mode: ClaudePermissionMode) -> Vec<String> {
        let mut cmd = if config.pty.claude_command.is_empty() {
            // Default: use the same shell as Terminal pane
            let mut shell_cmd = vec![config.terminal.shell_path.clone()];
            shell_cmd.extend(config.terminal.shell_args.clone());
            shell_cmd
        } else {
            config.pty.claude_command.clone()
        };

        // Only add permission flags if using claude command (not shell)
        if !config.pty.claude_command.is_empty() {
            if mode.is_yolo() {
                // YOLO mode: --dangerously-skip-permissions flag
                if !cmd
                    .iter()
                    .any(|a| a.contains("--dangerously-skip-permissions"))
                {
                    cmd.push("--dangerously-skip-permissions".to_string());
                }
            } else if let Some(flag_value) = mode.cli_flag() {
                // Normal modes: --permission-mode flag
                if !cmd.iter().any(|a| a.contains("--permission-mode")) {
                    cmd.push("--permission-mode".to_string());
                    cmd.push(flag_value.to_string());
                }
            }
        }

        cmd
    }

    /// Initialize Claude PTY with the selected permission mode
    fn init_claude_pty(&mut self, mode: ClaudePermissionMode) {
        self.claude_permission_mode = mode;
        self.claude_pty_pending = false;

        let claude_cmd = Self::build_claude_command(&self.config, mode);
        self.claude_command_used = claude_cmd.join(" ");

        let cwd = self.file_browser.current_dir.clone();
        let rows = 24;
        let cols = 80;

        match PseudoTerminal::new(&claude_cmd, rows, cols, &cwd) {
            Ok(pty) => {
                self.terminals.insert(PaneId::Claude, pty);
                self.claude_error = None;
            }
            Err(e) => {
                self.claude_error = Some(format!(
                    "Failed to start shell\n\nCommand: {}\n\nError: {}",
                    self.claude_command_used, e
                ));
            }
        }
    }

    /// Initialize Claude PTY after wizard completion
    /// Shows permission mode dialog if configured, otherwise starts Claude directly
    fn init_claude_after_wizard(&mut self) {
        // Remove existing Claude PTY (started with pre-wizard config)
        self.terminals.remove(&PaneId::Claude);
        self.claude_error = None;

        let should_show_permission_dialog = self.config.claude.show_permission_dialog;

        if should_show_permission_dialog {
            self.claude_pty_pending = true;
            self.permission_mode_dialog
                .open_with_default(self.config.claude.default_permission_mode);
        } else {
            let mode = self
                .config
                .claude
                .default_permission_mode
                .unwrap_or(ClaudePermissionMode::Default);
            self.init_claude_pty(mode);
            self.active_pane = PaneId::Claude;
        }
    }

    /// Trigger manual update check from settings menu
    pub fn trigger_update_check(&mut self) {
        self.update_state = UpdateState::new();
        self.update_state.show_dialog = true;
        self.update_state.manual_check = true; // Show errors for manual checks
        self.start_update_check();
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

            terminal.draw(|frame| self.draw(frame))?;

            if event::poll(std::time::Duration::from_millis(16))? {
                match event::read()? {
                    Event::Mouse(mouse) => {
                        let size = terminal.size()?;
                        let area = Rect::new(0, 0, size.width, size.height);
                        let (files, preview, claude, lazygit, term, footer_area) =
                            ui::layout::compute_layout(
                                area,
                                self.show_file_browser,
                                self.show_terminal,
                                self.show_lazygit,
                                self.show_preview,
                                &self.config.layout,
                            );

                        let x = mouse.column;
                        let y = mouse.row;

                        // Helper closure for hit testing
                        let is_inside = |r: Rect, x: u16, y: u16| -> bool {
                            x >= r.x && x < r.x + r.width && y >= r.y && y < r.y + r.height
                        };

                        match mouse.kind {
                            crossterm::event::MouseEventKind::Down(
                                crossterm::event::MouseButton::Left,
                            ) => {
                                // Block all background interaction when any modal is open
                                // Update dialog - handle before other modals
                                if self.update_state.show_dialog {
                                    use crate::ui::update_dialog;
                                    if update_dialog::is_inside_popup(
                                        &self.update_dialog_areas,
                                        x,
                                        y,
                                    ) {
                                        if let Some(button) = update_dialog::check_button_click(
                                            &self.update_dialog_areas,
                                            x,
                                            y,
                                        ) {
                                            match button {
                                                UpdateDialogButton::Update => {
                                                    if !self.update_state.updating {
                                                        self.start_update();
                                                    }
                                                }
                                                UpdateDialogButton::Later
                                                | UpdateDialogButton::Close => {
                                                    self.update_state.close_dialog();
                                                }
                                                UpdateDialogButton::Restart => {
                                                    // Signal restart and exit cleanly
                                                    self.should_restart = true;
                                                    self.should_quit = true;
                                                }
                                            }
                                        }
                                    } else {
                                        self.update_state.close_dialog();
                                    }
                                    continue;
                                }

                                // About dialog - click outside to close
                                if self.about.visible {
                                    if let Some(popup) = self.about.popup_area {
                                        if !is_inside(popup, x, y) {
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

                                // Permission mode dialog - click outside uses default mode
                                // Skip if update dialog is visible - update takes priority
                                if self.permission_mode_dialog.visible
                                    && !self.update_state.show_dialog
                                {
                                    let mode = ClaudePermissionMode::Default;
                                    self.permission_mode_dialog.close();
                                    if self.claude_pty_pending {
                                        self.init_claude_pty(mode);
                                    }
                                    self.active_pane = PaneId::Claude;
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

                                // Check scrollbar click (before normal pane clicks)
                                {
                                    let mut hit_scrollbar = false;
                                    for pane_id in [
                                        PaneId::FileBrowser,
                                        PaneId::Preview,
                                        PaneId::Claude,
                                        PaneId::LazyGit,
                                        PaneId::Terminal,
                                    ] {
                                        if let Some(sb) = self.scrollbar_areas.get(&pane_id) {
                                            if x == sb.x && y >= sb.y && y < sb.y + sb.height {
                                                self.scrollbar_drag.dragging = true;
                                                self.scrollbar_drag.pane = Some(pane_id);
                                                self.scrollbar_drag.axis = ScrollbarAxis::Vertical;
                                                self.handle_scrollbar_position(pane_id, y, sb);
                                                hit_scrollbar = true;
                                                break;
                                            }
                                        }
                                    }
                                    // Check horizontal scrollbar for preview pane
                                    if !hit_scrollbar {
                                        if let Some(hsb) = self.scrollbar_areas.preview_horizontal {
                                            if y == hsb.y && x >= hsb.x && x < hsb.x + hsb.width {
                                                self.scrollbar_drag.dragging = true;
                                                self.scrollbar_drag.pane = Some(PaneId::Preview);
                                                self.scrollbar_drag.axis =
                                                    ScrollbarAxis::Horizontal;
                                                self.handle_horizontal_scrollbar_position(x, hsb);
                                                hit_scrollbar = true;
                                            }
                                        }
                                    }
                                    if hit_scrollbar {
                                        continue;
                                    }
                                }

                                // Check for pane border drag (interactive resizing)
                                {
                                    let mut hit_border = false;
                                    // Check vertical border: FileBrowser | Preview
                                    if let Some(border_x) = self.border_areas.file_preview_x {
                                        let top_limit = self
                                            .border_areas
                                            .top_claude_y
                                            .unwrap_or(self.border_areas.total_height);
                                        if (x as i16 - border_x as i16).abs() <= 1
                                            && y > 0
                                            && y < top_limit
                                        {
                                            self.resize_state.dragging = true;
                                            self.resize_state.border =
                                                Some(ResizeBorder::FilePreview);
                                            hit_border = true;
                                        }
                                    }
                                    // Check vertical border: Preview | Right Panel
                                    if !hit_border {
                                        if let Some(border_x) = self.border_areas.preview_right_x {
                                            let top_limit = self
                                                .border_areas
                                                .top_claude_y
                                                .unwrap_or(self.border_areas.total_height);
                                            if (x as i16 - border_x as i16).abs() <= 1
                                                && y > 0
                                                && y < top_limit
                                            {
                                                self.resize_state.dragging = true;
                                                self.resize_state.border =
                                                    Some(ResizeBorder::PreviewRight);
                                                hit_border = true;
                                            }
                                        }
                                    }
                                    // Check horizontal border: Top Area | Claude
                                    if !hit_border {
                                        if let Some(border_y) = self.border_areas.top_claude_y {
                                            if (y as i16 - border_y as i16).abs() <= 1 {
                                                self.resize_state.dragging = true;
                                                self.resize_state.border =
                                                    Some(ResizeBorder::TopClaude);
                                                hit_border = true;
                                            }
                                        }
                                    }
                                    if hit_border {
                                        continue;
                                    }
                                }

                                if is_inside(files, x, y) {
                                    self.active_pane = PaneId::FileBrowser;
                                    // File browser layout: [list with borders] + [info bar (1 line)]
                                    // List content area: after top border, before bottom border and info bar
                                    let list_content_top = files.y + 1; // After top border
                                    let list_content_bottom =
                                        files.y + files.height.saturating_sub(3); // Before bottom border (1) + info bar (1)

                                    // Only handle clicks within the list content area
                                    if y >= list_content_top && y <= list_content_bottom {
                                        // Calculate relative position within visible list
                                        let relative_y = y.saturating_sub(list_content_top);

                                        // Get scroll offset to calculate actual item index
                                        let scroll_offset = self.file_browser.list_state.offset();
                                        let idx = scroll_offset + relative_y as usize;

                                        if idx < self.file_browser.entries.len() {
                                            // Start drag with the file path
                                            let entry = &self.file_browser.entries[idx];
                                            self.drag_state.start(entry.path.clone(), x, y);
                                            // Check for double-click (same item within 300ms)
                                            let now = std::time::Instant::now();
                                            let is_double_click = self.last_click_idx == Some(idx)
                                                && now
                                                    .duration_since(self.last_click_time)
                                                    .as_millis()
                                                    < 300;

                                            // Update tracking for next click
                                            self.last_click_time = now;
                                            self.last_click_idx = Some(idx);

                                            // Check if editor has unsaved changes before switching
                                            let has_unsaved = self.preview.mode == EditorMode::Edit
                                                && self.preview.is_modified();

                                            if is_double_click {
                                                // Double-click: enter directory or open file
                                                let is_dir = self
                                                    .file_browser
                                                    .entries
                                                    .get(idx)
                                                    .map(|e| e.is_dir)
                                                    .unwrap_or(false);
                                                if is_dir {
                                                    if has_unsaved {
                                                        self.dialog.dialog_type = ui::dialog::DialogType::Confirm {
                                                        title: "Unsaved Changes".to_string(),
                                                        message: "Discard changes and enter directory?".to_string(),
                                                        action: ui::dialog::DialogAction::EnterDirectory { target_idx: idx },
                                                    };
                                                    } else {
                                                        self.file_browser
                                                            .list_state
                                                            .select(Some(idx));
                                                        self.file_browser.enter_selected();
                                                        self.update_preview();
                                                        self.sync_terminals();
                                                        self.check_repo_change();
                                                    }
                                                }
                                            } else {
                                                // Single click: just select (but check for unsaved changes)
                                                if has_unsaved {
                                                    self.dialog.dialog_type =
                                                    ui::dialog::DialogType::Confirm {
                                                        title: "Unsaved Changes".to_string(),
                                                        message: "Discard changes and switch file?"
                                                            .to_string(),
                                                        action:
                                                            ui::dialog::DialogAction::SwitchFile {
                                                                target_idx: idx,
                                                            },
                                                    };
                                                } else {
                                                    self.file_browser.list_state.select(Some(idx));
                                                    self.update_preview();
                                                }
                                            }
                                        }
                                    } // Close: if y >= list_content_top && y <= list_content_bottom
                                } else if is_inside(preview, x, y) {
                                    // Mouse selection in Preview (Read-Only mode only)
                                    // Alt+Click OR normal click starts character-level selection
                                    if self.preview.mode == crate::types::EditorMode::ReadOnly {
                                        self.mouse_selection.start(PaneId::Preview, x, y, preview);
                                    }
                                    // Click-to-position cursor in Edit mode
                                    else if self.preview.mode == crate::types::EditorMode::Edit {
                                        self.position_preview_cursor(preview, x, y);
                                    }
                                    self.active_pane = PaneId::Preview;
                                } else if is_inside(claude, x, y) {
                                    // Show startup dialog if prefixes configured and not yet shown
                                    if !self.claude_startup.shown_this_session
                                        && !self.config.claude.startup_prefixes.is_empty()
                                    {
                                        self.claude_startup
                                            .open(self.config.claude.startup_prefixes.clone());
                                    } else {
                                        // Normal click starts character-level mouse text selection
                                        self.mouse_selection.start(PaneId::Claude, x, y, claude);
                                        self.active_pane = PaneId::Claude;
                                    }
                                } else if is_inside(lazygit, x, y) {
                                    // Normal click starts character-level mouse text selection
                                    self.mouse_selection.start(PaneId::LazyGit, x, y, lazygit);
                                    self.active_pane = PaneId::LazyGit;
                                } else if is_inside(term, x, y) {
                                    // Normal click starts character-level mouse text selection
                                    self.mouse_selection.start(PaneId::Terminal, x, y, term);
                                    self.active_pane = PaneId::Terminal;
                                } else if is_inside(footer_area, x, y) {
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
                                                FooterAction::ToggleFiles => {
                                                    self.show_file_browser =
                                                        !self.show_file_browser;
                                                    if self.show_file_browser {
                                                        self.active_pane = PaneId::FileBrowser;
                                                    } else if self.active_pane
                                                        == PaneId::FileBrowser
                                                    {
                                                        self.active_pane = PaneId::Claude;
                                                    }
                                                }
                                                FooterAction::TogglePreview => {
                                                    self.show_preview = !self.show_preview;
                                                    if self.show_preview {
                                                        self.active_pane = PaneId::Preview;
                                                    } else if self.active_pane == PaneId::Preview {
                                                        self.active_pane = PaneId::FileBrowser;
                                                    }
                                                }
                                                FooterAction::Refresh => {
                                                    self.file_browser.refresh();
                                                    self.update_preview();
                                                }
                                                FooterAction::FocusClaude => {
                                                    if !self.claude_startup.shown_this_session
                                                        && !self
                                                            .config
                                                            .claude
                                                            .startup_prefixes
                                                            .is_empty()
                                                    {
                                                        self.claude_startup.open(
                                                            self.config
                                                                .claude
                                                                .startup_prefixes
                                                                .clone(),
                                                        );
                                                    } else {
                                                        self.active_pane = PaneId::Claude;
                                                    }
                                                }
                                                FooterAction::ToggleGit => {
                                                    self.show_lazygit = !self.show_lazygit;
                                                    if self.show_lazygit {
                                                        self.active_pane = PaneId::LazyGit;
                                                    }
                                                }
                                                FooterAction::ToggleTerm => {
                                                    self.show_terminal = !self.show_terminal;
                                                    if self.show_terminal {
                                                        self.active_pane = PaneId::Terminal;
                                                    }
                                                }
                                                FooterAction::FuzzyFind => {
                                                    self.fuzzy_finder
                                                        .open(&self.file_browser.current_dir);
                                                }
                                                FooterAction::OpenFile => {
                                                    if let Some(path) =
                                                        self.file_browser.selected_file()
                                                    {
                                                        if browser::can_preview_in_browser(&path) {
                                                            let preview_path =
                                                                if browser::is_markdown(&path) {
                                                                    browser::markdown_to_html(&path)
                                                                        .unwrap_or(path)
                                                                } else {
                                                                    path
                                                                };
                                                            let _ =
                                                                browser::open_file(&preview_path);
                                                        }
                                                    }
                                                }
                                                FooterAction::OpenFinder => {
                                                    let _ = browser::open_in_file_manager(
                                                        &self.file_browser.current_dir,
                                                    );
                                                }
                                                FooterAction::ToggleHidden => {
                                                    self.file_browser.show_hidden =
                                                        !self.file_browser.show_hidden;
                                                    self.file_browser.refresh();
                                                    self.update_preview();
                                                }
                                                FooterAction::Settings => {
                                                    let cfg = self.config.clone();
                                                    self.settings.open(&cfg);
                                                }
                                                FooterAction::About => self.about.open(),
                                                FooterAction::Help => self.help.open(),
                                                FooterAction::Edit => {
                                                    // Enter edit mode in Preview
                                                    if self.active_pane == PaneId::Preview {
                                                        self.preview.mode =
                                                            crate::types::EditorMode::Edit;
                                                    }
                                                }
                                                FooterAction::StartSelect => {
                                                    // Start selection mode in current pane
                                                    if self.active_pane == PaneId::Preview {
                                                        self.terminal_selection.start(
                                                            self.preview.scroll as usize,
                                                            PaneId::Preview,
                                                        );
                                                    } else if matches!(
                                                        self.active_pane,
                                                        PaneId::Claude
                                                            | PaneId::LazyGit
                                                            | PaneId::Terminal
                                                    ) {
                                                        self.terminal_selection
                                                            .start(0, self.active_pane);
                                                    }
                                                }
                                                FooterAction::Save => {
                                                    // Save in edit mode
                                                    if self.active_pane == PaneId::Preview
                                                        && self.preview.mode
                                                            == crate::types::EditorMode::Edit
                                                    {
                                                        let _ = self.preview.save();
                                                    }
                                                }
                                                FooterAction::ExitEdit => {
                                                    // Exit edit mode
                                                    if self.active_pane == PaneId::Preview {
                                                        self.preview.mode =
                                                            crate::types::EditorMode::ReadOnly;
                                                    }
                                                }
                                                FooterAction::Undo => {
                                                    if self.active_pane == PaneId::Preview
                                                        && self.preview.mode
                                                            == crate::types::EditorMode::Edit
                                                    {
                                                        if let Some(editor) =
                                                            &mut self.preview.editor
                                                        {
                                                            editor.undo();
                                                            self.preview.update_modified();
                                                            self.preview.update_edit_highlighting(
                                                                &self.syntax_manager,
                                                            );
                                                        }
                                                    }
                                                }
                                                FooterAction::Redo => {
                                                    // Redo handled by keyboard only
                                                }
                                                FooterAction::SelectDown
                                                | FooterAction::SelectUp => {
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
                                                FooterAction::ToggleBlock => {
                                                    // MC Edit: Toggle block marking (F3)
                                                    if self.active_pane == PaneId::Preview
                                                        && self.preview.mode
                                                            == crate::types::EditorMode::Edit
                                                    {
                                                        self.preview.toggle_block_marking();
                                                    }
                                                }
                                                FooterAction::CopyBlock => {
                                                    // MC Edit: Copy block (F5)
                                                    if self.active_pane == PaneId::Preview
                                                        && self.preview.mode
                                                            == crate::types::EditorMode::Edit
                                                    {
                                                        self.preview.copy_block();
                                                    }
                                                }
                                                FooterAction::MoveBlock => {
                                                    // MC Edit: Move/cut block (F6)
                                                    if self.active_pane == PaneId::Preview
                                                        && self.preview.mode
                                                            == crate::types::EditorMode::Edit
                                                    {
                                                        self.preview.move_block();
                                                        self.preview.update_modified();
                                                        self.preview.update_edit_highlighting(
                                                            &self.syntax_manager,
                                                        );
                                                    }
                                                }
                                                FooterAction::DeleteBlock => {
                                                    // MC Edit: Delete block (F8)
                                                    if self.active_pane == PaneId::Preview
                                                        && self.preview.mode
                                                            == crate::types::EditorMode::Edit
                                                    {
                                                        self.preview.delete_block();
                                                        self.preview.update_modified();
                                                        self.preview.update_edit_highlighting(
                                                            &self.syntax_manager,
                                                        );
                                                    }
                                                }
                                                FooterAction::PlatformPaste => {
                                                    // Platform paste (Ctrl+V)
                                                    if self.active_pane == PaneId::Preview
                                                        && self.preview.mode
                                                            == crate::types::EditorMode::Edit
                                                    {
                                                        self.preview.paste_from_clipboard();
                                                        self.preview.update_modified();
                                                        self.preview.update_edit_highlighting(
                                                            &self.syntax_manager,
                                                        );
                                                    }
                                                }
                                                FooterAction::Search => {
                                                    // Open search (/ or Ctrl+F)
                                                    if self.active_pane == PaneId::Preview {
                                                        self.preview.search.open();
                                                    }
                                                }
                                                FooterAction::SearchReplace => {
                                                    // Open search & replace (Ctrl+H) - only in Edit mode
                                                    if self.active_pane == PaneId::Preview
                                                        && self.preview.mode
                                                            == crate::types::EditorMode::Edit
                                                    {
                                                        self.preview.search.open();
                                                        self.preview.search.mode =
                                                            crate::types::SearchMode::Replace;
                                                    }
                                                }
                                                FooterAction::FileMenu => {
                                                    // Open file menu (F9)
                                                    if self.active_pane == PaneId::FileBrowser {
                                                        self.menu.visible = true;
                                                    }
                                                }
                                                FooterAction::None => {}
                                            }
                                            break;
                                        }
                                    }
                                }
                            }
                            // Handle drag movement
                            crossterm::event::MouseEventKind::Drag(
                                crossterm::event::MouseButton::Left,
                            ) => {
                                // Block drag when any modal is open
                                if self.about.visible
                                    || self.help.visible
                                    || self.settings.visible
                                    || self.dialog.is_active()
                                    || self.fuzzy_finder.visible
                                    || self.permission_mode_dialog.visible
                                    || self.claude_startup.visible
                                    || self.wizard.visible
                                    || self.menu.visible
                                {
                                    continue;
                                }
                                // Handle scrollbar drag
                                if self.scrollbar_drag.dragging {
                                    match self.scrollbar_drag.axis {
                                        ScrollbarAxis::Horizontal => {
                                            if let Some(hsb) =
                                                self.scrollbar_areas.preview_horizontal
                                            {
                                                self.handle_horizontal_scrollbar_position(x, hsb);
                                            }
                                        }
                                        ScrollbarAxis::Vertical => {
                                            if let Some(pane) = self.scrollbar_drag.pane {
                                                if let Some(sb) = self.scrollbar_areas.get(&pane) {
                                                    self.handle_scrollbar_position(pane, y, sb);
                                                }
                                            }
                                        }
                                    }
                                    continue;
                                }
                                // Handle pane border resize drag
                                if self.resize_state.dragging {
                                    match self.resize_state.border {
                                        Some(ResizeBorder::FilePreview) => {
                                            if self.border_areas.total_width > 0 {
                                                let new_pct = ((x as f64
                                                    / self.border_areas.total_width as f64)
                                                    * 100.0)
                                                    as u16;
                                                self.config.layout.file_browser_width_percent =
                                                    new_pct.clamp(10, 50);
                                            }
                                        }
                                        Some(ResizeBorder::PreviewRight) => {
                                            if self.border_areas.total_width > 0 {
                                                let file_pct =
                                                    self.config.layout.file_browser_width_percent;
                                                let new_preview_pct = ((x as f64
                                                    / self.border_areas.total_width as f64)
                                                    * 100.0)
                                                    as u16;
                                                let preview_pct = new_preview_pct
                                                    .saturating_sub(file_pct)
                                                    .clamp(15, 70);
                                                self.config.layout.preview_width_percent =
                                                    preview_pct;
                                                self.config.layout.right_panel_width_percent =
                                                    100u16
                                                        .saturating_sub(file_pct)
                                                        .saturating_sub(preview_pct)
                                                        .clamp(10, 60);
                                            }
                                        }
                                        Some(ResizeBorder::TopClaude) => {
                                            // Footer is 1 line at bottom
                                            let usable_height =
                                                self.border_areas.total_height.saturating_sub(1);
                                            if usable_height > 0 {
                                                let claude_start_pct =
                                                    ((y as f64 / usable_height as f64) * 100.0)
                                                        as u16;
                                                self.config.layout.claude_height_percent = 100u16
                                                    .saturating_sub(claude_start_pct)
                                                    .clamp(20, 80);
                                            }
                                        }
                                        None => {}
                                    }
                                    continue;
                                }
                                // Handle character-level mouse text selection in terminal panes
                                if self.mouse_selection.selecting {
                                    self.mouse_selection.update(x, y);
                                } else if self.drag_state.dragging {
                                    self.drag_state.update_position(x, y);
                                }
                            }
                            // Handle drag drop and mouse selection finish
                            crossterm::event::MouseEventKind::Up(
                                crossterm::event::MouseButton::Left,
                            ) => {
                                // Block drop when any modal is open
                                if self.about.visible
                                    || self.help.visible
                                    || self.settings.visible
                                    || self.dialog.is_active()
                                    || self.fuzzy_finder.visible
                                    || self.permission_mode_dialog.visible
                                    || self.claude_startup.visible
                                    || self.wizard.visible
                                    || self.menu.visible
                                {
                                    continue;
                                }
                                // Handle pane resize drag finish - save config
                                if self.resize_state.dragging {
                                    self.resize_state.dragging = false;
                                    self.resize_state.border = None;
                                    let _ = crate::config::save_config(&self.config);
                                    continue;
                                }
                                // Handle scrollbar drag finish
                                if self.scrollbar_drag.dragging {
                                    self.scrollbar_drag.dragging = false;
                                    self.scrollbar_drag.pane = None;
                                    self.scrollbar_drag.axis = ScrollbarAxis::default();
                                    continue;
                                }
                                // Handle mouse text selection finish - copy to clipboard
                                // Only copy if selection covers meaningful distance (>2 chars)
                                // to prevent clipboard overwrite on simple focus-clicks
                                if self.mouse_selection.selecting {
                                    if self.mouse_selection.has_meaningful_selection() {
                                        self.copy_mouse_selection_to_clipboard();
                                    }
                                    self.mouse_selection.clear();
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
                                            self.active_pane = target_pane;
                                        }
                                    } else {
                                        self.drag_state.clear();
                                    }
                                }
                            }
                            crossterm::event::MouseEventKind::ScrollDown => {
                                // Block all background scroll when any modal is open
                                if self.about.visible {
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
                                if self.settings.visible
                                    || self.dialog.is_active()
                                    || self.fuzzy_finder.visible
                                    || self.permission_mode_dialog.visible
                                    || self.claude_startup.visible
                                    || self.wizard.visible
                                    || self.menu.visible
                                {
                                    continue;
                                }

                                if is_inside(files, x, y) {
                                    self.file_browser.down();
                                    self.update_preview();
                                } else if is_inside(preview, x, y) {
                                    // Shift+Scroll = horizontal scroll
                                    if mouse
                                        .modifiers
                                        .contains(crossterm::event::KeyModifiers::SHIFT)
                                    {
                                        if self.preview.mode == crate::types::EditorMode::Edit {
                                            // In Edit mode: scroll content horizontally (not cursor)
                                            let max = self.preview.edit_max_display_width();
                                            for _ in 0..3 {
                                                self.preview.scroll_right(max);
                                            }
                                        } else {
                                            let max = self.preview.max_line_width();
                                            for _ in 0..3 {
                                                self.preview.scroll_right(max);
                                            }
                                        }
                                    } else if self.preview.mode == crate::types::EditorMode::Edit {
                                        // In Edit mode, move TextArea cursor
                                        if let Some(editor) = &mut self.preview.editor {
                                            for _ in 0..3 {
                                                editor.move_cursor(tui_textarea::CursorMove::Down);
                                            }
                                        }
                                    } else {
                                        self.preview.scroll_down();
                                    }
                                } else if is_inside(claude, x, y) {
                                    if let Some(pty) = self.terminals.get_mut(&PaneId::Claude) {
                                        pty.scroll_down(3);
                                    }
                                } else if is_inside(lazygit, x, y) {
                                    if let Some(pty) = self.terminals.get_mut(&PaneId::LazyGit) {
                                        pty.scroll_down(3);
                                    }
                                } else if is_inside(term, x, y) {
                                    if let Some(pty) = self.terminals.get_mut(&PaneId::Terminal) {
                                        pty.scroll_down(3);
                                    }
                                }
                            }
                            crossterm::event::MouseEventKind::ScrollUp => {
                                // Block all background scroll when any modal is open
                                if self.about.visible {
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
                                if self.settings.visible
                                    || self.dialog.is_active()
                                    || self.fuzzy_finder.visible
                                    || self.permission_mode_dialog.visible
                                    || self.claude_startup.visible
                                    || self.wizard.visible
                                    || self.menu.visible
                                {
                                    continue;
                                }

                                if is_inside(files, x, y) {
                                    self.file_browser.up();
                                    self.update_preview();
                                } else if is_inside(preview, x, y) {
                                    // Shift+Scroll = horizontal scroll
                                    if mouse
                                        .modifiers
                                        .contains(crossterm::event::KeyModifiers::SHIFT)
                                    {
                                        if self.preview.mode == crate::types::EditorMode::Edit {
                                            // In Edit mode: scroll content horizontally (not cursor)
                                            for _ in 0..3 {
                                                self.preview.scroll_left();
                                            }
                                        } else {
                                            for _ in 0..3 {
                                                self.preview.scroll_left();
                                            }
                                        }
                                    } else if self.preview.mode == crate::types::EditorMode::Edit {
                                        // In Edit mode, move TextArea cursor
                                        if let Some(editor) = &mut self.preview.editor {
                                            for _ in 0..3 {
                                                editor.move_cursor(tui_textarea::CursorMove::Up);
                                            }
                                        }
                                    } else {
                                        self.preview.scroll_up();
                                    }
                                } else if is_inside(claude, x, y) {
                                    if let Some(pty) = self.terminals.get_mut(&PaneId::Claude) {
                                        pty.scroll_up(3);
                                    }
                                } else if is_inside(lazygit, x, y) {
                                    if let Some(pty) = self.terminals.get_mut(&PaneId::LazyGit) {
                                        pty.scroll_up(3);
                                    }
                                } else if is_inside(term, x, y) {
                                    if let Some(pty) = self.terminals.get_mut(&PaneId::Terminal) {
                                        pty.scroll_up(3);
                                    }
                                }
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
                                            let full_path =
                                                self.fuzzy_finder.base_dir.join(&selected);
                                            // Navigate to file's directory and select it
                                            if let Some(parent) = full_path.parent() {
                                                self.file_browser.current_dir =
                                                    parent.to_path_buf();
                                                self.file_browser.load_directory();
                                                // Try to select the file
                                                let file_name = full_path
                                                    .file_name()
                                                    .map(|n| n.to_string_lossy().to_string());
                                                if let Some(name) = file_name {
                                                    for (i, entry) in
                                                        self.file_browser.entries.iter().enumerate()
                                                    {
                                                        if entry.name == name {
                                                            self.file_browser
                                                                .list_state
                                                                .select(Some(i));
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

                            // Update dialog handling (high priority)
                            if self.update_state.show_dialog {
                                match key.code {
                                    KeyCode::Esc => {
                                        self.update_state.close_dialog();
                                    }
                                    KeyCode::Enter => {
                                        // Success screen with Restart/Close buttons
                                        if self.update_state.update_success {
                                            if self.update_dialog_button
                                                == UpdateDialogButton::Restart
                                            {
                                                // Signal restart and exit cleanly
                                                self.should_restart = true;
                                                self.should_quit = true;
                                            } else {
                                                self.update_state.close_dialog();
                                            }
                                        }
                                        // Update available with Update/Later buttons
                                        else if self.update_state.available_version.is_some()
                                            && !self.update_state.updating
                                        {
                                            if self.update_dialog_button
                                                == UpdateDialogButton::Update
                                            {
                                                self.start_update();
                                            } else {
                                                self.update_state.close_dialog();
                                            }
                                        } else {
                                            self.update_state.close_dialog();
                                        }
                                    }
                                    KeyCode::Tab | KeyCode::Left | KeyCode::Right => {
                                        // Toggle buttons for update available or success screens
                                        if self.update_state.update_success {
                                            // Toggle Restart/Close
                                            self.update_dialog_button = if self.update_dialog_button
                                                == UpdateDialogButton::Restart
                                            {
                                                UpdateDialogButton::Close
                                            } else {
                                                UpdateDialogButton::Restart
                                            };
                                        } else if self.update_state.available_version.is_some()
                                            && !self.update_state.updating
                                        {
                                            // Toggle Update/Later
                                            self.update_dialog_button =
                                                self.update_dialog_button.toggle();
                                        }
                                    }
                                    KeyCode::Up | KeyCode::Char('k') => {
                                        // Scroll release notes up
                                        self.update_state.scroll_release_notes_up();
                                    }
                                    KeyCode::Down | KeyCode::Char('j') => {
                                        // Scroll release notes down
                                        // Max scroll = number of lines - visible area (estimate ~10)
                                        let max = self
                                            .update_state
                                            .release_notes
                                            .as_ref()
                                            .map(|n| n.lines().count().saturating_sub(10) as u16)
                                            .unwrap_or(0);
                                        self.update_state.scroll_release_notes_down(max);
                                    }
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
                                match &self.dialog.dialog_type {
                                    ui::dialog::DialogType::Input { value, action, .. } => {
                                        match key.code {
                                            KeyCode::Esc => self.dialog.close(),
                                            KeyCode::Enter => {
                                                let val = value.clone();
                                                let act = action.clone();
                                                self.dialog.close();
                                                self.execute_dialog_action(act, Some(val));
                                            }
                                            // Tab: Path completion for GoToPath dialog
                                            KeyCode::Tab => {
                                                if matches!(
                                                    action,
                                                    ui::dialog::DialogAction::GoToPath
                                                ) {
                                                    self.dialog.try_complete_path();
                                                }
                                            }
                                            KeyCode::Backspace => self.dialog.delete_char_before(),
                                            KeyCode::Delete => self.dialog.delete_char_at(),
                                            KeyCode::Left => self.dialog.cursor_left(),
                                            KeyCode::Right => self.dialog.cursor_right(),
                                            KeyCode::Home => self.dialog.cursor_home(),
                                            KeyCode::End => self.dialog.cursor_end(),
                                            KeyCode::Char(c) => self.dialog.insert_char(c),
                                            _ => {}
                                        }
                                    }
                                    ui::dialog::DialogType::Confirm { action, .. } => {
                                        match key.code {
                                            KeyCode::Esc
                                            | KeyCode::Char('n')
                                            | KeyCode::Char('N') => self.dialog.close(),
                                            KeyCode::Char('y')
                                            | KeyCode::Char('Y')
                                            | KeyCode::Enter => {
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
                                    KeyCode::Char('n') => {
                                        self.menu.visible = false;
                                        self.handle_menu_action(ui::menu::MenuAction::NewFile);
                                    }
                                    KeyCode::Char('N') => {
                                        self.menu.visible = false;
                                        self.handle_menu_action(ui::menu::MenuAction::NewDirectory);
                                    }
                                    KeyCode::Char('r') => {
                                        self.menu.visible = false;
                                        self.handle_menu_action(ui::menu::MenuAction::RenameFile);
                                    }
                                    KeyCode::Char('u') => {
                                        self.menu.visible = false;
                                        self.handle_menu_action(
                                            ui::menu::MenuAction::DuplicateFile,
                                        );
                                    }
                                    KeyCode::Char('c') => {
                                        self.menu.visible = false;
                                        self.handle_menu_action(ui::menu::MenuAction::CopyFileTo);
                                    }
                                    KeyCode::Char('m') => {
                                        self.menu.visible = false;
                                        self.handle_menu_action(ui::menu::MenuAction::MoveFileTo);
                                    }
                                    KeyCode::Char('d') => {
                                        self.menu.visible = false;
                                        self.handle_menu_action(ui::menu::MenuAction::DeleteFile);
                                    }
                                    KeyCode::Char('y') => {
                                        self.menu.visible = false;
                                        self.handle_menu_action(
                                            ui::menu::MenuAction::CopyAbsolutePath,
                                        );
                                    }
                                    KeyCode::Char('Y') => {
                                        self.menu.visible = false;
                                        self.handle_menu_action(
                                            ui::menu::MenuAction::CopyRelativePath,
                                        );
                                    }
                                    KeyCode::Char('g') => {
                                        self.menu.visible = false;
                                        self.handle_menu_action(ui::menu::MenuAction::GoToPath);
                                    }
                                    KeyCode::Char('i') => {
                                        self.menu.visible = false;
                                        self.handle_menu_action(
                                            ui::menu::MenuAction::AddToGitignore,
                                        );
                                    }
                                    _ => {}
                                }
                                continue;
                            }

                            // About dialog handling
                            if self.about.visible {
                                match key.code {
                                    KeyCode::Esc | KeyCode::F(10) | KeyCode::Char('q') => {
                                        self.about.close()
                                    }
                                    _ => {}
                                }
                                continue;
                            }

                            if self.help.visible {
                                // Search mode active: handle text input
                                if self.help.search_active {
                                    match key.code {
                                        KeyCode::Esc => {
                                            // Cancel search, keep query visible
                                            self.help.stop_search();
                                        }
                                        KeyCode::Enter => {
                                            // Confirm search, navigate results
                                            self.help.stop_search();
                                            self.help.scroll = 0; // Jump to first match
                                        }
                                        KeyCode::Backspace => {
                                            self.help.search_backspace();
                                            self.help.scroll = 0; // Reset scroll on query change
                                        }
                                        KeyCode::Char('u')
                                            if key.modifiers.contains(
                                                crossterm::event::KeyModifiers::CONTROL,
                                            ) =>
                                        {
                                            // Ctrl+U: Clear search
                                            self.help.clear_search();
                                        }
                                        KeyCode::Char(c) => {
                                            self.help.search_add_char(c);
                                            self.help.scroll = 0; // Reset scroll on query change
                                        }
                                        _ => {}
                                    }
                                } else {
                                    // Normal mode: navigation and search activation
                                    match key.code {
                                        KeyCode::Esc | KeyCode::F(12) | KeyCode::Char('q') => {
                                            self.help.close()
                                        }
                                        KeyCode::Char('/') | KeyCode::Char('f')
                                            if key.code == KeyCode::Char('/')
                                                || key.modifiers.contains(
                                                    crossterm::event::KeyModifiers::CONTROL,
                                                ) =>
                                        {
                                            // '/' or Ctrl+F: Start search
                                            self.help.start_search();
                                        }
                                        KeyCode::Up | KeyCode::Char('k') => self.help.scroll_up(1),
                                        KeyCode::Down | KeyCode::Char('j') => {
                                            self.help.scroll_down(1)
                                        }
                                        KeyCode::PageUp => self.help.page_up(),
                                        KeyCode::PageDown => self.help.page_down(),
                                        KeyCode::Home | KeyCode::Char('g') => {
                                            self.help.scroll_to_top()
                                        }
                                        KeyCode::End | KeyCode::Char('G') => {
                                            self.help.scroll_to_bottom()
                                        }
                                        KeyCode::Char('u') => {
                                            // Trigger manual update check from Help screen
                                            self.help.close();
                                            self.update_state.manual_check = true;
                                            self.start_update_check();
                                        }
                                        _ => {}
                                    }
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

                            // F7: Toggle between ~/.claude and previous directory
                            if key.code == KeyCode::F(7) {
                                if let Some(home) = std::env::var_os("HOME") {
                                    let claude_dir = std::path::PathBuf::from(home).join(".claude");
                                    // Already in ~/.claude? → toggle back
                                    if self.file_browser.current_dir.starts_with(&claude_dir)
                                        || self.file_browser.root_dir.starts_with(&claude_dir)
                                    {
                                        if let Some(prev) = self.file_browser.previous_dir.take() {
                                            self.file_browser.current_dir = prev;
                                            self.file_browser.load_directory();
                                            self.active_pane = PaneId::FileBrowser;
                                        }
                                    } else if claude_dir.exists() && claude_dir.is_dir() {
                                        // Save current dir, navigate to ~/.claude
                                        self.file_browser.previous_dir =
                                            Some(self.file_browser.root_dir.clone());
                                        self.file_browser.current_dir = claude_dir;
                                        self.file_browser.load_directory();
                                        self.active_pane = PaneId::FileBrowser;
                                    }
                                }
                                continue;
                            }

                            // Context-specific shortcuts (only in non-terminal panes)
                            // '?' for help - only in FileBrowser or Preview (read-only)
                            if key.code == KeyCode::Char('?')
                                && matches!(self.active_pane, PaneId::FileBrowser | PaneId::Preview)
                                && self.preview.mode != EditorMode::Edit
                            {
                                self.help.open();
                                continue;
                            }

                            if key.code == KeyCode::F(9) {
                                self.menu.toggle();
                                continue;
                            }

                            // Ctrl+P: Open fuzzy finder
                            if key.code == KeyCode::Char('p')
                                && key
                                    .modifiers
                                    .contains(crossterm::event::KeyModifiers::CONTROL)
                            {
                                self.fuzzy_finder.open(&self.file_browser.current_dir);
                                continue;
                            }

                            // F8: Open settings
                            if key.code == KeyCode::F(8) {
                                self.settings.open(&self.config);
                                continue;
                            }

                            // Ctrl+Shift+W: Re-run setup wizard
                            if key.code == KeyCode::Char('W')
                                && key.modifiers.contains(
                                    crossterm::event::KeyModifiers::CONTROL
                                        | crossterm::event::KeyModifiers::SHIFT,
                                )
                            {
                                self.wizard.open();
                                continue;
                            }

                            // Permission mode dialog handling (high priority - before Claude startup)
                            // Skip if update dialog is visible - update takes priority
                            if self.permission_mode_dialog.visible && !self.update_state.show_dialog
                            {
                                match key.code {
                                    KeyCode::Esc => {
                                        // Cancel: use saved default or fall back to Default
                                        let mode = self
                                            .config
                                            .claude
                                            .default_permission_mode
                                            .unwrap_or(ClaudePermissionMode::Default);
                                        self.permission_mode_dialog.close();
                                        if self.claude_pty_pending {
                                            self.init_claude_pty(mode);
                                        }
                                        self.active_pane = PaneId::Claude;
                                    }
                                    KeyCode::Enter => {
                                        // Confirm selected mode and save to config
                                        let mode = self.permission_mode_dialog.selected_mode();
                                        self.permission_mode_dialog.confirm();
                                        self.config.claude.default_permission_mode = Some(mode);
                                        let _ = crate::config::save_config(&self.config);
                                        if self.claude_pty_pending {
                                            self.init_claude_pty(mode);
                                        }
                                        self.active_pane = PaneId::Claude;
                                    }
                                    KeyCode::Up | KeyCode::Char('k') => {
                                        self.permission_mode_dialog.prev()
                                    }
                                    KeyCode::Down | KeyCode::Char('j') => {
                                        self.permission_mode_dialog.next()
                                    }
                                    _ => {}
                                }
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
                                        if let Some(prefix) = self.claude_startup.selected_prefix()
                                        {
                                            if !prefix.is_empty() {
                                                if let Some(pty) =
                                                    self.terminals.get_mut(&PaneId::Claude)
                                                {
                                                    let cmd = format!("{}\n", prefix);
                                                    let _ = pty.write_input(cmd.as_bytes());
                                                }
                                            }
                                        }
                                        self.claude_startup.close();
                                        self.active_pane = PaneId::Claude;
                                    }
                                    KeyCode::Up | KeyCode::Char('k') => self.claude_startup.prev(),
                                    KeyCode::Down | KeyCode::Char('j') => {
                                        self.claude_startup.next()
                                    }
                                    _ => {}
                                }
                                continue;
                            }

                            // Interactive pane resizing: Alt+Shift+Arrow
                            if key
                                .modifiers
                                .contains(KeyModifiers::ALT | KeyModifiers::SHIFT)
                            {
                                match key.code {
                                    KeyCode::Left => {
                                        self.config.layout.file_browser_width_percent = self
                                            .config
                                            .layout
                                            .file_browser_width_percent
                                            .saturating_sub(2)
                                            .max(10);
                                        let _ = crate::config::save_config(&self.config);
                                        continue;
                                    }
                                    KeyCode::Right => {
                                        self.config.layout.file_browser_width_percent =
                                            (self.config.layout.file_browser_width_percent + 2)
                                                .min(50);
                                        let _ = crate::config::save_config(&self.config);
                                        continue;
                                    }
                                    KeyCode::Up => {
                                        self.config.layout.claude_height_percent = self
                                            .config
                                            .layout
                                            .claude_height_percent
                                            .saturating_sub(2)
                                            .max(20);
                                        let _ = crate::config::save_config(&self.config);
                                        continue;
                                    }
                                    KeyCode::Down => {
                                        self.config.layout.claude_height_percent =
                                            (self.config.layout.claude_height_percent + 2).min(80);
                                        let _ = crate::config::save_config(&self.config);
                                        continue;
                                    }
                                    _ => {}
                                }
                            }

                            // Global Focus Switching
                            match key.code {
                                KeyCode::F(1) => {
                                    self.show_file_browser = !self.show_file_browser;
                                    if self.show_file_browser {
                                        self.active_pane = PaneId::FileBrowser;
                                    } else if self.active_pane == PaneId::FileBrowser {
                                        self.active_pane = PaneId::Claude;
                                    }
                                }
                                KeyCode::F(2) => {
                                    self.show_preview = !self.show_preview;
                                    if self.show_preview {
                                        self.active_pane = PaneId::Preview;
                                    } else if self.active_pane == PaneId::Preview {
                                        self.active_pane = PaneId::FileBrowser;
                                    }
                                }
                                KeyCode::F(3) => {
                                    self.file_browser.refresh();
                                    self.update_preview();
                                }
                                KeyCode::F(4) => {
                                    // Show startup dialog if prefixes configured and not yet shown
                                    if !self.claude_startup.shown_this_session
                                        && !self.config.claude.startup_prefixes.is_empty()
                                    {
                                        self.claude_startup
                                            .open(self.config.claude.startup_prefixes.clone());
                                    } else {
                                        self.active_pane = PaneId::Claude;
                                    }
                                }
                                KeyCode::F(5) => {
                                    let was_hidden = !self.show_lazygit;
                                    self.show_lazygit = !self.show_lazygit;
                                    if self.show_lazygit {
                                        self.active_pane = PaneId::LazyGit;
                                        // Restart LazyGit in current directory when showing
                                        if was_hidden {
                                            self.restart_lazygit_in_current_dir();
                                        }
                                    } else if self.active_pane == PaneId::LazyGit {
                                        self.active_pane = PaneId::Preview;
                                    }
                                }
                                KeyCode::F(6) => {
                                    let was_hidden = !self.show_terminal;
                                    self.show_terminal = !self.show_terminal;
                                    if self.show_terminal {
                                        self.active_pane = PaneId::Terminal;
                                        // Sync directory when showing terminal
                                        if was_hidden {
                                            self.sync_terminal_to_current_dir(PaneId::Terminal);
                                        }
                                    } else if self.active_pane == PaneId::Terminal {
                                        self.active_pane = PaneId::Preview;
                                    }
                                }
                                // QUIT: Ctrl+Q only (Ctrl+C goes to PTY for SIGINT)
                                KeyCode::Char('q')
                                    if key
                                        .modifiers
                                        .contains(crossterm::event::KeyModifiers::CONTROL) =>
                                {
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
                                                KeyCode::Enter
                                                | KeyCode::Right
                                                | KeyCode::Char('l') => {
                                                    if let Some(_path) =
                                                        self.file_browser.enter_selected()
                                                    {
                                                        // File opened
                                                    } else {
                                                        self.update_preview();
                                                        self.sync_terminals();
                                                        self.check_repo_change();
                                                    }
                                                }
                                                KeyCode::Backspace
                                                | KeyCode::Left
                                                | KeyCode::Char('h') => {
                                                    self.file_browser.go_parent();
                                                    self.update_preview();
                                                    self.sync_terminals();
                                                    self.check_repo_change();
                                                }
                                                // Open file in browser/external viewer
                                                KeyCode::Char('o') => {
                                                    if let Some(path) =
                                                        self.file_browser.selected_file()
                                                    {
                                                        if browser::can_preview_in_browser(&path) {
                                                            let preview_path =
                                                                if browser::is_markdown(&path) {
                                                                    browser::markdown_to_html(&path)
                                                                        .unwrap_or(path)
                                                                } else if browser::can_syntax_highlight(&path) {
                                                                    browser::text_to_html(&path)
                                                                        .unwrap_or(path)
                                                                } else {
                                                                    path
                                                                };
                                                            let _ =
                                                                browser::open_file(&preview_path);
                                                        }
                                                    }
                                                }
                                                // Open current directory in file manager
                                                KeyCode::Char('O') => {
                                                    let _ = browser::open_in_file_manager(
                                                        &self.file_browser.current_dir,
                                                    );
                                                }
                                                // Allow single q to quit if in browser
                                                KeyCode::Char('q') => {
                                                    self.should_quit = true;
                                                }
                                                // Toggle hidden files visibility
                                                KeyCode::Char('.') => {
                                                    self.file_browser.show_hidden =
                                                        !self.file_browser.show_hidden;
                                                    self.file_browser.refresh();
                                                    self.update_preview();
                                                }
                                                // Add selected file/folder to .gitignore
                                                KeyCode::Char('i') => {
                                                    if let Some(path) =
                                                        self.file_browser.selected_file()
                                                    {
                                                        self.add_to_gitignore(&path);
                                                    }
                                                }
                                                _ => {}
                                            }
                                        }

                                        PaneId::Preview => {
                                            // Search/Replace mode handling (priority over other modes)
                                            if self.preview.search.active {
                                                match key.code {
                                                    KeyCode::Esc => {
                                                        self.preview.search.close();
                                                        continue;
                                                    }
                                                    // Ctrl+H: Toggle between Search and Replace mode (when search is open)
                                                    KeyCode::Char('h')
                                                        if key
                                                            .modifiers
                                                            .contains(KeyModifiers::CONTROL) =>
                                                    {
                                                        if self.preview.mode == EditorMode::Edit {
                                                            self.preview
                                                                .search
                                                                .toggle_replace_mode();
                                                        }
                                                        continue;
                                                    }
                                                    // Tab: Switch between search/replace fields (only in Replace mode)
                                                    KeyCode::Tab => {
                                                        self.preview.search.toggle_field_focus();
                                                        continue;
                                                    }
                                                    // Ctrl+I: Toggle case sensitivity
                                                    KeyCode::Char('i')
                                                        if key
                                                            .modifiers
                                                            .contains(KeyModifiers::CONTROL) =>
                                                    {
                                                        self.preview.search.case_sensitive =
                                                            !self.preview.search.case_sensitive;
                                                        self.preview.perform_search();
                                                        continue;
                                                    }
                                                    KeyCode::Char('\x09') => {
                                                        // Ctrl+I as control char
                                                        self.preview.search.case_sensitive =
                                                            !self.preview.search.case_sensitive;
                                                        self.preview.perform_search();
                                                        continue;
                                                    }
                                                    // Enter: In Search mode = confirm and close
                                                    //        In Replace mode = replace current and move to next
                                                    KeyCode::Enter => {
                                                        if self.preview.search.mode
                                                            == SearchMode::Replace
                                                            && self.preview.mode == EditorMode::Edit
                                                        {
                                                            self.preview.replace_and_next(
                                                                &self.syntax_manager,
                                                            );
                                                        } else {
                                                            self.preview.jump_to_current_match();
                                                            self.preview.search.active = false;
                                                            // Keep query for n/N navigation
                                                        }
                                                        continue;
                                                    }
                                                    // Ctrl+R: Replace all (only in Replace mode)
                                                    KeyCode::Char('r')
                                                        if key
                                                            .modifiers
                                                            .contains(KeyModifiers::CONTROL)
                                                            && self.preview.search.mode
                                                                == SearchMode::Replace =>
                                                    {
                                                        if self.preview.mode == EditorMode::Edit {
                                                            let _count = self
                                                                .preview
                                                                .replace_all(&self.syntax_manager);
                                                            // Could show count in status
                                                        }
                                                        continue;
                                                    }
                                                    // Ctrl+N: Next match
                                                    KeyCode::Char('n')
                                                        if key
                                                            .modifiers
                                                            .contains(KeyModifiers::CONTROL) =>
                                                    {
                                                        self.preview.search.next_match();
                                                        self.preview.jump_to_current_match();
                                                        continue;
                                                    }
                                                    // Ctrl+P: Previous match
                                                    KeyCode::Char('p')
                                                        if key
                                                            .modifiers
                                                            .contains(KeyModifiers::CONTROL) =>
                                                    {
                                                        self.preview.search.prev_match();
                                                        self.preview.jump_to_current_match();
                                                        continue;
                                                    }
                                                    // ─────────────────────────────────────────
                                                    // Cursor Navigation in Search/Replace fields
                                                    // ─────────────────────────────────────────
                                                    KeyCode::Left => {
                                                        self.preview.search.cursor_left();
                                                        continue;
                                                    }
                                                    KeyCode::Right => {
                                                        self.preview.search.cursor_right();
                                                        continue;
                                                    }
                                                    KeyCode::Home => {
                                                        self.preview.search.cursor_home();
                                                        continue;
                                                    }
                                                    KeyCode::End => {
                                                        self.preview.search.cursor_end();
                                                        continue;
                                                    }
                                                    // Delete: Delete character at cursor
                                                    KeyCode::Delete => {
                                                        self.preview.search.delete_char_at();
                                                        if !self.preview.search.focus_on_replace {
                                                            self.preview.perform_search();
                                                            self.preview.jump_to_current_match();
                                                        }
                                                        continue;
                                                    }
                                                    // Backspace: Delete character before cursor
                                                    KeyCode::Backspace => {
                                                        self.preview.search.delete_char_before();
                                                        if !self.preview.search.focus_on_replace {
                                                            self.preview.perform_search();
                                                            self.preview.jump_to_current_match();
                                                        }
                                                        continue;
                                                    }
                                                    // Character input at cursor position
                                                    KeyCode::Char(c) => {
                                                        self.preview.search.insert_char(c);
                                                        if !self.preview.search.focus_on_replace {
                                                            self.preview.perform_search();
                                                            self.preview.jump_to_current_match();
                                                        }
                                                        continue;
                                                    }
                                                    _ => {
                                                        continue;
                                                    }
                                                }
                                            }

                                            // Check for search trigger (/ in read-only, Ctrl+F in any mode)
                                            let is_ctrl_f = (key.code == KeyCode::Char('f')
                                                && key.modifiers.contains(KeyModifiers::CONTROL))
                                                || key.code == KeyCode::Char('\x06'); // Ctrl+F as control char
                                            let is_slash = key.code == KeyCode::Char('/')
                                                && self.preview.mode == EditorMode::ReadOnly;
                                            // Ctrl+H opens Search & Replace directly
                                            let is_ctrl_h = (key.code == KeyCode::Char('h')
                                                && key.modifiers.contains(KeyModifiers::CONTROL))
                                                || key.code == KeyCode::Char('\x08'); // Ctrl+H as control char (backspace, but with CONTROL modifier)

                                            if is_ctrl_f || is_slash {
                                                self.preview.search.open();
                                                continue;
                                            }

                                            // Ctrl+H: Open search in Replace mode directly
                                            if is_ctrl_h && self.preview.mode == EditorMode::Edit {
                                                self.preview.search.open();
                                                self.preview.search.mode = SearchMode::Replace;
                                                continue;
                                            }

                                            // Edit mode handling
                                            if self.preview.mode == EditorMode::Edit {
                                                // Check for Ctrl+S (save) - handle both modifier and control char
                                                let is_ctrl_s = (key.code == KeyCode::Char('s')
                                                    && key
                                                        .modifiers
                                                        .contains(KeyModifiers::CONTROL))
                                                    || key.code == KeyCode::Char('\x13'); // Ctrl+S as control char
                                                                                          // Check for Ctrl+Y (delete line - MC Edit style)
                                                let is_ctrl_y = (key.code == KeyCode::Char('y')
                                                    && key
                                                        .modifiers
                                                        .contains(KeyModifiers::CONTROL))
                                                    || key.code == KeyCode::Char('\x19'); // Ctrl+Y as control char

                                                if key.code == KeyCode::Esc {
                                                    // Cancel selection first if active, then exit
                                                    if self.preview.block_marking {
                                                        self.preview.cancel_selection();
                                                    } else if self.preview.is_modified() {
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
                                                        self.preview.refresh_highlighting(
                                                            &self.syntax_manager,
                                                        );
                                                    }
                                                }
                                                // MC Edit style: Ctrl+Y = delete line
                                                else if is_ctrl_y {
                                                    self.preview.delete_line();
                                                    self.preview.update_modified();
                                                    self.preview.update_edit_highlighting(
                                                        &self.syntax_manager,
                                                    );
                                                }
                                                // Platform Copy: Cmd+C / Ctrl+C
                                                else if key.code == KeyCode::Char('c')
                                                    && (key.modifiers.contains(KeyModifiers::SUPER)
                                                        || key
                                                            .modifiers
                                                            .contains(KeyModifiers::CONTROL))
                                                {
                                                    self.preview.copy_block();
                                                }
                                                // Platform Cut: Cmd+X / Ctrl+X
                                                else if key.code == KeyCode::Char('x')
                                                    && (key.modifiers.contains(KeyModifiers::SUPER)
                                                        || key
                                                            .modifiers
                                                            .contains(KeyModifiers::CONTROL))
                                                {
                                                    self.preview.move_block();
                                                    self.preview.update_modified();
                                                    self.preview.update_edit_highlighting(
                                                        &self.syntax_manager,
                                                    );
                                                }
                                                // Platform Paste: Cmd+V / Ctrl+V
                                                else if key.code == KeyCode::Char('v')
                                                    && (key.modifiers.contains(KeyModifiers::SUPER)
                                                        || key
                                                            .modifiers
                                                            .contains(KeyModifiers::CONTROL))
                                                {
                                                    self.preview.paste_from_clipboard();
                                                    self.preview.update_modified();
                                                    self.preview.update_edit_highlighting(
                                                        &self.syntax_manager,
                                                    );
                                                }
                                                // MC Edit style: Ctrl+F3 = toggle block marking
                                                else if key.code == KeyCode::F(3)
                                                    && key.modifiers.contains(KeyModifiers::CONTROL)
                                                {
                                                    self.preview.toggle_block_marking();
                                                }
                                                // MC Edit style: Ctrl+F5 = copy block
                                                else if key.code == KeyCode::F(5)
                                                    && key.modifiers.contains(KeyModifiers::CONTROL)
                                                {
                                                    self.preview.copy_block();
                                                    self.preview.update_modified();
                                                    self.preview.update_edit_highlighting(
                                                        &self.syntax_manager,
                                                    );
                                                }
                                                // MC Edit style: Ctrl+F6 = move (cut) block
                                                else if key.code == KeyCode::F(6)
                                                    && key.modifiers.contains(KeyModifiers::CONTROL)
                                                {
                                                    self.preview.move_block();
                                                    self.preview.update_modified();
                                                    self.preview.update_edit_highlighting(
                                                        &self.syntax_manager,
                                                    );
                                                }
                                                // MC Edit style: Ctrl+F8 = delete block
                                                else if key.code == KeyCode::F(8)
                                                    && key.modifiers.contains(KeyModifiers::CONTROL)
                                                {
                                                    self.preview.delete_block();
                                                    self.preview.update_modified();
                                                    self.preview.update_edit_highlighting(
                                                        &self.syntax_manager,
                                                    );
                                                }
                                                // MC Edit style: Shift+Arrow = extend selection
                                                else if key
                                                    .modifiers
                                                    .contains(KeyModifiers::SHIFT)
                                                {
                                                    use tui_textarea::CursorMove;
                                                    match key.code {
                                                        KeyCode::Up => {
                                                            self.preview
                                                                .extend_selection(CursorMove::Up);
                                                        }
                                                        KeyCode::Down => {
                                                            self.preview
                                                                .extend_selection(CursorMove::Down);
                                                        }
                                                        KeyCode::Left => {
                                                            self.preview
                                                                .extend_selection(CursorMove::Back);
                                                        }
                                                        KeyCode::Right => {
                                                            self.preview.extend_selection(
                                                                CursorMove::Forward,
                                                            );
                                                        }
                                                        _ => {
                                                            // Forward other Shift+key combos to TextArea
                                                            if let Some(editor) =
                                                                &mut self.preview.editor
                                                            {
                                                                editor.input(Event::Key(key));
                                                                self.preview.update_modified();
                                                                self.preview
                                                                    .update_edit_highlighting(
                                                                        &self.syntax_manager,
                                                                    );
                                                            }
                                                        }
                                                    }
                                                } else {
                                                    // Handle scrolling keys specially (TextArea moves cursor, not view)
                                                    match key.code {
                                                        KeyCode::PageUp => {
                                                            if let Some(editor) =
                                                                &mut self.preview.editor
                                                            {
                                                                // Move cursor up by ~20 lines to simulate page scroll
                                                                for _ in 0..20 {
                                                                    editor.move_cursor(tui_textarea::CursorMove::Up);
                                                                }
                                                            }
                                                        }
                                                        KeyCode::PageDown => {
                                                            if let Some(editor) =
                                                                &mut self.preview.editor
                                                            {
                                                                for _ in 0..20 {
                                                                    editor.move_cursor(tui_textarea::CursorMove::Down);
                                                                }
                                                            }
                                                        }
                                                        _ => {
                                                            // Forward other keys to TextArea (handles Ctrl+Z undo, etc.)
                                                            if let Some(editor) =
                                                                &mut self.preview.editor
                                                            {
                                                                editor.input(Event::Key(key));
                                                                self.preview.update_modified();
                                                                // Update syntax highlighting for edit mode
                                                                self.preview
                                                                    .update_edit_highlighting(
                                                                        &self.syntax_manager,
                                                                    );
                                                            }
                                                            // Auto-adjust horizontal scroll to follow cursor
                                                            if let Some(editor) =
                                                                &self.preview.editor
                                                            {
                                                                let (_, cursor_col) =
                                                                    editor.cursor();
                                                                let visible_width =
                                                                    self.preview_width as usize;
                                                                let h_scroll =
                                                                    self.preview.horizontal_scroll
                                                                        as usize;
                                                                if visible_width > 0
                                                                    && cursor_col
                                                                        >= h_scroll
                                                                            + visible_width
                                                                                .saturating_sub(5)
                                                                {
                                                                    self.preview
                                                                        .horizontal_scroll =
                                                                        (cursor_col.saturating_sub(
                                                                            visible_width / 2,
                                                                        ))
                                                                            as u16;
                                                                } else if cursor_col < h_scroll {
                                                                    self.preview
                                                                        .horizontal_scroll =
                                                                        cursor_col.saturating_sub(5)
                                                                            as u16;
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            } else {
                                                // Read-only mode - check for selection mode first
                                                if self.terminal_selection.active
                                                    && self.terminal_selection.source_pane
                                                        == Some(PaneId::Preview)
                                                {
                                                    // Selection mode active in Preview
                                                    match key.code {
                                                        KeyCode::Up | KeyCode::Char('k') => {
                                                            if let Some(end) =
                                                                self.terminal_selection.end_line
                                                            {
                                                                self.terminal_selection
                                                                    .extend(end.saturating_sub(1));
                                                            }
                                                            continue;
                                                        }
                                                        KeyCode::Down | KeyCode::Char('j') => {
                                                            if let Some(end) =
                                                                self.terminal_selection.end_line
                                                            {
                                                                self.terminal_selection
                                                                    .extend(end + 1);
                                                            }
                                                            continue;
                                                        }
                                                        KeyCode::Enter | KeyCode::Char('y') => {
                                                            self.copy_selection_to_claude();
                                                            self.terminal_selection.clear();
                                                            continue;
                                                        }
                                                        // Ctrl+C / Cmd+C: Copy selection to system clipboard
                                                        KeyCode::Char('c')
                                                            if key.modifiers.contains(
                                                                KeyModifiers::CONTROL,
                                                            ) || key
                                                                .modifiers
                                                                .contains(KeyModifiers::SUPER) =>
                                                        {
                                                            self.copy_selection_to_clipboard();
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
                                                    let is_ctrl_s = (key.code
                                                        == KeyCode::Char('s')
                                                        && key
                                                            .modifiers
                                                            .contains(KeyModifiers::CONTROL))
                                                        || key.code == KeyCode::Char('\x13');
                                                    if is_ctrl_s {
                                                        // Start keyboard selection at current scroll position
                                                        self.terminal_selection.start(
                                                            self.preview.scroll as usize,
                                                            PaneId::Preview,
                                                        );
                                                        continue;
                                                    }

                                                    match key.code {
                                                        KeyCode::Down | KeyCode::Char('j') => {
                                                            self.preview.scroll_down()
                                                        }
                                                        KeyCode::Up | KeyCode::Char('k') => {
                                                            self.preview.scroll_up()
                                                        }
                                                        KeyCode::Left | KeyCode::Char('h') => {
                                                            self.preview.scroll_left();
                                                        }
                                                        KeyCode::Right | KeyCode::Char('l') => {
                                                            let max = self.preview.max_line_width();
                                                            self.preview.scroll_right(max);
                                                        }
                                                        KeyCode::PageDown => {
                                                            for _ in 0..10 {
                                                                self.preview.scroll_down();
                                                            }
                                                        }
                                                        KeyCode::PageUp => {
                                                            for _ in 0..10 {
                                                                self.preview.scroll_up();
                                                            }
                                                        }
                                                        KeyCode::Home => {
                                                            self.preview.scroll = 0;
                                                        }
                                                        KeyCode::End => {
                                                            let max = self
                                                                .preview
                                                                .highlighted_lines
                                                                .len()
                                                                .saturating_sub(1)
                                                                as u16;
                                                            self.preview.scroll = max;
                                                        }
                                                        KeyCode::Char('e') | KeyCode::Char('E') => {
                                                            self.preview.enter_edit_mode();
                                                        }
                                                        // Search navigation: n = next match, N = previous match
                                                        KeyCode::Char('n')
                                                            if !self
                                                                .preview
                                                                .search
                                                                .matches
                                                                .is_empty() =>
                                                        {
                                                            self.preview.search.next_match();
                                                            self.preview.jump_to_current_match();
                                                        }
                                                        KeyCode::Char('N')
                                                            if !self
                                                                .preview
                                                                .search
                                                                .matches
                                                                .is_empty() =>
                                                        {
                                                            self.preview.search.prev_match();
                                                            self.preview.jump_to_current_match();
                                                        }
                                                        _ => {}
                                                    }
                                                }
                                            }
                                        }
                                        PaneId::Terminal | PaneId::Claude | PaneId::LazyGit => {
                                            // Terminal selection mode handling
                                            if self.terminal_selection.active
                                                && self.terminal_selection.source_pane
                                                    == Some(self.active_pane)
                                            {
                                                match key.code {
                                                    KeyCode::Up | KeyCode::Char('k') => {
                                                        if let Some(end) =
                                                            self.terminal_selection.end_line
                                                        {
                                                            self.terminal_selection
                                                                .extend(end.saturating_sub(1));
                                                        }
                                                        continue;
                                                    }
                                                    KeyCode::Down | KeyCode::Char('j') => {
                                                        if let Some(end) =
                                                            self.terminal_selection.end_line
                                                        {
                                                            self.terminal_selection.extend(end + 1);
                                                        }
                                                        continue;
                                                    }
                                                    KeyCode::Enter | KeyCode::Char('y') => {
                                                        self.copy_selection_to_claude();
                                                        self.terminal_selection.clear();
                                                        continue;
                                                    }
                                                    // Ctrl+C / Cmd+C: Copy selection to system clipboard
                                                    KeyCode::Char('c')
                                                        if key
                                                            .modifiers
                                                            .contains(KeyModifiers::CONTROL)
                                                            || key
                                                                .modifiers
                                                                .contains(KeyModifiers::SUPER) =>
                                                    {
                                                        self.copy_selection_to_clipboard();
                                                        self.terminal_selection.clear();
                                                        continue;
                                                    }
                                                    KeyCode::Esc => {
                                                        self.terminal_selection.clear();
                                                        continue;
                                                    }
                                                    _ => {
                                                        // Let other keys pass through to PTY
                                                    }
                                                }
                                            }

                                            // Ctrl+S: Start terminal selection mode
                                            if key.code == KeyCode::Char('s')
                                                && key.modifiers.contains(KeyModifiers::CONTROL)
                                            {
                                                if let Some(pty) =
                                                    self.terminals.get(&self.active_pane)
                                                {
                                                    let cursor_row = pty.cursor_row() as usize;
                                                    self.terminal_selection
                                                        .start(cursor_row, self.active_pane);
                                                }
                                                continue;
                                            }

                                            if let Some(pty) = self.terminals.get(&self.active_pane)
                                            {
                                                // Check if PTY has exited and auto_restart is disabled
                                                if pty.has_exited() && !self.config.pty.auto_restart
                                                {
                                                    // Manual restart on Enter
                                                    if key.code == KeyCode::Enter {
                                                        self.restart_single_pty(self.active_pane);
                                                    }
                                                    continue;
                                                }
                                            }

                                            if let Some(pty) =
                                                self.terminals.get_mut(&self.active_pane)
                                            {
                                                // Scroll Handling
                                                if key
                                                    .modifiers
                                                    .contains(crossterm::event::KeyModifiers::SHIFT)
                                                {
                                                    match key.code {
                                                        KeyCode::PageUp => {
                                                            pty.scroll_up(10);
                                                            continue;
                                                        }
                                                        KeyCode::PageDown => {
                                                            pty.scroll_down(10);
                                                            continue;
                                                        }
                                                        KeyCode::Up => {
                                                            pty.scroll_up(1);
                                                            continue;
                                                        }
                                                        KeyCode::Down => {
                                                            pty.scroll_down(1);
                                                            continue;
                                                        }
                                                        _ => {}
                                                    }
                                                }

                                                if let Some(bytes) =
                                                    crate::input::map_key_to_pty(key)
                                                {
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
                            PaneId::Claude => {
                                // Claude CLI doesn't understand bracketed paste sequences
                                // Send text directly - for multiline, user must use \ continuation
                                if let Some(pty) = self.terminals.get_mut(&PaneId::Claude) {
                                    let _ = pty.write_input(text.as_bytes());
                                }
                            }
                            PaneId::LazyGit | PaneId::Terminal => {
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
        Ok(self.should_restart)
    }

    /// Handle scrollbar drag: convert mouse Y position to scroll position for a pane
    fn handle_scrollbar_position(&mut self, pane: PaneId, y: u16, sb: Rect) {
        let clamped = y.clamp(sb.y, sb.y + sb.height.saturating_sub(1));
        let ratio = (clamped - sb.y) as f64 / sb.height.max(1) as f64;

        match pane {
            PaneId::Preview => {
                if self.preview.mode == EditorMode::Edit {
                    // In Edit mode, scroll_offset is derived from cursor position,
                    // so we must move the cursor to the target line for scrolling to work
                    if let Some(editor) = &mut self.preview.editor {
                        let total = editor.lines().len();
                        let target_line =
                            ((ratio * total as f64) as usize).min(total.saturating_sub(1));
                        editor.move_cursor(tui_textarea::CursorMove::Jump(target_line as u16, 0));
                    }
                } else {
                    let total = self.preview.highlighted_lines.len();
                    self.preview.scroll = (ratio * total as f64) as u16;
                }
            }
            PaneId::FileBrowser => {
                let total = self.file_browser.entries.len();
                let idx = ((ratio * total as f64) as usize).min(total.saturating_sub(1));
                self.file_browser.list_state.select(Some(idx));
            }
            PaneId::Claude | PaneId::LazyGit | PaneId::Terminal => {
                if let Some(pty) = self.terminals.get(&pane) {
                    pty.set_scrollback_position(ratio);
                }
            }
        }
    }

    /// Handle horizontal scrollbar drag: convert mouse X position to horizontal scroll
    fn handle_horizontal_scrollbar_position(&mut self, x: u16, hsb: Rect) {
        let clamped = x.clamp(hsb.x, hsb.x + hsb.width.saturating_sub(1));
        let ratio = (clamped - hsb.x) as f64 / hsb.width.max(1) as f64;
        let max_width = if self.preview.mode == EditorMode::Edit {
            self.preview.edit_max_display_width() as usize
        } else {
            self.preview.max_line_width() as usize
        };
        self.preview.horizontal_scroll = (ratio * max_width as f64) as u16;
    }

    fn draw(&mut self, frame: &mut Frame) {
        let area = frame.area();
        let (files, preview, claude, lazygit, terminal, footer) = ui::layout::compute_layout(
            area,
            self.show_file_browser,
            self.show_terminal,
            self.show_lazygit,
            self.show_preview,
            &self.config.layout,
        );

        // Helper to resize PTY
        // We need to account for borders (1px each side => -2)
        // Ensure strictly positive
        let resize_pty =
            |terminals: &mut HashMap<PaneId, PseudoTerminal>, id: PaneId, rect: Rect| {
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

        // Cache border positions for interactive pane resizing
        self.border_areas.total_width = area.width;
        self.border_areas.total_height = area.height;
        self.border_areas.file_preview_x = if files.width > 0 && preview.width > 0 {
            Some(files.x + files.width)
        } else {
            None
        };
        self.border_areas.preview_right_x =
            if preview.width > 0 && (lazygit.width > 0 || terminal.width > 0) {
                Some(preview.x + preview.width)
            } else {
                None
            };
        self.border_areas.top_claude_y = if claude.height > 0 && claude.y > 0 {
            Some(claude.y)
        } else {
            None
        };

        // Calculate scrollbar areas for drag support (right edge inside borders)
        let sb_area = |rect: Rect| -> Option<Rect> {
            if rect.width > 2 && rect.height > 2 {
                Some(Rect::new(
                    rect.x + rect.width.saturating_sub(1),
                    rect.y + 1,
                    1,
                    rect.height.saturating_sub(2),
                ))
            } else {
                None
            }
        };
        self.scrollbar_areas.file_browser = sb_area(files);
        self.scrollbar_areas.preview = sb_area(preview);
        self.scrollbar_areas.claude = sb_area(claude);
        self.scrollbar_areas.lazygit = sb_area(lazygit);
        self.scrollbar_areas.terminal = sb_area(terminal);
        // Use the cached horizontal scrollbar area from the actual render pass
        // (accounts for gutter width, content area, and scrollbar visibility)
        self.scrollbar_areas.preview_horizontal = self.preview.cached_h_scrollbar_area;
        // Cache preview width for horizontal scroll auto-adjust
        self.preview_width = preview.width.saturating_sub(10); // Account for gutter+borders

        if self.show_file_browser {
            ui::file_browser::render(
                frame,
                files,
                &mut self.file_browser,
                self.active_pane == PaneId::FileBrowser,
            );
        }

        // Calculate Preview selection range (keyboard or mouse selection)
        // Keyboard selection is line-based
        let preview_selection_range = if self.terminal_selection.active
            && self.terminal_selection.source_pane == Some(PaneId::Preview)
        {
            self.terminal_selection.line_range()
        } else {
            None
        };
        // Mouse selection is character-based
        let preview_char_selection = if self.mouse_selection.is_selecting_in(PaneId::Preview) {
            self.mouse_selection.char_range()
        } else {
            None
        };
        if self.show_preview {
            ui::preview::render(
                frame,
                preview,
                &mut self.preview,
                self.active_pane == PaneId::Preview,
                preview_selection_range,
                preview_char_selection,
            );
        }

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

        // Update dialog - render on top of most things
        if self.update_state.show_dialog {
            self.update_dialog_areas = ui::update_dialog::render(
                frame,
                area,
                &self.update_state,
                self.update_dialog_button,
            );
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

        // Permission mode dialog (render on top, before drag ghost)
        // Don't show if update dialog is visible - update takes priority
        if self.permission_mode_dialog.visible && !self.update_state.show_dialog {
            ui::permission_mode::render(frame, area, &self.permission_mode_dialog);
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

    /// Send cd command to a specific terminal pane
    fn sync_terminal_to_current_dir(&mut self, pane: PaneId) {
        let path_str = self.file_browser.current_dir.to_string_lossy();
        let escaped = escape(Cow::Borrowed(&path_str));
        let cmd = format!("cd {}\r", escaped);

        if let Some(pty) = self.terminals.get_mut(&pane) {
            let _ = pty.write_input(cmd.as_bytes());
        }
    }

    /// Restart LazyGit PTY in current directory
    fn restart_lazygit_in_current_dir(&mut self) {
        let cwd = self.file_browser.current_dir.clone();
        // Use default size, will be resized on first draw
        let rows = 24;
        let cols = 80;

        // Get lazygit command from config
        let lazygit_cmd = if self.config.pty.lazygit_command.is_empty() {
            vec!["lazygit".to_string()]
        } else {
            self.config.pty.lazygit_command.clone()
        };

        // Remove old PTY
        self.terminals.remove(&PaneId::LazyGit);

        // Create new PTY in current directory
        if let Ok(pty) = PseudoTerminal::new(&lazygit_cmd, rows, cols, &cwd) {
            self.terminals.insert(PaneId::LazyGit, pty);
        }
    }

    /// Check if we've entered a different Git repository and start async remote check
    fn check_repo_change(&mut self) {
        // Find current repo root
        let current_repo = git::find_repo_root(&self.file_browser.current_dir);

        // Check if repo changed
        let repo_changed = match (&current_repo, &self.git_remote.last_repo_root) {
            (Some(curr), Some(last)) => curr != last,
            (Some(_), None) => true,  // Entered a repo
            (None, Some(_)) => false, // Left a repo, don't check
            (None, None) => false,    // Still no repo
        };

        if repo_changed && !self.git_remote.checking {
            if let Some(repo_root) = &current_repo {
                // Get current branch
                if let Some(branch) = git::get_current_branch(repo_root) {
                    // Start async check
                    self.git_remote.checking = true;
                    self.git_check_receiver =
                        Some(git::check_remote_changes_async(repo_root, &branch));
                }
            }
        }

        // Update last repo
        self.git_remote.last_repo_root = current_repo;
    }

    /// Add selected file/folder to .gitignore and open it in editor
    fn add_to_gitignore(&mut self, path: &std::path::Path) {
        use std::fs::OpenOptions;
        use std::io::Write;

        // 1. Find .gitignore path (in Git root or current dir)
        let gitignore_path = if let Some(repo_root) = &self.file_browser.repo_root {
            repo_root.join(".gitignore")
        } else {
            self.file_browser.current_dir.join(".gitignore")
        };

        // 2. Compute relative path from repo root (or just filename)
        let relative_path = if let Some(repo_root) = &self.file_browser.repo_root {
            path.strip_prefix(repo_root)
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|_| {
                    path.file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_default()
                })
        } else {
            path.file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default()
        };

        // Skip if empty
        if relative_path.is_empty() {
            return;
        }

        // 3. Append entry to .gitignore
        if let Ok(mut file) = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&gitignore_path)
        {
            // Add newline before entry if file exists and doesn't end with newline
            if gitignore_path.exists() {
                if let Ok(content) = std::fs::read_to_string(&gitignore_path) {
                    if !content.is_empty() && !content.ends_with('\n') {
                        let _ = writeln!(file);
                    }
                }
            }

            // Add the path (with trailing slash for directories)
            if path.is_dir() {
                let _ = writeln!(file, "{}/", relative_path);
            } else {
                let _ = writeln!(file, "{}", relative_path);
            }
        }

        // 4. Open .gitignore in preview and enter edit mode
        self.preview.load_file(gitignore_path, &self.syntax_manager);
        self.preview.enter_edit_mode();
        self.active_pane = PaneId::Preview;

        // 5. Scroll to bottom to show new entry
        if let Some(editor) = &mut self.preview.editor {
            let line_count = editor.lines().len();
            if line_count > 0 {
                editor.move_cursor(tui_textarea::CursorMove::Jump((line_count - 1) as u16, 0));
            }
        }

        // 6. Refresh file browser to update git status
        self.file_browser.refresh();
    }

    /// Poll for git remote check results and show dialog if needed
    fn poll_git_check(&mut self) {
        if let Some(ref receiver) = self.git_check_receiver {
            // Non-blocking check for result
            if let Ok(result) = receiver.try_recv() {
                self.git_remote.checking = false;

                match result {
                    GitRemoteCheckResult::RemoteAhead {
                        commits_ahead,
                        branch,
                    } => {
                        // Show pull confirmation dialog
                        if let Some(repo_root) = self.git_remote.last_repo_root.clone() {
                            use ui::dialog::{DialogAction, DialogType};
                            self.dialog.dialog_type = DialogType::Confirm {
                                title: "Git Pull".to_string(),
                                message: format!(
                                    "Branch '{}' is {} commit{} behind remote. Pull now?",
                                    branch,
                                    commits_ahead,
                                    if commits_ahead == 1 { "" } else { "s" }
                                ),
                                action: DialogAction::GitPull { repo_root },
                            };
                        }
                    }
                    GitRemoteCheckResult::UpToDate => {
                        // No action needed
                    }
                    GitRemoteCheckResult::Error(_err) => {
                        // Silently ignore errors (no network, no remote, etc.)
                    }
                }

                // Clear receiver
                self.git_check_receiver = None;
            }
        }
    }

    fn handle_menu_action(&mut self, action: ui::menu::MenuAction) {
        use ui::dialog::{DialogAction, DialogType};
        use ui::menu::MenuAction;

        match action {
            MenuAction::NewFile => {
                self.dialog.dialog_type = DialogType::Input {
                    title: "New File".to_string(),
                    value: String::new(),
                    cursor: 0,
                    action: DialogAction::NewFile,
                };
            }
            MenuAction::NewDirectory => {
                self.dialog.dialog_type = DialogType::Input {
                    title: "New Directory".to_string(),
                    value: String::new(),
                    cursor: 0,
                    action: DialogAction::NewDirectory,
                };
            }
            MenuAction::RenameFile => {
                if let Some(selected) = self.file_browser.selected_file() {
                    let name = selected
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_default();
                    let cursor_pos = name.chars().count();
                    self.dialog.dialog_type = DialogType::Input {
                        title: "Rename".to_string(),
                        value: name,
                        cursor: cursor_pos,
                        action: DialogAction::RenameFile { old_path: selected },
                    };
                }
            }
            MenuAction::DuplicateFile => {
                if let Some(selected) = self.file_browser.selected_file() {
                    if selected.is_file() {
                        // Generate duplicate name with counter
                        let parent = selected.parent().unwrap_or(&self.file_browser.current_dir);
                        let stem = selected
                            .file_stem()
                            .map(|s| s.to_string_lossy().to_string())
                            .unwrap_or_default();
                        let ext = selected
                            .extension()
                            .map(|e| format!(".{}", e.to_string_lossy()))
                            .unwrap_or_default();

                        let mut counter = 1;
                        let mut new_path;
                        loop {
                            let new_name = format!("{} - Duplikat {}{}", stem, counter, ext);
                            new_path = parent.join(&new_name);
                            if !new_path.exists() {
                                break;
                            }
                            counter += 1;
                        }

                        if let Err(e) = std::fs::copy(&selected, &new_path) {
                            eprintln!("Failed to duplicate file: {}", e);
                        } else {
                            self.file_browser.refresh();
                            self.update_preview();
                        }
                    }
                }
            }
            MenuAction::CopyFileTo => {
                if let Some(selected) = self.file_browser.selected_file() {
                    if selected.is_file() {
                        let dir_str = self.file_browser.current_dir.to_string_lossy().to_string();
                        let cursor_pos = dir_str.chars().count();
                        self.dialog.dialog_type = DialogType::Input {
                            title: "Copy to".to_string(),
                            value: dir_str,
                            cursor: cursor_pos,
                            action: DialogAction::CopyFileTo { source: selected },
                        };
                    }
                }
            }
            MenuAction::MoveFileTo => {
                if let Some(selected) = self.file_browser.selected_file() {
                    if selected.is_file() {
                        let dir_str = self.file_browser.current_dir.to_string_lossy().to_string();
                        let cursor_pos = dir_str.chars().count();
                        self.dialog.dialog_type = DialogType::Input {
                            title: "Move to".to_string(),
                            value: dir_str,
                            cursor: cursor_pos,
                            action: DialogAction::MoveFileTo { source: selected },
                        };
                    }
                }
            }
            MenuAction::DeleteFile => {
                if let Some(selected) = self.file_browser.selected_file() {
                    if selected.file_name().map(|n| n.to_string_lossy()) != Some("..".into()) {
                        let name = selected
                            .file_name()
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
            MenuAction::GoToPath => {
                let dir_str = self.file_browser.current_dir.to_string_lossy().to_string();
                let cursor_pos = dir_str.chars().count();
                self.dialog.dialog_type = ui::dialog::DialogType::Input {
                    title: "Go to Path".to_string(),
                    value: dir_str,
                    cursor: cursor_pos,
                    action: ui::dialog::DialogAction::GoToPath,
                };
            }
            MenuAction::AddToGitignore => {
                if let Some(path) = self.file_browser.selected_file() {
                    self.add_to_gitignore(&path);
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
            DialogAction::NewDirectory => {
                if let Some(name) = value {
                    if !name.is_empty() {
                        let new_path = self.file_browser.current_dir.join(&name);
                        if let Err(e) = std::fs::create_dir(&new_path) {
                            eprintln!("Failed to create directory: {}", e);
                        } else {
                            self.file_browser.refresh();
                        }
                    }
                }
            }
            DialogAction::RenameFile { old_path } => {
                if let Some(new_name) = value {
                    if !new_name.is_empty() {
                        let new_path = old_path
                            .parent()
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
            DialogAction::CopyFileTo { source } => {
                if let Some(dest_dir) = value {
                    if !dest_dir.is_empty() {
                        let dest_path = std::path::Path::new(&dest_dir);
                        if dest_path.is_dir() {
                            if let Some(filename) = source.file_name() {
                                let target = dest_path.join(filename);
                                if let Err(e) = std::fs::copy(&source, &target) {
                                    eprintln!("Failed to copy file: {}", e);
                                } else {
                                    self.file_browser.refresh();
                                }
                            }
                        }
                    }
                }
            }
            DialogAction::MoveFileTo { source } => {
                if let Some(dest_dir) = value {
                    if !dest_dir.is_empty() {
                        let dest_path = std::path::Path::new(&dest_dir);
                        if dest_path.is_dir() {
                            if let Some(filename) = source.file_name() {
                                let target = dest_path.join(filename);
                                if let Err(e) = std::fs::rename(&source, &target) {
                                    eprintln!("Failed to move file: {}", e);
                                } else {
                                    self.file_browser.refresh();
                                }
                            }
                        }
                    }
                }
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
                self.check_repo_change();
            }
            DialogAction::GitPull { repo_root } => {
                // Execute git pull
                match git::pull(&repo_root) {
                    Ok(output) => {
                        // Show success dialog with first 2 lines of output
                        let summary: String = output.lines().take(2).collect::<Vec<_>>().join("\n");
                        self.dialog.dialog_type = ui::dialog::DialogType::Confirm {
                            title: "Git Pull".to_string(),
                            message: format!("✓ Pull successful!\n{}", summary),
                            action: DialogAction::GoToPath, // Dummy action - just closes on confirm
                        };
                        // Refresh file browser to show any new/changed files
                        self.file_browser.refresh();
                    }
                    Err(err) => {
                        // Show error dialog
                        self.dialog.dialog_type = ui::dialog::DialogType::Confirm {
                            title: "Git Pull Error".to_string(),
                            message: format!("✗ Pull failed:\n{}", err),
                            action: DialogAction::GoToPath, // Dummy action - just closes on confirm
                        };
                    }
                }
            }
            DialogAction::GoToPath => {
                if let Some(path_str) = value {
                    if !path_str.is_empty() {
                        let target = std::path::Path::new(&path_str);
                        if target.is_dir() {
                            self.file_browser.current_dir = target.to_path_buf();
                            self.file_browser.load_directory();
                            self.update_preview();
                            self.sync_terminals();
                            self.check_repo_change();
                        } else if target.is_file() {
                            if let Some(parent) = target.parent() {
                                self.file_browser.current_dir = parent.to_path_buf();
                                self.file_browser.load_directory();
                                // Select the file
                                if let Some(name) = target.file_name() {
                                    let name_str = name.to_string_lossy().to_string();
                                    for (idx, entry) in self.file_browser.entries.iter().enumerate()
                                    {
                                        if entry.name == name_str {
                                            self.file_browser.list_state.select(Some(idx));
                                            break;
                                        }
                                    }
                                }
                                self.update_preview();
                                self.sync_terminals();
                                self.check_repo_change();
                            }
                        }
                    }
                }
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
                KeyCode::Backspace => {
                    self.wizard.input_buffer.pop();
                }
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
                    // Close wizard and initialize Claude PTY with new config
                    self.wizard.close();
                    self.init_claude_after_wizard();
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
            KeyCode::Up | KeyCode::Char('k') => match self.wizard.step {
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
            },
            KeyCode::Down | KeyCode::Char('j') => match self.wizard.step {
                WizardStep::ShellSelection => {
                    if self.wizard.selected_shell_idx
                        < self.wizard.available_shells.len().saturating_sub(1)
                    {
                        self.wizard.selected_shell_idx += 1;
                    }
                }
                WizardStep::ClaudeConfig => {
                    if self.wizard.focused_field < 1 {
                        self.wizard.focused_field += 1;
                    }
                }
                _ => {}
            },
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
                KeyCode::Backspace => {
                    self.settings.input_buffer.pop();
                }
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
            KeyCode::Enter | KeyCode::Char(' ') => {
                // Check if "Check for Updates" is selected
                if self.settings.is_check_updates_selected() {
                    self.settings.close();
                    self.trigger_update_check();
                } else {
                    self.settings.toggle_or_select();
                }
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
            let content_lines: Vec<String> =
                self.preview.content.lines().map(String::from).collect();
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
        let formatted = format!("```{}\n{}\n```\n", syntax, formatted_lines.join("\n"));

        // Send to Claude PTY
        if let Some(claude_pty) = self.terminals.get_mut(&PaneId::Claude) {
            let _ = claude_pty.write_input(formatted.as_bytes());
        }
    }

    /// Copy selected lines to system clipboard (from terminal or preview)
    fn copy_selection_to_clipboard(&mut self) {
        let Some((start, end)) = self.terminal_selection.line_range() else {
            return;
        };

        let Some(source_pane) = self.terminal_selection.source_pane else {
            return;
        };

        // Extract lines from source (terminal or preview)
        let lines = if source_pane == PaneId::Preview {
            // Extract lines from preview content
            let content_lines: Vec<String> =
                self.preview.content.lines().map(String::from).collect();
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

        // Join lines and copy to system clipboard
        let text = lines.join("\n");

        if let Ok(mut clipboard) = arboard::Clipboard::new() {
            let _ = clipboard.set_text(text);
        }
    }

    /// Copy character-level mouse selection to system clipboard
    fn copy_mouse_selection_to_clipboard(&mut self) {
        let Some(((start_row, start_col), (end_row, end_col))) = self.mouse_selection.char_range()
        else {
            return;
        };

        let Some(source_pane) = self.mouse_selection.source_pane else {
            return;
        };

        // Extract text based on source pane type
        let text = if source_pane == PaneId::Preview {
            // Extract from preview content with character-level selection
            let content_lines: Vec<&str> = self.preview.content.lines().collect();

            // (C) Scroll offset: char_range() returns screen-relative rows
            let scroll_offset = self.preview.scroll as usize;
            let adj_start_row = start_row + scroll_offset;
            let adj_end_row = end_row + scroll_offset;

            if adj_start_row >= content_lines.len() {
                return;
            }

            // (B) Subtract gutter width from column indices
            let gutter_w = ui::preview::calculate_gutter_width(content_lines.len()) as usize;
            let base_start_col = start_col.saturating_sub(gutter_w);
            let base_end_col = end_col.saturating_sub(gutter_w);

            // (D) Add horizontal scroll offset
            let h_scroll = self.preview.horizontal_scroll as usize;
            let adj_start_col = base_start_col + h_scroll;
            let adj_end_col = base_end_col + h_scroll;

            let mut result = String::new();
            #[allow(clippy::needless_range_loop)] // row index needed for start/end column logic
            for row in adj_start_row..=adj_end_row.min(content_lines.len().saturating_sub(1)) {
                let line = content_lines[row];
                let line_chars: Vec<char> = line.chars().collect();

                let col_start = if row == adj_start_row {
                    adj_start_col.min(line_chars.len())
                } else {
                    0
                };
                let col_end = if row == adj_end_row {
                    (adj_end_col + 1).min(line_chars.len()) // +1 because end_col is inclusive
                } else {
                    line_chars.len()
                };

                let selected: String = line_chars[col_start..col_end].iter().collect();
                // (A) No trim_end() — preserve original content including spaces
                result.push_str(&selected);
                if row != adj_end_row {
                    result.push('\n');
                }
            }
            result
        } else if let Some(pty) = self.terminals.get(&source_pane) {
            // Extract from terminal with character-level selection
            pty.extract_char_range(start_row, start_col, end_row, end_col + 1) // +1 for inclusive end
        } else {
            return;
        };

        if text.is_empty() {
            return;
        }

        // Copy to system clipboard
        if let Ok(mut clipboard) = arboard::Clipboard::new() {
            let _ = clipboard.set_text(text);
        }
    }

    /// Position preview editor cursor based on mouse click coordinates
    fn position_preview_cursor(&mut self, area: Rect, click_x: u16, click_y: u16) {
        use tui_textarea::CursorMove;

        let Some(editor) = &mut self.preview.editor else {
            return;
        };

        let total_lines = editor.lines().len();

        // Calculate gutter width (same formula as in preview.rs)
        let gutter_width = if total_lines == 0 {
            4u16
        } else {
            let digits = ((total_lines as f64).log10().floor() as u16) + 1;
            digits + 3 // " " + digits + " │"
        };

        // Edit mode has a shortcut bar at the bottom (1 line)
        let shortcut_bar_height = 1u16;
        let editor_area_height = area.height.saturating_sub(shortcut_bar_height);

        // Account for block border (1px on each side)
        let inner_x = area.x + 1;
        let inner_y = area.y + 1;
        let mut inner_height = editor_area_height.saturating_sub(2);
        // Account for horizontal scrollbar if present
        if self.preview.cached_h_scrollbar_area.is_some() {
            inner_height = inner_height.saturating_sub(1);
        }

        // Content area starts after the gutter
        let content_x = inner_x + gutter_width;
        let content_width = area.width.saturating_sub(2 + gutter_width);

        // Check if click is within content area (not in gutter or outside)
        if click_x < content_x || click_x >= content_x + content_width {
            return;
        }
        if click_y < inner_y || click_y >= inner_y + inner_height {
            return;
        }

        // Calculate relative position within content area
        let rel_x = (click_x - content_x) as usize;
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
        let max_row = total_lines.saturating_sub(1) as u16;
        let clamped_row = target_row.min(max_row);

        let line_len = editor
            .lines()
            .get(clamped_row as usize)
            .map(|l| l.chars().count()) // Use char count for UTF-8 safety
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
        let panes_to_restart: Vec<PaneId> = self
            .terminals
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
