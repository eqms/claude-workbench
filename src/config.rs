use serde::{Deserialize, Serialize};
use anyhow::Result;
use std::path::Path;
use std::fs;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Config {
    pub terminal: TerminalConfig,
    pub ui: UiConfig,
    #[serde(default)]
    pub layout: LayoutConfig,
    #[serde(default)]
    pub file_browser: FileBrowserConfig,
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
            show_hidden: false,
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
        }
    }
}

pub fn load_config() -> Result<Config> {
    // 1. Check local config.yaml
    let local_config = Path::new("config.yaml");
    if local_config.exists() {
         let contents = fs::read_to_string(local_config)?;
         let config: Config = serde_yaml::from_str(&contents)?;
         return Ok(config);
    }

    // 2. Check ~/.config/claude-workbench/config.yaml
    if let Some(config_dir) = directories::ProjectDirs::from("com", "antigravity", "claude-workbench") {
        let config_path = config_dir.config_dir().join("config.yaml");
        if config_path.exists() {
             let contents = fs::read_to_string(&config_path)?;
             let config: Config = serde_yaml::from_str(&contents)?;
             
             // Override with defaults if fields missing? (serde handles this if optional, but here we require them or use default logic if we structure it differently)
             // For now assume valid yaml or error.
             return Ok(config);
        }
    }
    
    // Fallback: Check if user provided shell in env?
    // Or just return default
    Ok(Config::default())
}
