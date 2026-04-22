use crate::ui::update_dialog::UpdateDialogButton;
use crate::update::{self, UpdateCheckResult, UpdateResult, UpdateState};

use super::App;

impl App {
    pub(super) fn start_update_check(&mut self) {
        self.update_state.start_check();
        self.update_check_receiver = Some(update::check_for_update_async_with_version(
            self.fake_version.clone(),
        ));
    }

    /// Poll for async update check results
    pub(super) fn poll_update_check(&mut self) {
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
    pub(super) fn poll_update_result(&mut self) {
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
    pub(super) fn start_update(&mut self) {
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

    /// Trigger manual update check from settings menu
    pub fn trigger_update_check(&mut self) {
        self.update_state = UpdateState::new();
        self.update_state.show_dialog = true;
        self.update_state.manual_check = true; // Show errors for manual checks
        self.start_update_check();
    }
}
