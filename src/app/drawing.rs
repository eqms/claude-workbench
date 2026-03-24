use std::collections::HashMap;

use ratatui::{layout::Rect, Frame};

use crate::terminal::PseudoTerminal;
use crate::types::PaneId;
use crate::ui;

use super::App;

impl App {
    pub(super) fn cleanup_temp_files(&self) {
        for path in &self.temp_preview_files {
            let _ = std::fs::remove_file(path);
        }
    }

    pub(super) fn draw(&mut self, frame: &mut Frame) {
        let area = frame.area();
        let (files, preview, claude, lazygit, terminal, footer) = ui::layout::compute_layout(
            area,
            self.show_file_browser,
            self.show_terminal,
            self.show_lazygit,
            self.show_preview,
            self.preview_maximized,
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
                self.config.ui.autosave,
            );
        }

        ui::terminal_pane::render(frame, claude, PaneId::Claude, self);
        ui::terminal_pane::render(frame, lazygit, PaneId::LazyGit, self);
        ui::terminal_pane::render(frame, terminal, PaneId::Terminal, self);

        // Compute autosave flash state (2s duration after last autosave)
        let autosave_flash = self
            .last_autosave_time
            .map(|t| t.elapsed().as_secs() < 2)
            .unwrap_or(false);

        // Compute copy flash state (2s duration after last F9 copy)
        let copy_flash = self
            .last_copy_time
            .map(|t| t.elapsed().as_secs() < 2)
            .unwrap_or(false);
        let copy_flash_lines = self.copy_flash_lines;

        if footer.height > 0 {
            let footer_widget = ui::footer::Footer {
                active_pane: self.active_pane,
                editor_mode: self.preview.mode,
                editor_modified: self.preview.modified,
                selection_mode: self.terminal_selection.active,
                autosave: self.config.ui.autosave,
                autosave_flash,
                copy_flash,
                copy_flash_lines,
                preview_maximized: self.preview_maximized,
            };
            frame.render_widget(footer_widget, footer);
        }

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
}
