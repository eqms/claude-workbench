//! Auto-detection of installed browsers and editors on macOS and Linux.

#[cfg(not(target_os = "windows"))]
use std::path::PathBuf;
use std::process::Command;

/// A detected application with display name and launch command
#[derive(Debug, Clone)]
pub struct DetectedApp {
    /// Human-readable name, e.g. "Firefox", "VS Code"
    pub display_name: String,
    /// Command string for launching, e.g. "open -a Firefox" (macOS) or "firefox" (Linux)
    pub command: String,
}

/// Application definition for detection
struct AppDef {
    display_name: &'static str,
    /// macOS: app bundle name(s) to check in /Applications/
    #[cfg(target_os = "macos")]
    bundle_names: &'static [&'static str],
    /// Linux: binary name(s) to check via `which`
    #[cfg(target_os = "linux")]
    binary_names: &'static [&'static str],
    /// Also check via `which` on macOS (for CLI tools like nvim)
    #[cfg(target_os = "macos")]
    cli_names: &'static [&'static str],
}

// ── macOS ──────────────────────────────────────────────────────────────

#[cfg(target_os = "macos")]
const BROWSERS_MACOS: &[AppDef] = &[
    AppDef {
        display_name: "Safari",
        bundle_names: &["Safari"],
        cli_names: &[],
    },
    AppDef {
        display_name: "Firefox",
        bundle_names: &["Firefox"],
        cli_names: &[],
    },
    AppDef {
        display_name: "Google Chrome",
        bundle_names: &["Google Chrome"],
        cli_names: &[],
    },
    AppDef {
        display_name: "Chromium",
        bundle_names: &["Chromium"],
        cli_names: &[],
    },
    AppDef {
        display_name: "Brave Browser",
        bundle_names: &["Brave Browser"],
        cli_names: &[],
    },
    AppDef {
        display_name: "Microsoft Edge",
        bundle_names: &["Microsoft Edge"],
        cli_names: &[],
    },
    AppDef {
        display_name: "Opera",
        bundle_names: &["Opera"],
        cli_names: &[],
    },
    AppDef {
        display_name: "Vivaldi",
        bundle_names: &["Vivaldi"],
        cli_names: &[],
    },
    AppDef {
        display_name: "Arc",
        bundle_names: &["Arc"],
        cli_names: &[],
    },
    AppDef {
        display_name: "Zen Browser",
        bundle_names: &["Zen Browser"],
        cli_names: &[],
    },
];

#[cfg(target_os = "macos")]
const EDITORS_MACOS: &[AppDef] = &[
    AppDef {
        display_name: "Visual Studio Code",
        bundle_names: &["Visual Studio Code", "Visual Studio Code - Insiders"],
        cli_names: &[],
    },
    AppDef {
        display_name: "VSCodium",
        bundle_names: &["VSCodium"],
        cli_names: &[],
    },
    AppDef {
        display_name: "Cursor",
        bundle_names: &["Cursor"],
        cli_names: &[],
    },
    AppDef {
        display_name: "Zed",
        bundle_names: &["Zed"],
        cli_names: &[],
    },
    AppDef {
        display_name: "Sublime Text",
        bundle_names: &["Sublime Text"],
        cli_names: &[],
    },
    AppDef {
        display_name: "Nova",
        bundle_names: &["Nova"],
        cli_names: &[],
    },
    AppDef {
        display_name: "BBEdit",
        bundle_names: &["BBEdit"],
        cli_names: &[],
    },
    AppDef {
        display_name: "TextMate",
        bundle_names: &["TextMate"],
        cli_names: &[],
    },
    AppDef {
        display_name: "Neovim",
        bundle_names: &[],
        cli_names: &["nvim"],
    },
    AppDef {
        display_name: "Vim",
        bundle_names: &[],
        cli_names: &["vim"],
    },
    AppDef {
        display_name: "nano",
        bundle_names: &[],
        cli_names: &["nano"],
    },
];

// ── Linux ──────────────────────────────────────────────────────────────

#[cfg(target_os = "linux")]
const BROWSERS_LINUX: &[AppDef] = &[
    AppDef {
        display_name: "Firefox",
        binary_names: &["firefox"],
    },
    AppDef {
        display_name: "Google Chrome",
        binary_names: &["google-chrome", "google-chrome-stable"],
    },
    AppDef {
        display_name: "Chromium",
        binary_names: &["chromium", "chromium-browser"],
    },
    AppDef {
        display_name: "Brave Browser",
        binary_names: &["brave-browser", "brave-browser-stable"],
    },
    AppDef {
        display_name: "Microsoft Edge",
        binary_names: &["microsoft-edge", "microsoft-edge-stable"],
    },
    AppDef {
        display_name: "Opera",
        binary_names: &["opera"],
    },
    AppDef {
        display_name: "Vivaldi",
        binary_names: &["vivaldi", "vivaldi-stable"],
    },
    AppDef {
        display_name: "Zen Browser",
        binary_names: &["zen-browser"],
    },
];

