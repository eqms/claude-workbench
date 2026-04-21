//! Binary installation and restart logic.

use std::sync::mpsc;
use std::thread;

use self_update::backends::github::Update;

use super::check::get_target;
use super::log::log_update;
use super::state::UpdateResult;
use super::version::CURRENT_VERSION;
use super::{BIN_NAME, REPO_NAME, REPO_OWNER};

/// Perform the actual update (blocking)
///
/// This downloads and replaces the current binary.
/// The application should be restarted after this completes.
pub fn perform_update_sync() -> UpdateResult {
    log_update("=== perform_update_sync() STARTED ===");

    let target = get_target();
    let os = std::env::consts::OS;
    let arch = std::env::consts::ARCH;

    let context = format!(
        "Version: {} | Target: {} | Platform: {}-{}",
        CURRENT_VERSION, target, os, arch
    );

    log_update(&format!("Context: {}", context));
    log_update(&format!("Repo: {}/{}", REPO_OWNER, REPO_NAME));
    log_update(&format!("Binary name: {}", BIN_NAME));

    log_update("Creating Update configuration...");

    match Update::configure()
        .repo_owner(REPO_OWNER)
        .repo_name(REPO_NAME)
        .bin_name(BIN_NAME)
        .target(target)
        .current_version(CURRENT_VERSION)
        .show_download_progress(false)
        .show_output(false)
        .no_confirm(true)
        .build()
    {
        Ok(updater) => {
            log_update("Update configuration OK, calling updater.update()...");
            match updater.update() {
                Ok(status) => {
                    log_update(&format!(
                        "UPDATE SUCCESS: {} -> {}",
                        CURRENT_VERSION,
                        status.version()
                    ));
                    UpdateResult::Success {
                        old_version: CURRENT_VERSION.to_string(),
                        new_version: status.version().to_string(),
                    }
                }
                Err(e) => {
                    let error_msg = format!("{}\n\n[{}]", e, context);
                    log_update(&format!("UPDATE FAILED: {}", error_msg));
                    UpdateResult::Error(error_msg)
                }
            }
        }
        Err(e) => {
            let error_msg = format!("Configuration failed: {}\n\n[{}]", e, context);
            log_update(&format!("CONFIG FAILED: {}", error_msg));
            UpdateResult::Error(error_msg)
        }
    }
}

/// Perform update to a specific version (for testing/downgrade)
///
/// This allows updating to any version, including older ones.
/// Useful for testing the update mechanism without releasing new versions.
pub fn perform_update_to_version_sync(target_version: &str) -> UpdateResult {
    log_update(&format!(
        "=== perform_update_to_version_sync({}) STARTED ===",
        target_version
    ));

    let target = get_target();
    let os = std::env::consts::OS;
    let arch = std::env::consts::ARCH;

    let context = format!(
        "Current: {} | Target: {} | Platform target: {} | OS-Arch: {}-{}",
        CURRENT_VERSION, target_version, target, os, arch
    );

    log_update(&format!("Context: {}", context));

    let target_tag = if target_version.starts_with('v') {
        target_version.to_string()
    } else {
        format!("v{}", target_version)
    };

    log_update(&format!("Target tag: {}", target_tag));

    match Update::configure()
        .repo_owner(REPO_OWNER)
        .repo_name(REPO_NAME)
        .bin_name(BIN_NAME)
        .target(target)
        .target_version_tag(&target_tag)
        .current_version(CURRENT_VERSION)
        .show_download_progress(false)
        .show_output(false)
        .no_confirm(true)
        .build()
    {
        Ok(updater) => {
            log_update(&format!(
                "Update configuration OK, updating to {}...",
                target_tag
            ));
            match updater.update() {
                Ok(status) => {
                    log_update(&format!(
                        "UPDATE TO VERSION SUCCESS: {} -> {}",
                        CURRENT_VERSION,
                        status.version()
                    ));
                    UpdateResult::Success {
                        old_version: CURRENT_VERSION.to_string(),
                        new_version: status.version().to_string(),
                    }
                }
                Err(e) => {
                    let error_msg = format!("{}\n\n[{}]", e, context);
                    log_update(&format!("UPDATE TO VERSION FAILED: {}", error_msg));
                    UpdateResult::Error(error_msg)
                }
            }
        }
        Err(e) => {
            let error_msg = format!("Configuration failed: {}\n\n[{}]", e, context);
            log_update(&format!("CONFIG FAILED: {}", error_msg));
            UpdateResult::Error(error_msg)
        }
    }
}

/// Start an async update
///
/// Returns a receiver that will receive the result when the update is complete.
pub fn perform_update_async() -> mpsc::Receiver<UpdateResult> {
    log_update("=== perform_update_async() CALLED ===");
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        log_update("Update thread STARTED");
        let result = perform_update_sync();
        log_update(&format!("Update result: {:?}", result));
        match tx.send(result) {
            Ok(_) => log_update("Result SENT through channel"),
            Err(e) => log_update(&format!("FAILED to send result: {}", e)),
        }
        log_update("Update thread FINISHED");
    });
    log_update("perform_update_async() returning receiver");
    rx
}

/// Restart the application by re-executing the current binary
///
/// This function attempts to restart the application by:
/// 1. Getting the path to the current executable
/// 2. Spawning a new process with the same arguments
/// 3. The caller should exit the current process after this returns Ok
///
/// Returns Ok(()) on successful spawn, or an error message.
pub fn restart_application() -> Result<(), String> {
    log_update("=== restart_application() CALLED ===");

    let exe =
        std::env::current_exe().map_err(|e| format!("Failed to get executable path: {}", e))?;

    // On Linux, /proc/self/exe appends " (deleted)" when the running binary's
    // inode was replaced during self-update (atomic rename). The new binary
    // exists at the original path without this suffix.
    #[cfg(target_os = "linux")]
    let exe = {
        let exe_str = exe.to_string_lossy();
        if exe_str.ends_with(" (deleted)") {
            let clean_path = exe_str.trim_end_matches(" (deleted)");
            log_update(&format!(
                "Linux: stripped ' (deleted)' suffix: {:?} -> {:?}",
                exe_str, clean_path
            ));
            std::path::PathBuf::from(clean_path.to_string())
        } else {
            exe
        }
    };

    log_update(&format!("Executable path: {:?}", exe));

    if !exe.exists() {
        let msg = format!(
            "Binary not found at {:?} after update. Please restart manually.",
            exe
        );
        log_update(&msg);
        return Err(msg);
    }

    let args: Vec<String> = std::env::args().skip(1).collect();
    log_update(&format!("Arguments: {:?}", args));

    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;

        log_update("Using Unix exec() for seamless restart");

        let mut cmd = std::process::Command::new(&exe);
        cmd.args(&args);

        let error = cmd.exec();
        let msg = format!("exec() failed: {}", error);
        log_update(&msg);
        Err(msg)
    }

    #[cfg(not(unix))]
    {
        log_update("Using spawn() for Windows restart");

        std::process::Command::new(&exe)
            .args(&args)
            .spawn()
            .map_err(|e| format!("Failed to spawn new process: {}", e))?;

        log_update("New process spawned successfully");
        Ok(())
    }
}
