use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

use crate::types::ClaudePermissionMode;

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
    #[serde(default)]
    pub document: DocumentConfig,
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
    /// Number of lines to copy when pressing F9 in terminal panes (default: 50)
    #[serde(default = "default_copy_lines_count")]
    pub copy_lines_count: usize,
}

fn default_true() -> bool {
    true
}

fn default_copy_lines_count() -> usize {
    50
}

impl Default for PtyConfig {
    fn default() -> Self {
        Self {
            // Empty = use terminal shell (Fish/Bash), user starts claude manually
            claude_command: vec![],
            lazygit_command: vec!["lazygit".to_string()],
            scrollback_lines: 1000,
            auto_restart: true,
            copy_lines_count: 50,
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
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ClaudeConfig {
    /// Startup prefixes shown in dialog
    #[serde(default)]
    pub startup_prefixes: Vec<ClaudePrefix>,
    /// Default permission mode (if set, skips the dialog)
    #[serde(default)]
    pub default_permission_mode: Option<ClaudePermissionMode>,
    /// Show permission mode selection dialog at startup (default: true)
    #[serde(default = "default_show_permission_dialog")]
    pub show_permission_dialog: bool,
    /// Enable remote control mode (claude remote-control) for session sharing
    #[serde(default)]
    pub remote_control: bool,
}

impl Default for ClaudeConfig {
    fn default() -> Self {
        Self {
            startup_prefixes: Vec::new(),
            default_permission_mode: None,
            show_permission_dialog: true, // Dialog is shown by default
            remote_control: false,
        }
    }
}

fn default_show_permission_dialog() -> bool {
    true
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct TerminalConfig {
    pub shell_path: String,
    pub shell_args: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct UiConfig {
    pub theme: String,
    #[serde(default)]
    pub autosave: bool,
    #[serde(default = "default_true")]
    pub show_file_browser: bool,
    #[serde(default)]
    pub show_terminal: bool,
    #[serde(default)]
    pub show_lazygit: bool,
    #[serde(default = "default_true")]
    pub show_preview: bool,
    /// Browser command for opening files (empty = system default)
    #[serde(default)]
    pub browser: String,
    /// External GUI editor command (empty = not configured)
    #[serde(default)]
    pub external_editor: String,
    /// Default export directory for Markdown/PDF exports (empty = ~/Downloads)
    #[serde(default)]
    pub export_dir: String,
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
    pub auto_refresh_ms: u64, // 0 = disabled
}

impl Default for FileBrowserConfig {
    fn default() -> Self {
        Self {
            show_hidden: true, // Show hidden files by default (toggle with '.')
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
                autosave: false,
                show_file_browser: true,
                show_terminal: false,
                show_lazygit: false,
                show_preview: true,
                browser: String::new(),
                external_editor: String::new(),
                export_dir: String::new(),
            },
            layout: LayoutConfig::default(),
            file_browser: FileBrowserConfig::default(),
            pty: PtyConfig::default(),
            setup: SetupConfig::default(),
            claude: ClaudeConfig::default(),
            document: DocumentConfig::default(),
        }
    }
}

// --- Document export configuration (HTML/PDF templates) ---

/// Company/branding configuration for document exports
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct CompanyConfig {
    /// Company name used in footer and author fields
    #[serde(default = "default_company_name")]
    pub name: String,
    /// Footer text template — {company_name} is replaced with company.name
    #[serde(default = "default_footer_text")]
    pub footer_text: String,
    /// Author text template — {company_name} is replaced with company.name
    #[serde(default = "default_author_text")]
    pub author: String,
    /// Company website (optional, shown in footer if set)
    #[serde(default)]
    pub website: String,
}

fn default_company_name() -> String {
    "Claude Workbench".to_string()
}
fn default_footer_text() -> String {
    "Generated by {company_name}".to_string()
}
fn default_author_text() -> String {
    "{company_name}".to_string()
}

impl Default for CompanyConfig {
    fn default() -> Self {
        Self {
            name: default_company_name(),
            footer_text: default_footer_text(),
            author: default_author_text(),
            website: String::new(),
        }
    }
}

/// Font configuration for document exports
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct DocFontConfig {
    /// Body font family (CSS font-family for HTML, font name for PDF)
    #[serde(default = "default_body_font")]
    pub body: String,
    /// Code/monospace font family
    #[serde(default = "default_code_font")]
    pub code: String,
}

fn default_body_font() -> String {
    "Calibri, -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Oxygen, Ubuntu, sans-serif"
        .to_string()
}
fn default_code_font() -> String {
    "'SF Mono', Monaco, 'Cascadia Code', Consolas, monospace".to_string()
}

impl Default for DocFontConfig {
    fn default() -> Self {
        Self {
            body: default_body_font(),
            code: default_code_font(),
        }
    }
}

/// Color configuration for document exports
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct DocColorConfig {
    /// Accent color for interactive elements
    #[serde(default = "default_accent_color")]
    pub accent: String,
    /// Table header background color
    #[serde(default = "default_table_header_bg")]
    pub table_header_bg: String,
    /// Table border color
    #[serde(default = "default_table_border")]
    pub table_border: String,
    /// Link color
    #[serde(default = "default_link_color")]
    pub link: String,
    /// Footer text color
    #[serde(default = "default_footer_color")]
    pub footer: String,
    /// Header/footer separator line color
    #[serde(default = "default_header_border")]
    pub header_border: String,
}

fn default_accent_color() -> String {
    "#0366d6".to_string()
}
fn default_table_header_bg() -> String {
    "#D5E8F0".to_string()
}
fn default_table_border() -> String {
    "#999999".to_string()
}
fn default_link_color() -> String {
    "#0366d6".to_string()
}
fn default_footer_color() -> String {
    "#999999".to_string()
}
fn default_header_border() -> String {
    "#999999".to_string()
}

impl Default for DocColorConfig {
    fn default() -> Self {
        Self {
            accent: default_accent_color(),
            table_header_bg: default_table_header_bg(),
            table_border: default_table_border(),
            link: default_link_color(),
            footer: default_footer_color(),
            header_border: default_header_border(),
        }
    }
}

/// Font size configuration for document exports (CSS-compatible strings, e.g. "10pt")
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct DocSizeConfig {
    /// Title font size
    #[serde(default = "default_size_title")]
    pub title: String,
    /// Heading 1 font size
    #[serde(default = "default_size_h1")]
    pub h1: String,
    /// Heading 2 font size
    #[serde(default = "default_size_h2")]
    pub h2: String,
    /// Heading 3 font size
    #[serde(default = "default_size_h3")]
    pub h3: String,
    /// Body text font size
    #[serde(default = "default_size_body")]
    pub body: String,
    /// Table text font size
    #[serde(default = "default_size_table")]
    pub table: String,
    /// Footer text font size
    #[serde(default = "default_size_footer")]
    pub footer: String,
}

fn default_size_title() -> String {
    "16pt".to_string()
}
fn default_size_h1() -> String {
    "14pt".to_string()
}
fn default_size_h2() -> String {
    "12pt".to_string()
}
fn default_size_h3() -> String {
    "11pt".to_string()
}
fn default_size_body() -> String {
    "10pt".to_string()
}
fn default_size_table() -> String {
    "9pt".to_string()
}
fn default_size_footer() -> String {
    "8pt".to_string()
}

impl Default for DocSizeConfig {
    fn default() -> Self {
        Self {
            title: default_size_title(),
            h1: default_size_h1(),
            h2: default_size_h2(),
            h3: default_size_h3(),
            body: default_size_body(),
            table: default_size_table(),
            footer: default_size_footer(),
        }
    }
}

/// PDF page configuration
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct PdfPageConfig {
    /// Page size (e.g. "A4", "Letter")
    #[serde(default = "default_page_size")]
    pub page_size: String,
    /// Page margin (e.g. "2.5cm", "1in")
    #[serde(default = "default_page_margin")]
    pub margin: String,
}

fn default_page_size() -> String {
    "A4".to_string()
}
fn default_page_margin() -> String {
    "2.5cm".to_string()
}

impl Default for PdfPageConfig {
    fn default() -> Self {
        Self {
            page_size: default_page_size(),
            margin: default_page_margin(),
        }
    }
}

/// Central document configuration for HTML and PDF exports
#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct DocumentConfig {
    #[serde(default)]
    pub company: CompanyConfig,
    #[serde(default)]
    pub fonts: DocFontConfig,
    #[serde(default)]
    pub colors: DocColorConfig,
    #[serde(default)]
    pub sizes: DocSizeConfig,
    #[serde(default)]
    pub pdf: PdfPageConfig,
}

impl DocumentConfig {
    /// Resolve {company_name} placeholder in footer_text
    pub fn resolved_footer_text(&self) -> String {
        self.company
            .footer_text
            .replace("{company_name}", &self.company.name)
    }

    /// Resolve {company_name} placeholder in author
    pub fn resolved_author(&self) -> String {
        self.company
            .author
            .replace("{company_name}", &self.company.name)
    }

    /// Resolve footer text with date appended
    pub fn resolved_footer_with_date(&self, date: &str) -> String {
        format!("{} \u{2014} {}", self.resolved_footer_text(), date)
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
        let config: Config = serde_yaml_ng::from_str(&contents)?;
        return Ok(config);
    }

    // 2. Check ~/.config/claude-workbench/config.yaml (XDG-style)
    if let Some(config_dir) = get_config_dir() {
        let config_path = config_dir.join("config.yaml");
        if config_path.exists() {
            let contents = fs::read_to_string(&config_path)?;
            let config: Config = serde_yaml_ng::from_str(&contents)?;
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
        let yaml = serde_yaml_ng::to_string(config)?;
        fs::write(local_config, &yaml)?;
        set_restrictive_permissions(local_config)?;
        return Ok(());
    }

    // Otherwise save to XDG config directory
    if let Some(config_dir) = get_config_dir() {
        let config_path = config_dir.join("config.yaml");
        fs::create_dir_all(&config_dir)?;
        let yaml = serde_yaml_ng::to_string(config)?;
        fs::write(&config_path, &yaml)?;
        set_restrictive_permissions(&config_path)?;
    }
    Ok(())
}

/// Get the config file path (for display purposes)
pub fn get_config_path() -> Option<std::path::PathBuf> {
    get_config_dir().map(|d| d.join("config.yaml"))
}