#[cfg(target_os = "linux")]
const EDITORS_LINUX: &[AppDef] = &[
    AppDef {
        display_name: "VS Code",
        binary_names: &["code"],
    },
    AppDef {
        display_name: "VSCodium",
        binary_names: &["codium"],
    },
    AppDef {
        display_name: "Cursor",
        binary_names: &["cursor"],
    },
    AppDef {
        display_name: "Zed",
        binary_names: &["zed"],
    },
    AppDef {
        display_name: "Sublime Text",
        binary_names: &["subl", "sublime_text"],
    },
    AppDef {
        display_name: "Kate",
        binary_names: &["kate"],
    },
    AppDef {
        display_name: "Gedit",
        binary_names: &["gedit"],
    },
    AppDef {
        display_name: "Neovim",
        binary_names: &["nvim"],
    },
    AppDef {
        display_name: "Vim",
        binary_names: &["vim"],
    },
    AppDef {
        display_name: "nano",
        binary_names: &["nano"],
    },
];

// ── Detection Logic ────────────────────────────────────────────────────

/// Detect installed browsers on the current platform
pub fn detect_browsers() -> Vec<DetectedApp> {
    #[cfg(target_os = "macos")]
    {
        detect_macos_apps(BROWSERS_MACOS)
    }
    #[cfg(target_os = "linux")]
    {
        detect_linux_apps(BROWSERS_LINUX)
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        Vec::new()
    }
}

/// Detect installed editors on the current platform
pub fn detect_editors() -> Vec<DetectedApp> {
    #[cfg(target_os = "macos")]
    {
        detect_macos_apps(EDITORS_MACOS)
    }
    #[cfg(target_os = "linux")]
    {
        detect_linux_apps(EDITORS_LINUX)
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        Vec::new()
    }
}

// ── macOS helpers ──────────────────────────────────────────────────────

#[cfg(target_os = "macos")]
fn detect_macos_apps(defs: &[AppDef]) -> Vec<DetectedApp> {
    let home = dirs::home_dir();
    let mut found = Vec::new();

    for def in defs {
        // Check app bundles in /Applications/ and ~/Applications/
        for bundle_name in def.bundle_names {
            let system_path = PathBuf::from(format!("/Applications/{}.app", bundle_name));
            let user_path = home
                .as_ref()
                .map(|h| h.join(format!("Applications/{}.app", bundle_name)));

            if system_path.exists() || user_path.as_ref().is_some_and(|p| p.exists()) {
                let command = if bundle_name.contains(' ') {
                    format!("open -a \"{}\"", bundle_name)
                } else {
                    format!("open -a {}", bundle_name)
                };
                found.push(DetectedApp {
                    display_name: def.display_name.to_string(),
                    command,
                });
                break; // Found this app, skip alternate bundle names
            }
        }

        // Check CLI tools via `which` (for terminal editors like nvim, vim, nano)
        if found
            .last()
            .is_none_or(|a| a.display_name != def.display_name)
        {
            for cli_name in def.cli_names {
                if which_exists(cli_name) {
                    found.push(DetectedApp {
                        display_name: format!("{} (terminal)", def.display_name),
                        command: cli_name.to_string(),
                    });
                    break;
                }
            }
        }
    }

    found
}

// ── Linux helpers ──────────────────────────────────────────────────────

#[cfg(target_os = "linux")]
fn detect_linux_apps(defs: &[AppDef]) -> Vec<DetectedApp> {
    let mut found = Vec::new();

    for def in defs {
        for binary_name in def.binary_names {
            if which_exists(binary_name) {
                found.push(DetectedApp {
                    display_name: def.display_name.to_string(),
                    command: binary_name.to_string(),
                });
                break; // Found this app, skip alternate binary names
            }
        }
    }

    found
}

// ── Shared helpers ─────────────────────────────────────────────────────

/// Check if a binary exists via `which`
fn which_exists(name: &str) -> bool {
    Command::new("which")
        .arg(name)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_browsers_returns_vec() {
        let browsers = detect_browsers();
        // Should return a non-panicking result on any platform
        assert!(browsers.len() <= 20);
    }

    #[test]
    fn test_detect_editors_returns_vec() {
        let editors = detect_editors();
        assert!(editors.len() <= 20);
    }

    #[test]
    fn test_which_exists() {
        // `ls` should exist on all Unix systems
        assert!(which_exists("ls"));
        // Random nonexistent binary
        assert!(!which_exists("totally_nonexistent_binary_xyz123"));
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn test_macos_safari_detected() {
        let browsers = detect_browsers();
        // Safari is always present on macOS
        assert!(
            browsers.iter().any(|b| b.display_name == "Safari"),
            "Safari should be detected on macOS"
        );
    }
}
