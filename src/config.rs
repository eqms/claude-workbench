use serde::{Deserialize, Serialize};
use anyhow::Result;
use std::path::Path;
use std::fs;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Config {
    pub terminal: TerminalConfig,
    pub ui: UiConfig,
    #[serde(default)]
    pub layout: LayoutConfig,
    #[serde(default)]
    pub file_browser: FileBrowserConfig,
    #[serde(default)]
    pub pty: PtyConfig,
    #[serde(default)]
    pub setup: SetupConfig,
    #[serde(default)]
    pub claude: ClaudeConfig,
}

/// PTY configuration for all terminal panes
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct PtyConfig {
    pub claude_command: Vec<String>,
    pub lazygit_command: Vec<String>,
    pub scrollback_lines: usize,
    /// Auto-restart PTY processes when they exit (default: true)
    #[serde(default = "default_true")]
    pub auto_restart: bool,
}

fn default_true() -> bool { true }

impl Default for PtyConfig {
    fn default() -> Self {
        Self {
            // Empty = use terminal shell (Fish/Bash), user starts claude manually
            claude_command: vec![],
            lazygit_command: vec!["lazygit".to_string()],
            scrollback_lines: 1000,
            auto_restart: true,
        }
    }
}

/// Setup/wizard state persisted in config
#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct SetupConfig {
    pub wizard_completed: bool,
    pub wizard_version: u8,
    pub active_template: String,
}

/// Claude startup prefix definition
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ClaudePrefix {
    pub name: String,
    pub prefix: String,
    pub description: String,
}

/// Claude-specific configuration
#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct ClaudeConfig {
    /// Startup prefixes shown in dialog
    #[serde(default)]
    pub startup_prefixes: Vec<ClaudePrefix>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct TerminalConfig {
    pub shell_path: String,
    pub shell_args: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct UiConfig {
    pub theme: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct LayoutConfig {
    pub claude_height_percent: u16,
    pub file_browser_width_percent: u16,
    pub preview_width_percent: u16,
    pub right_panel_width_percent: u16,
}

impl Default for LayoutConfig {
    fn default() -> Self {
        Self {
            claude_height_percent: 40,
            file_browser_width_percent: 20,
            preview_width_percent: 50,
            right_panel_width_percent: 30,
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct FileBrowserConfig {
    pub show_hidden: bool,
    pub show_file_info: bool,
    pub date_format: String,
    pub auto_refresh_ms: u64,  // 0 = disabled
}

impl Default for FileBrowserConfig {
    fn default() -> Self {
        Self {
            show_hidden: true,  // Show hidden files by default (toggle with '.')
            show_file_info: true,
            date_format: "%d.%m.%Y %H:%M:%S".to_string(),
            auto_refresh_ms: 2000, // 2 seconds
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            terminal: TerminalConfig {
                shell_path: "/bin/bash".into(),
                shell_args: vec![],
            },
            ui: UiConfig {
                theme: "default".into(),
            },
            layout: LayoutConfig::default(),
            file_browser: FileBrowserConfig::default(),
            pty: PtyConfig::default(),
            setup: SetupConfig::default(),
            claude: ClaudeConfig::default(),
        }
    }
}

/// Get XDG-style config directory: ~/.config/claude-workbench/
fn get_config_dir() -> Option<std::path::PathBuf> {
    // Use $XDG_CONFIG_HOME if set, otherwise ~/.config
    if let Ok(xdg_config) = std::env::var("XDG_CONFIG_HOME") {
        return Some(std::path::PathBuf::from(xdg_config).join("claude-workbench"));
    }

    // Fallback to ~/.config/claude-workbench
    dirs::home_dir().map(|home| home.join(".config").join("claude-workbench"))
}

pub fn load_config() -> Result<Config> {
    // 1. Check local config.yaml (project-specific override)
    let local_config = Path::new("config.yaml");
    if local_config.exists() {
         let contents = fs::read_to_string(local_config)?;
         let config: Config = serde_yaml::from_str(&contents)?;
         return Ok(config);
    }

    // 2. Check ~/.config/claude-workbench/config.yaml (XDG-style)
    if let Some(config_dir) = get_config_dir() {
        let config_path = config_dir.join("config.yaml");
        if config_path.exists() {
             let contents = fs::read_to_string(&config_path)?;
             let config: Config = serde_yaml::from_str(&contents)?;
             return Ok(config);
        }
    }

    // Fallback to default config
    Ok(Config::default())
}

/// Set restrictive file permissions (0600 - owner read/write only) on Unix systems
#[cfg(unix)]
fn set_restrictive_permissions(path: &Path) -> Result<()> {
    let mut perms = fs::metadata(path)?.permissions();
    perms.set_mode(0o600);
    fs::set_permissions(path, perms)?;
    Ok(())
}

#[cfg(not(unix))]
fn set_restrictive_permissions(_path: &Path) -> Result<()> {
    // No-op on non-Unix systems (Windows handles permissions differently)
    Ok(())
}

/// Save config - updates local config.yaml if it exists, otherwise XDG config
pub fn save_config(config: &Config) -> Result<()> {
    // If local config.yaml exists, update it (maintains project-specific settings)
    let local_config = Path::new("config.yaml");
    if local_config.exists() {
        let yaml = serde_yaml::to_string(config)?;
        fs::write(local_config, &yaml)?;
        set_restrictive_permissions(local_config)?;
        return Ok(());
    }

    // Otherwise save to XDG config directory
    if let Some(config_dir) = get_config_dir() {
        let config_path = config_dir.join("config.yaml");
        fs::create_dir_all(&config_dir)?;
        let yaml = serde_yaml::to_string(config)?;
        fs::write(&config_path, &yaml)?;
        set_restrictive_permissions(&config_path)?;
    }
    Ok(())
}

/// Get the config file path (for display purposes)
pub fn get_config_path() -> Option<std::path::PathBuf> {
    get_config_dir().map(|d| d.join("config.yaml"))
}
