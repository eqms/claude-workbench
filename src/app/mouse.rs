use crossterm::event::{MouseEvent, MouseEventKind};
use ratatui::layout::Rect;

use crate::browser;
use crate::types::{ClaudePermissionMode, EditorMode, PaneId, ResizeBorder, ScrollbarAxis};
use crate::ui;

use super::App;

/// Free helper for hit-testing: returns true when (x, y) falls inside rect r.
fn is_inside(r: Rect, x: u16, y: u16) -> bool {
    x >= r.x && x < r.x + r.width && y >= r.y && y < r.y + r.height
}

impl App {
    /// Handle scrollbar drag: convert mouse Y position to scroll position for a pane
    pub(super) fn handle_scrollbar_position(&mut self, pane: PaneId, y: u16, sb: Rect) {
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
    pub(super) fn handle_horizontal_scrollbar_position(&mut self, x: u16, hsb: Rect) {
        let clamped = x.clamp(hsb.x, hsb.x + hsb.width.saturating_sub(1));
        let ratio = (clamped - hsb.x) as f64 / hsb.width.max(1) as f64;
        let max_width = if self.preview.mode == EditorMode::Edit {
            self.preview.edit_max_display_width() as usize
        } else {
            self.preview.max_line_width() as usize
        };
        self.preview.horizontal_scroll = (ratio * max_width as f64) as u16;
    }

    /// Position preview editor cursor based on mouse click coordinates
    pub(super) fn position_preview_cursor(&mut self, area: Rect, click_x: u16, click_y: u16) {
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

    /// Handle all mouse events. Layout rects are pre-computed by the caller.
    pub(super) fn handle_mouse_event(&mut self, mouse: MouseEvent, rects: super::LayoutRects) {
        let files = rects.files;
        let preview = rects.preview;
        let claude = rects.claude;
        let lazygit = rects.lazygit;
        let term = rects.terminal;
        let footer_area = rects.footer;
        let x = mouse.column;
        let y = mouse.row;

        match mouse.kind {
            MouseEventKind::Down(crossterm::event::MouseButton::Left) => {
                // Block all background interaction when any modal is open
                // Update dialog - handle before other modals
                if self.update_state.show_dialog {
                    use crate::ui::update_dialog;
                    if update_dialog::is_inside_popup(&self.update_dialog_areas, x, y) {
                        if let Some(button) =
                            update_dialog::check_button_click(&self.update_dialog_areas, x, y)
                        {
                            match button {
                                crate::ui::update_dialog::UpdateDialogButton::Update => {
                                    if !self.update_state.updating {
                                        self.start_update();
                                    }
                                }
                                crate::ui::update_dialog::UpdateDialogButton::Later
                                | crate::ui::update_dialog::UpdateDialogButton::Close => {
                                    self.update_state.close_dialog();
                                }
                                crate::ui::update_dialog::UpdateDialogButton::Restart => {
                                    // Signal restart and exit cleanly
                                    self.should_restart = true;
                                    self.should_quit = true;
                                }
                            }
                        }
                    } else {
                        self.update_state.close_dialog();
                    }
                    return;
                }

                // About dialog - click outside to close
                if self.about.visible {
                    if let Some(popup) = self.about.popup_area {
                        if !is_inside(popup, x, y) {
                            self.about.close();
                        }
                    }
                    return;
                }

                // Help popup - click inside to interact, outside to close
                if self.help.visible {
                    if !self.help.contains(x, y) {
                        self.help.close();
                    }
                    return;
                }

                // Settings menu - click outside closes it
                if self.settings.visible {
                    self.settings.close();
                    return;
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
                    return;
                }

                // Fuzzy finder - click outside closes it
                if self.fuzzy_finder.visible {
                    self.fuzzy_finder.close();
                    return;
                }

                // Permission mode dialog - click outside uses default mode
                // Skip if update dialog is visible - update takes priority
                if self.permission_mode_dialog.visible && !self.update_state.show_dialog {
                    let mode = ClaudePermissionMode::Default;
                    self.permission_mode_dialog.close();
                    if self.claude_pty_pending {
                        self.init_claude_pty(mode);
                    }
                    self.active_pane = PaneId::Claude;
                    return;
                }

                // Claude startup dialog - click outside closes it
                // Do NOT set active_pane or return - let click fall through
                // to normal pane hit-testing logic
                if self.claude_startup.visible {
                    self.claude_startup.close();
                }

                // Setup wizard - block all background clicks
                if self.wizard.visible {
                    return;
                }

                // Menu popup - click outside closes it
                if self.menu.visible {
                    self.menu.visible = false;
                    return;
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
                                self.scrollbar_drag.axis = ScrollbarAxis::Horizontal;
                                self.handle_horizontal_scrollbar_position(x, hsb);
                                hit_scrollbar = true;
                            }
                        }
                    }
                    if hit_scrollbar {
                        return;
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
                        if (x as i16 - border_x as i16).abs() <= 1 && y > 0 && y < top_limit {
                            self.resize_state.dragging = true;
                            self.resize_state.border = Some(ResizeBorder::FilePreview);
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
                            if (x as i16 - border_x as i16).abs() <= 1 && y > 0 && y < top_limit {
                                self.resize_state.dragging = true;
                                self.resize_state.border = Some(ResizeBorder::PreviewRight);
                                hit_border = true;
                            }
                        }
                    }
                    // Check horizontal border: Top Area | Claude
                    if !hit_border {
                        if let Some(border_y) = self.border_areas.top_claude_y {
                            if (y as i16 - border_y as i16).abs() <= 1 {
                                self.resize_state.dragging = true;
                                self.resize_state.border = Some(ResizeBorder::TopClaude);
                                hit_border = true;
                            }
                        }
                    }
                    if hit_border {
                        return;
                    }
                }

                if is_inside(files, x, y) {
                    self.active_pane = PaneId::FileBrowser;
                    // File browser layout: [list with borders] + [info bar (1 line)]
                    // List content area: after top border, before bottom border and info bar
                    let list_content_top = files.y + 1; // After top border
                    let list_content_bottom = files.y + files.height.saturating_sub(3); // Before bottom border (1) + info bar (1)

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
                                && now.duration_since(self.last_click_time).as_millis() < 300;

                            // Update tracking for next click
                            self.last_click_time = now;
                            self.last_click_idx = Some(idx);

                            // Check if editor has unsaved changes before switching
                            let has_unsaved =
                                self.preview.mode == EditorMode::Edit && self.preview.is_modified();

                            if is_double_click {
                                // Double-click: enter directory or open file
                                let is_dir = self
                                    .file_browser
                                    .entries
                                    .get(idx)
                                    .map(|e| e.is_dir)
                                    .unwrap_or(false);
                                if is_dir {
                                    if has_unsaved && self.config.ui.autosave {
                                        // Autosave: save, exit edit mode, then enter directory
                                        let _ = self.preview.save();
                                        self.last_autosave_time = Some(std::time::Instant::now());
                                        self.preview.exit_edit_mode(false);
                                        self.preview.refresh_highlighting(&self.syntax_manager);
                                        self.file_browser.list_state.select(Some(idx));
                                        self.file_browser.enter_selected();
                                        self.update_preview();
                                        self.sync_terminals();
                                        self.check_repo_change();
                                    } else if has_unsaved {
                                        self.dialog.dialog_type = ui::dialog::DialogType::Confirm {
                                            title: "Unsaved Changes".to_string(),
                                            message: "Discard changes and enter directory?"
                                                .to_string(),
                                            action: ui::dialog::DialogAction::EnterDirectory {
                                                target_idx: idx,
                                            },
                                        };
                                    } else {
                                        self.file_browser.list_state.select(Some(idx));
                                        self.file_browser.enter_selected();
                                        self.update_preview();
                                        self.sync_terminals();
                                        self.check_repo_change();
                                    }
                                }
                            } else {
                                // Single click: just select (but check for unsaved changes)
                                if has_unsaved && self.config.ui.autosave {
                                    // Autosave: save, exit edit mode, then switch file
                                    let _ = self.preview.save();
                                    self.last_autosave_time = Some(std::time::Instant::now());
                                    self.preview.exit_edit_mode(false);
                                    self.preview.refresh_highlighting(&self.syntax_manager);
                                    self.file_browser.list_state.select(Some(idx));
                                    self.update_preview();
                                } else if has_unsaved {
                                    self.dialog.dialog_type = ui::dialog::DialogType::Confirm {
                                        title: "Unsaved Changes".to_string(),
                                        message: "Discard changes and switch file?".to_string(),
                                        action: ui::dialog::DialogAction::SwitchFile {
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
                    }
                    self.active_pane = PaneId::Claude;
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
                        self.config.ui.autosave,
                    );

                    for (start, end, action) in positions {
                        if footer_x >= start && footer_x < end {
                            use ui::footer::FooterAction;
                            match action {
                                FooterAction::ToggleFiles => {
                                    self.show_file_browser = !self.show_file_browser;
                                    self.config.ui.show_file_browser = self.show_file_browser;
                                    let _ = crate::config::save_config(&self.config);
                                    if self.show_file_browser {
                                        self.active_pane = PaneId::FileBrowser;
                                    } else if self.active_pane == PaneId::FileBrowser {
                                        self.active_pane = PaneId::Claude;
                                    }
                                }
                                FooterAction::TogglePreview => {
                                    self.show_preview = !self.show_preview;
                                    self.config.ui.show_preview = self.show_preview;
                                    let _ = crate::config::save_config(&self.config);
                                    if self.show_preview {
                                        self.active_pane = PaneId::Preview;
                                    } else if self.active_pane == PaneId::Preview {
                                        self.active_pane = PaneId::FileBrowser;
                                    }
                                }
                                FooterAction::MaximizePreview => {
                                    self.toggle_preview_maximize();
                                }
                                FooterAction::FocusClaude => {
                                    if !self.claude_startup.shown_this_session
                                        && !self.config.claude.startup_prefixes.is_empty()
                                    {
                                        self.claude_startup
                                            .open(self.config.claude.startup_prefixes.clone());
                                    } else {
                                        self.active_pane = PaneId::Claude;
                                    }
                                }
                                FooterAction::ToggleGit => {
                                    self.show_lazygit = !self.show_lazygit;
                                    self.config.ui.show_lazygit = self.show_lazygit;
                                    let _ = crate::config::save_config(&self.config);
                                    if self.show_lazygit {
                                        self.active_pane = PaneId::LazyGit;
                                    }
                                }
                                FooterAction::ToggleTerm => {
                                    self.show_terminal = !self.show_terminal;
                                    self.config.ui.show_terminal = self.show_terminal;
                                    let _ = crate::config::save_config(&self.config);
                                    if self.show_terminal {
                                        self.active_pane = PaneId::Terminal;
                                    }
                                }
                                FooterAction::FuzzyFind => {
                                    self.fuzzy_finder.open(&self.file_browser.current_dir);
                                }
                                FooterAction::OpenFile => {
                                    if let Some(path) = self.file_browser.selected_file() {
                                        if browser::can_preview_in_browser(&path) {
                                            let preview_path = if browser::is_markdown(&path) {
                                                match browser::markdown_to_html(&path) {
                                                    Ok(p) => {
                                                        self.temp_preview_files.push(p.clone());
                                                        p
                                                    }
                                                    Err(_) => path,
                                                }
                                            } else {
                                                path
                                            };
                                            let _ = browser::open_file(&preview_path);
                                        }
                                    }
                                }
                                FooterAction::OpenFinder => {
                                    let _ = browser::open_in_file_manager(
                                        &self.file_browser.current_dir,
                                    );
                                }
                                FooterAction::ToggleHidden => {
                                    self.file_browser.show_hidden = !self.file_browser.show_hidden;
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
                                        self.preview.mode = crate::types::EditorMode::Edit;
                                    }
                                }
                                FooterAction::StartSelect => {
                                    // Start selection mode in current pane
                                    if self.active_pane == PaneId::Preview {
                                        self.terminal_selection
                                            .start(self.preview.scroll as usize, PaneId::Preview);
                                    } else if matches!(
                                        self.active_pane,
                                        PaneId::Claude | PaneId::LazyGit | PaneId::Terminal
                                    ) {
                                        self.terminal_selection.start(0, self.active_pane);
                                    }
                                }
                                FooterAction::Save => {
                                    // Save in edit mode
                                    if self.active_pane == PaneId::Preview
                                        && self.preview.mode == crate::types::EditorMode::Edit
                                    {
                                        let _ = self.preview.save();
                                    }
                                }
                                FooterAction::ExitEdit => {
                                    // Exit edit mode
                                    if self.active_pane == PaneId::Preview {
                                        self.preview.mode = crate::types::EditorMode::ReadOnly;
                                    }
                                }
                                FooterAction::Undo => {
                                    if self.active_pane == PaneId::Preview
                                        && self.preview.mode == crate::types::EditorMode::Edit
                                    {
                                        if let Some(editor) = &mut self.preview.editor {
                                            editor.undo();
                                            self.preview.update_modified();
                                            self.preview
                                                .update_edit_highlighting(&self.syntax_manager);
                                        }
                                    }
                                }
                                FooterAction::Redo => {
                                    // Redo handled by keyboard only
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
                                FooterAction::ToggleBlock => {
                                    // MC Edit: Toggle block marking (F3)
                                    if self.active_pane == PaneId::Preview
                                        && self.preview.mode == crate::types::EditorMode::Edit
                                    {
                                        self.preview.toggle_block_marking();
                                    }
                                }
                                FooterAction::CopyBlock => {
                                    // MC Edit: Copy block (F5)
                                    if self.active_pane == PaneId::Preview
                                        && self.preview.mode == crate::types::EditorMode::Edit
                                    {
                                        self.preview.copy_block();
                                    }
                                }
                                FooterAction::MoveBlock => {
                                    // MC Edit: Move/cut block (F6)
                                    if self.active_pane == PaneId::Preview
                                        && self.preview.mode == crate::types::EditorMode::Edit
                                    {
                                        self.preview.move_block();
                                        self.preview.update_modified();
                                        self.preview.update_edit_highlighting(&self.syntax_manager);
                                    }
                                }
                                FooterAction::DeleteBlock => {
                                    // MC Edit: Delete block (F8)
                                    if self.active_pane == PaneId::Preview
                                        && self.preview.mode == crate::types::EditorMode::Edit
                                    {
                                        self.preview.delete_block();
                                        self.preview.update_modified();
                                        self.preview.update_edit_highlighting(&self.syntax_manager);
                                    }
                                }
                                FooterAction::PlatformPaste => {
                                    // Platform paste (Ctrl+V)
                                    if self.active_pane == PaneId::Preview
                                        && self.preview.mode == crate::types::EditorMode::Edit
                                    {
                                        self.preview.paste_from_clipboard();
                                        self.preview.update_modified();
                                        self.preview.update_edit_highlighting(&self.syntax_manager);
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
                                        && self.preview.mode == crate::types::EditorMode::Edit
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
                                FooterAction::CopyLastLines => {
                                    if matches!(
                                        self.active_pane,
                                        PaneId::Claude | PaneId::LazyGit | PaneId::Terminal
                                    ) {
                                        self.copy_last_lines_to_clipboard();
                                    }
                                }
                                FooterAction::ToggleAutosave => {
                                    self.config.ui.autosave = !self.config.ui.autosave;
                                    let _ = crate::config::save_config(&self.config);
                                }
                                FooterAction::None => {}
                            }
                            break;
                        }
                    }
                }
            }
            // Handle drag movement
            MouseEventKind::Drag(crossterm::event::MouseButton::Left) => {
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
                    return;
                }
                // Handle scrollbar drag
                if self.scrollbar_drag.dragging {
                    match self.scrollbar_drag.axis {
                        ScrollbarAxis::Horizontal => {
                            if let Some(hsb) = self.scrollbar_areas.preview_horizontal {
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
                    return;
                }
                // Handle pane border resize drag
                if self.resize_state.dragging {
                    match self.resize_state.border {
                        Some(ResizeBorder::FilePreview) => {
                            if self.border_areas.total_width > 0 {
                                let new_pct = ((x as f64 / self.border_areas.total_width as f64)
                                    * 100.0) as u16;
                                self.config.layout.file_browser_width_percent =
                                    new_pct.clamp(10, 50);
                            }
                        }
                        Some(ResizeBorder::PreviewRight) => {
                            if self.border_areas.total_width > 0 {
                                let file_pct = self.config.layout.file_browser_width_percent;
                                let new_preview_pct =
                                    ((x as f64 / self.border_areas.total_width as f64) * 100.0)
                                        as u16;
                                let preview_pct =
                                    new_preview_pct.saturating_sub(file_pct).clamp(15, 70);
                                self.config.layout.preview_width_percent = preview_pct;
                                self.config.layout.right_panel_width_percent = 100u16
                                    .saturating_sub(file_pct)
                                    .saturating_sub(preview_pct)
                                    .clamp(10, 60);
                            }
                        }
                        Some(ResizeBorder::TopClaude) => {
                            // Footer is 1 line at bottom
                            let usable_height = self.border_areas.total_height.saturating_sub(1);
                            if usable_height > 0 {
                                let claude_start_pct =
                                    ((y as f64 / usable_height as f64) * 100.0) as u16;
                                self.config.layout.claude_height_percent =
                                    100u16.saturating_sub(claude_start_pct).clamp(20, 80);
                            }
                        }
                        None => {}
                    }
                    return;
                }
                // Handle character-level mouse text selection in terminal panes
                if self.mouse_selection.selecting {
                    self.mouse_selection.update(x, y);
                } else if self.drag_state.dragging {
                    self.drag_state.update_position(x, y);
                }
            }
            // Handle drag drop and mouse selection finish
            MouseEventKind::Up(crossterm::event::MouseButton::Left) => {
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
                    return;
                }
                // Handle pane resize drag finish - save config
                if self.resize_state.dragging {
                    self.resize_state.dragging = false;
                    self.resize_state.border = None;
                    let _ = crate::config::save_config(&self.config);
                    return;
                }
                // Handle scrollbar drag finish
                if self.scrollbar_drag.dragging {
                    self.scrollbar_drag.dragging = false;
                    self.scrollbar_drag.pane = None;
                    self.scrollbar_drag.axis = ScrollbarAxis::default();
                    return;
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
            MouseEventKind::ScrollDown => {
                // Block all background scroll when any modal is open
                if self.about.visible {
                    return;
                }

                // Help popup scroll handling
                if self.help.visible {
                    if self.help.contains(x, y) {
                        self.help.scroll_down(1);
                    }
                    return;
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
                    return;
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
            MouseEventKind::ScrollUp => {
                // Block all background scroll when any modal is open
                if self.about.visible {
                    return;
                }

                // Help popup scroll handling
                if self.help.visible {
                    if self.help.contains(x, y) {
                        self.help.scroll_up(1);
                    }
                    return;
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
                    return;
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
}
