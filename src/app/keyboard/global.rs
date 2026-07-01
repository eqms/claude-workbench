//! Global keyboard shortcuts that fire regardless of which pane is active —
//! help (F12), about (F10), F7 ~/.claude jump, F9 copy-N-lines / file-menu,
//! F11 universal paste, Ctrl+P/O/X pickers, Ctrl+Alt+E external editor,
//! F8 settings, Ctrl+Shift+W wizard. Returns true when the key was consumed
//! so the caller can stop routing it further.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::types::{EditorMode, PaneId};
use crate::ui;

use super::super::App;

impl App {
    pub(super) fn handle_global_shortcut(&mut self, key: KeyEvent) -> bool {
        // F10/F12 work everywhere
        if key.code == KeyCode::F(12) {
            self.help.open();
            return true;
        }
        if key.code == KeyCode::F(10) {
            self.about.open();
            return true;
        }

        // F7: Toggle between ~/.claude and previous directory
        if key.code == KeyCode::F(7) {
            if let Some(home) = std::env::var_os("HOME") {
                let claude_dir = std::path::PathBuf::from(home).join(".claude");
                if self.file_browser.current_dir.starts_with(&claude_dir)
                    || self.file_browser.root_dir.starts_with(&claude_dir)
                {
                    if let Some(prev) = self.file_browser.previous_dir.take() {
                        self.file_browser.current_dir = prev;
                        self.file_browser.load_directory();
                        self.active_pane = PaneId::FileBrowser;
                    }
                } else if claude_dir.exists() && claude_dir.is_dir() {
                    self.file_browser.previous_dir = Some(self.file_browser.root_dir.clone());
                    self.file_browser.current_dir = claude_dir;
                    self.file_browser.load_directory();
                    self.active_pane = PaneId::FileBrowser;
                }
            }
            return true;
        }

        // '?' for help - only in FileBrowser or Preview (read-only)
        if key.code == KeyCode::Char('?')
            && matches!(self.active_pane, PaneId::FileBrowser | PaneId::Preview)
            && self.preview.mode != EditorMode::Edit
        {
            self.help.open();
            return true;
        }

        // Shift+F9 or Ctrl+F9: Copy last N lines with interactive count input
        if key.code == KeyCode::F(9)
            && (key.modifiers.contains(KeyModifiers::SHIFT)
                || key.modifiers.contains(KeyModifiers::CONTROL))
        {
            if matches!(
                self.active_pane,
                PaneId::Claude | PaneId::LazyGit | PaneId::Terminal
            ) {
                let default_count = self.config.pty.copy_lines_count.to_string();
                let cursor_pos = default_count.chars().count();
                self.dialog.dialog_type = ui::dialog::DialogType::Input {
                    title: "Copy last N lines".to_string(),
                    value: default_count,
                    cursor: cursor_pos,
                    action: ui::dialog::DialogAction::CopyLastLines,
                };
            }
            return true;
        }

        // F9: Copy last N lines (terminal panes) or File Menu (file browser/preview)
        if key.code == KeyCode::F(9)
            && !key.modifiers.contains(KeyModifiers::SHIFT)
            && !key.modifiers.contains(KeyModifiers::CONTROL)
        {
            match self.active_pane {
                // Real shell pane: copy the whole last-command block from the
                // full scrollback (prompt-to-prompt), not just the visible rows.
                PaneId::Terminal => self.copy_last_command_output(),
                // Claude/LazyGit are TUI apps; their scrollback is not populated
                // the same way, so keep the visible last-N behaviour.
                PaneId::Claude | PaneId::LazyGit => self.copy_last_lines_to_clipboard(),
                _ => self.menu.toggle(),
            }
            return true;
        }

        // F11: Universal paste — read system clipboard via fallback chain
        // (arboard → xclip → xsel → wl-paste) and inject into the active
        // pane. Bypasses Kitty's bracketed-paste bridge entirely; this is
        // the workaround for XRDP sessions where Kitty cannot read the
        // system clipboard.
        if key.code == KeyCode::F(11) && key.modifiers.is_empty() {
            self.paste_from_clipboard_to_active_pane();
            return true;
        }

        // Ctrl+P: Open fuzzy finder
        if key.code == KeyCode::Char('p') && key.modifiers.contains(KeyModifiers::CONTROL) {
            self.fuzzy_finder.open(&self.file_browser.current_dir);
            return true;
        }

        // Ctrl+O: Open Markdown file by path (dialog with tab-completion)
        if key.code == KeyCode::Char('o') && key.modifiers.contains(KeyModifiers::CONTROL) {
            let default_path = dirs::home_dir()
                .map(|h| format!("{}/.claude/plans/", h.display()))
                .unwrap_or_default();
            let cursor = default_path.len();
            self.dialog.dialog_type = crate::ui::dialog::DialogType::Input {
                title: "Open Markdown Preview".to_string(),
                value: default_path,
                cursor,
                action: crate::ui::dialog::DialogAction::OpenMarkdownPreview,
            };
            return true;
        }

        // Ctrl+X: Export current Markdown file or batch-export a folder
        if key.code == KeyCode::Char('x') && key.modifiers.contains(KeyModifiers::CONTROL) {
            // Guard: let the edit handler consume Ctrl+X as cut when in Edit mode
            if self.active_pane == PaneId::Preview && self.preview.mode == EditorMode::Edit {
                return false;
            }
            // Priority 1: FileBrowser pane has focus AND selected entry is a directory (not "..")
            if self.active_pane == PaneId::FileBrowser {
                if let Some(entry) = self.file_browser.selected_entry() {
                    if entry.is_dir && entry.name != ".." {
                        self.export_chooser = crate::types::ExportChooserState {
                            visible: true,
                            source_path: entry.path.clone(),
                            selected: 0,
                            is_batch: true,
                        };
                        return true;
                    }
                }
            }
            // Priority 2: Preview has a Markdown file open (existing single-file behavior)
            if let Some(path) = &self.preview.current_file {
                if self.preview.is_markdown {
                    self.export_chooser = crate::types::ExportChooserState {
                        visible: true,
                        source_path: path.clone(),
                        selected: 0,
                        is_batch: false,
                    };
                    return true;
                }
            }
            // Priority 3: no-op
        }

        // Ctrl+Alt+E (Ctrl+Option+E on macOS): Open selected file in external
        // GUI editor. Was plain Ctrl+E until Claude Code reserved that combo
        // for its own use.
        if key.code == KeyCode::Char('e')
            && key
                .modifiers
                .contains(KeyModifiers::CONTROL | KeyModifiers::ALT)
        {
            let path_opt = if self.active_pane == PaneId::Preview {
                self.preview.current_file.clone()
            } else {
                self.file_browser.selected_file()
            };
            if let Some(path) = path_opt {
                let editor = &self.config.ui.external_editor;
                if !editor.is_empty() {
                    let _ = crate::browser::open_file_with_editor(&path, editor);
                }
            }
            return true;
        }

        // F8: Open settings
        if key.code == KeyCode::F(8) {
            self.settings.open(&self.config);
            return true;
        }

        // Ctrl+Shift+W: Re-run setup wizard
        if key.code == KeyCode::Char('W')
            && key
                .modifiers
                .contains(KeyModifiers::CONTROL | KeyModifiers::SHIFT)
        {
            self.wizard.open();
            return true;
        }

        false
    }
}
