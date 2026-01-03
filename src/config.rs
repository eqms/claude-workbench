use serde::{Deserialize, Serialize};
use anyhow::Result;
use std::path::Path;
use std::fs;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Config {
    pub terminal: TerminalConfig,
    pub ui: UiConfig,
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

impl Default for Config {
    fn default() -> Self {
        Self {
            terminal: TerminalConfig {
                shell_path: "/bin/bash".into(), // Default fallback
                shell_args: vec![],
            },
            ui: UiConfig {
                theme: "default".into(),
            },
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
