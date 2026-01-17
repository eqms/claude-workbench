//! Dependency checker for verifying required and optional tools

use std::path::PathBuf;
use std::process::Command;

/// Result of checking a single dependency
#[derive(Debug, Clone, Default)]
pub struct DependencyStatus {
    pub name: String,
    pub found: bool,
    pub path: Option<PathBuf>,
    pub version: Option<String>,
    pub required: bool,
}

/// All dependencies checked at startup
#[derive(Debug, Clone, Default)]
pub struct DependencyReport {
    pub git: DependencyStatus,
    pub claude_cli: DependencyStatus,
    pub lazygit: DependencyStatus,
    pub shells: Vec<DependencyStatus>,
}

impl DependencyReport {
    /// Check all dependencies and return report
    pub fn check() -> Self {
        Self {
            git: check_command("git", &["--version"], true),
            claude_cli: check_command("claude", &["--version"], false),
            lazygit: check_command("lazygit", &["--version"], false),
            shells: check_available_shells(),
        }
    }

    /// Returns true if all required dependencies are met
    pub fn all_required_met(&self) -> bool {
        !self.git.required || self.git.found
    }

    /// Returns true if any optional but recommended dep is missing
    pub fn has_missing_optional(&self) -> bool {
        !self.claude_cli.found || !self.lazygit.found
    }

    /// Get summary counts
    pub fn summary(&self) -> (usize, usize, usize) {
        let mut found = 0;
        let mut missing_required = 0;
        let mut missing_optional = 0;

        // Git
        if self.git.found {
            found += 1;
        } else if self.git.required {
            missing_required += 1;
        }

        // Claude CLI
        if self.claude_cli.found {
            found += 1;
        } else {
            missing_optional += 1;
        }

        // LazyGit
        if self.lazygit.found {
            found += 1;
        } else {
            missing_optional += 1;
        }

        // Shells
        for shell in &self.shells {
            if shell.found {
                found += 1;
            }
        }

        (found, missing_required, missing_optional)
    }
}

/// Check if a command exists and get its version
fn check_command(name: &str, args: &[&str], required: bool) -> DependencyStatus {
    // First try direct execution
    let result = Command::new(name).args(args).output();

    if let Ok(output) = result {
        if output.status.success() {
            let version = extract_version(&output.stdout, name);
            let path = find_executable_path(name);

            return DependencyStatus {
                name: name.to_string(),
                found: true,
                path,
                version,
                required,
            };
        }
    }

    // If direct execution fails, try via user's shell in interactive mode
    // This handles shell functions/aliases like Claude Code
    let shell_cmd = format!("{} {}", name, args.join(" "));

    // Get user's default shell from $SHELL environment variable
    let user_shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".to_string());

    // Try interactive shell mode (-i flag) to load shell functions/aliases
    let shell_result = Command::new(&user_shell)
        .args(["-i", "-c", &shell_cmd])
        .output();

    match shell_result {
        Ok(output) if output.status.success() => {
            let version = extract_version(&output.stdout, name);
            // For shell functions, path might not be available
            let path = find_executable_path(name);

            DependencyStatus {
                name: name.to_string(),
                found: true,
                path,
                version,
                required,
            }
        }
        _ => DependencyStatus {
            name: name.to_string(),
            found: false,
            path: None,
            version: None,
            required,
        },
    }
}

/// Extract version from command output
fn extract_version(stdout: &[u8], name: &str) -> Option<String> {
    let output = String::from_utf8_lossy(stdout);
    let first_line = output.lines().next()?.trim().to_string();

    // Special handling for Claude Code: "2.0.76 (Claude Code)"
    if name == "claude" {
        // Return just the version number
        if let Some(version) = first_line.split_whitespace().next() {
            return Some(version.to_string());
        }
    }

    Some(first_line)
}

/// Find executable path using 'which' command
fn find_executable_path(name: &str) -> Option<PathBuf> {
    let output = Command::new("which").arg(name).output().ok()?;

    if output.status.success() {
        let path_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !path_str.is_empty() {
            // Handle Fish/Zsh alias output like "claude: aliased to /path/to/claude"
            if path_str.contains("aliased to ") {
                if let Some(actual_path) = path_str.split("aliased to ").nth(1) {
                    let path = PathBuf::from(actual_path.trim());
                    if path.exists() {
                        return Some(path);
                    }
                }
            }
            // Standard path output
            let path = PathBuf::from(&path_str);
            if path.exists() && path.is_absolute() {
                return Some(path);
            }
        }
    }

    // Fallback: Check common Claude installation locations
    if name == "claude" {
        let common_paths = [
            dirs::home_dir().map(|h| h.join(".claude/local/claude")),
            Some(PathBuf::from("/usr/local/bin/claude")),
            dirs::home_dir().map(|h| h.join(".local/bin/claude")),
        ];
        for path_opt in common_paths.into_iter().flatten() {
            if path_opt.exists() {
                return Some(path_opt);
            }
        }
    }

    None
}

/// Check for available shells
fn check_available_shells() -> Vec<DependencyStatus> {
    ["bash", "zsh", "fish", "sh"]
        .iter()
        .map(|shell| check_command(shell, &["--version"], false))
        .filter(|s| s.found) // Only include found shells
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dependency_check() {
        let report = DependencyReport::check();
        // Git should be available in most dev environments
        // This is more of an integration test
        println!("Git found: {}", report.git.found);
        println!("Claude CLI found: {}", report.claude_cli.found);
        println!("LazyGit found: {}", report.lazygit.found);
        println!(
            "Shells found: {:?}",
            report.shells.iter().map(|s| &s.name).collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_check_command_not_found() {
        let status = check_command("nonexistent_command_12345", &["--version"], false);
        assert!(!status.found);
        assert!(status.path.is_none());
    }
}
