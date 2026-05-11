//! Terminal-pane key handling — Claude, LazyGit, and the user terminal.
//! Handles terminal-selection mode (Ctrl+S), Shift+PageUp/Down scrollback,
//! the SSH-image-paste hint, and forwards everything else to the PTY.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::types::PaneId;

use super::super::App;

impl App {
    pub(super) fn handle_terminal_pane_key(&mut self, key: KeyEvent) {
        // Terminal selection mode handling
        if self.terminal_selection.active
            && self.terminal_selection.source_pane == Some(self.active_pane)
        {
            match key.code {
                KeyCode::Up | KeyCode::Char('k') => {
                    if let Some(end) = self.terminal_selection.end_line {
                        self.terminal_selection.extend(end.saturating_sub(1));
                    }
                    return;
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if let Some(end) = self.terminal_selection.end_line {
                        self.terminal_selection.extend(end + 1);
                    }
                    return;
                }
                KeyCode::Enter | KeyCode::Char('y') => {
                    self.copy_selection_to_claude();
                    self.terminal_selection.clear();
                    return;
                }
                KeyCode::Char('c')
                    if key.modifiers.contains(KeyModifiers::CONTROL)
                        || key.modifiers.contains(KeyModifiers::SUPER) =>
                {
                    self.copy_selection_to_clipboard();
                    self.terminal_selection.clear();
                    return;
                }
                KeyCode::Esc => {
                    self.terminal_selection.clear();
                    return;
                }
                _ => {
                    // Let other keys pass through to PTY
                }
            }
        }

        // Ctrl+S: Start terminal selection mode
        if key.code == KeyCode::Char('s') && key.modifiers.contains(KeyModifiers::CONTROL) {
            if let Some(pty) = self.terminals.get(&self.active_pane) {
                let cursor_row = pty.cursor_row() as usize;
                self.terminal_selection.start(cursor_row, self.active_pane);
            }
            return;
        }

        if let Some(pty) = self.terminals.get(&self.active_pane) {
            if pty.has_exited() && !self.config.pty.auto_restart {
                if key.code == KeyCode::Enter {
                    self.restart_single_pty(self.active_pane);
                }
                return;
            }
        }

        // SSH image-paste hint: Ctrl+V in the Claude pane during an SSH
        // session cannot reach the upstream Mac/Windows pasteboard. We
        // flash a one-time hint and persist `notification_dismissed` so
        // the user is not nagged again. The keystroke is *not* consumed —
        // `map_key_to_pty()` below still forwards 0x16 so the Claude CLI's
        // own paste path keeps its current behavior.
        //
        // Cmd+V (SUPER) on macOS is excluded because iTerm2 intercepts it
        // locally and forwards a bracketed-paste sequence rather than 0x16.
        if matches!(self.active_pane, PaneId::Claude)
            && key.code == KeyCode::Char('v')
            && key.modifiers.contains(KeyModifiers::CONTROL)
            && !key.modifiers.contains(KeyModifiers::SUPER)
            && self.config.ssh.enabled
            && !self.config.ssh.notification_dismissed
            && crate::clipboard::is_ssh_session()
        {
            self.show_ssh_image_paste_hint();
        }

        if let Some(pty) = self.terminals.get_mut(&self.active_pane) {
            if key.modifiers.contains(KeyModifiers::SHIFT) {
                match key.code {
                    KeyCode::PageUp => {
                        pty.scroll_up(10);
                        return;
                    }
                    KeyCode::PageDown => {
                        pty.scroll_down(10);
                        return;
                    }
                    KeyCode::Up => {
                        pty.scroll_up(1);
                        return;
                    }
                    KeyCode::Down => {
                        pty.scroll_down(1);
                        return;
                    }
                    _ => {}
                }
            }

            if let Some(bytes) = crate::input::map_key_to_pty(key) {
                let _ = pty.write_input(&bytes);
            }
        }
    }
}
