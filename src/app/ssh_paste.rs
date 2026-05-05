//! SSH-Image-Paste hint glue.
//!
//! Image paste in the Claude pane (`Ctrl+V`) cannot transparently reach the
//! upstream Mac/Windows pasteboard over an SSH PTY — the byte `0x16` flows
//! to the Claude CLI on the remote host, which then tries to read its own
//! local clipboard and finds nothing. This module surfaces a one-time
//! footer hint pointing the user at `cc-clip`
//! (https://github.com/ShunmeiCho/cc-clip) and persists the dismissed flag
//! so we do not nag on every keypress.
//!
//! The keystroke itself is *not* consumed — callers still pass `0x16` to
//! the PTY so the Claude CLI keeps its existing behavior (no regression
//! for users who later configure `cc-clip` and have the Claude side
//! cooperate).

use super::App;

impl App {
    /// Trigger the one-shot SSH image paste hint and persist the dismissed
    /// flag so the user is not reminded again.
    ///
    /// Called from the keyboard handler when all of these are true:
    /// - active pane is `PaneId::Claude`
    /// - keystroke is `Ctrl+V` (not `Cmd+V`)
    /// - `clipboard::is_ssh_session()` reports true
    /// - `config.ssh.enabled` and not yet `notification_dismissed`
    pub(super) fn show_ssh_image_paste_hint(&mut self) {
        self.ssh_image_paste_hint = Some((
            "Image paste over SSH needs cc-clip — see Settings → SSH (or run --ssh-paste-diag)."
                .to_string(),
            std::time::Instant::now(),
        ));
        self.config.ssh.notification_dismissed = true;
        // Best-effort persist — if the save fails (read-only FS, perms),
        // the hint will fire again next session, which is acceptable.
        let _ = crate::config::save_config(&self.config);
    }
}
