//! Central syntax registry for file type detection and syntax highlighting.
//!
//! Single source of truth for mapping file extensions and filenames to syntect
//! syntax definitions. Used by both browser preview (`browser/syntax.rs`) and
//! TUI preview (`ui/syntax.rs`).

use std::path::Path;
use syntect::parsing::{SyntaxReference, SyntaxSet};

/// Extra extension mappings for file types that syntect doesn't know natively.
/// Maps unknown extensions to a known syntect extension for best-effort highlighting.
const EXTRA_EXTENSION_MAPPINGS: &[(&str, &str)] = &[
    // Config formats
    ("toml", "yaml"),
    ("conf", "sh"),
    ("cfg", "properties"),
    ("ini", "properties"),
    ("cnf", "properties"),
    ("env", "sh"),
    ("lock", "yaml"),
    // Systemd units
    ("service", "sh"),
    ("socket", "sh"),
    ("timer", "sh"),
    ("mount", "sh"),
    ("target", "sh"),
    ("desktop", "sh"),
    // Infrastructure / IaC
    ("tf", "sh"),
    ("tfvars", "sh"),
    ("hcl", "sh"),
    // PowerShell
    ("ps1", "sh"),
    ("psm1", "sh"),
    ("psd1", "sh"),
    // Schema / API
    ("graphql", "txt"),
    ("gql", "txt"),
    ("proto", "txt"),
    // Web / Node config
    ("htaccess", "sh"),
    ("npmrc", "properties"),
    ("yarnrc", "yaml"),
    ("editorconfig", "properties"),
    // Nix / Vim
    ("nix", "sh"),
    ("vim", "sh"),
    ("vimrc", "sh"),
    // Apple
    ("plist", "xml"),
    // Documentation
    ("adoc", "txt"),
    ("asciidoc", "txt"),
    // Shell-adjacent
    ("awk", "sh"),
    ("sed", "sh"),
    ("crontab", "sh"),
    // Data / Logs
    ("csv", "txt"),
    ("tsv", "txt"),
    ("log", "txt"),
    // Windows
    ("reg", "properties"),
    // Docker (syntect doesn't register "dockerfile" as extension)
    ("dockerfile", "sh"),
];

/// Full filename mappings for dotfiles and special files without extensions.
/// Maps exact (lowercased) filenames to a known syntect extension.
const FILENAME_MAPPINGS: &[(&str, &str)] = &[
    // Build systems
    ("makefile", "makefile"),
    ("gnumakefile", "makefile"),
    ("cmakelists.txt", "txt"),
    ("justfile", "sh"),
    ("procfile", "sh"),
    ("brewfile", "rb"),
    ("vagrantfile", "rb"),
    ("gemfile", "rb"),
    ("rakefile", "rb"),
    // Git
    (".gitignore", "sh"),
    (".gitattributes", "sh"),
    (".gitconfig", "properties"),
    (".gitmodules", "properties"),
    // Docker
    ("dockerfile", "sh"),
    (".dockerignore", "sh"),
    // VCS
    (".hgignore", "sh"),
    // Editor config
    (".editorconfig", "properties"),
    // Node / JS
    (".npmrc", "properties"),
    (".yarnrc", "yaml"),
    (".prettierrc", "json"),
    (".eslintrc", "json"),
    (".babelrc", "json"),
    // Python
    (".flake8", "properties"),
    (".pylintrc", "properties"),
    (".coveragerc", "properties"),
    // Apache
    (".htaccess", "sh"),
    // Shell RC files
    (".bashrc", "sh"),
    (".bash_profile", "sh"),
    (".bash_aliases", "sh"),
    (".zshrc", "sh"),
    (".zprofile", "sh"),
    (".zshenv", "sh"),
    (".profile", "sh"),
    (".inputrc", "sh"),
    // Env
    (".env", "sh"),
];

/// Human-readable display names for extra extensions.
const EXTENSION_DISPLAY_NAMES: &[(&str, &str)] = &[
    ("toml", "TOML"),
    ("conf", "Config"),
    ("cfg", "Config"),
    ("ini", "INI"),
    ("cnf", "Config"),
    ("env", "Environment"),
    ("lock", "Lock File"),
    ("service", "Systemd Service"),
    ("socket", "Systemd Socket"),
    ("timer", "Systemd Timer"),
    ("mount", "Systemd Mount"),
    ("target", "Systemd Target"),
    ("desktop", "Desktop Entry"),
    ("tf", "Terraform"),
    ("tfvars", "Terraform Vars"),
    ("hcl", "HCL"),
    ("ps1", "PowerShell"),
    ("psm1", "PowerShell Module"),
    ("psd1", "PowerShell Data"),
    ("graphql", "GraphQL"),
    ("gql", "GraphQL"),
    ("proto", "Protocol Buffers"),
    ("htaccess", "Apache Config"),
    ("npmrc", "npm Config"),
    ("yarnrc", "Yarn Config"),
    ("editorconfig", "EditorConfig"),
    ("nix", "Nix"),
    ("vim", "VimScript"),
    ("vimrc", "VimScript"),
    ("plist", "Property List"),
    ("adoc", "AsciiDoc"),
    ("asciidoc", "AsciiDoc"),
    ("awk", "AWK"),
    ("sed", "sed"),
    ("crontab", "Crontab"),
    ("csv", "CSV"),
    ("tsv", "TSV"),
    ("log", "Log"),
    ("reg", "Windows Registry"),
    ("dockerfile", "Dockerfile"),
];

/// Human-readable display names for special filenames.
const FILENAME_DISPLAY_NAMES: &[(&str, &str)] = &[
    ("makefile", "Makefile"),
    ("gnumakefile", "Makefile"),
    ("cmakelists.txt", "CMake"),
    ("justfile", "Justfile"),
    ("procfile", "Procfile"),
    ("brewfile", "Brewfile"),
    ("vagrantfile", "Vagrantfile"),
    ("gemfile", "Ruby (Gemfile)"),
    ("rakefile", "Ruby (Rakefile)"),
    ("dockerfile", "Dockerfile"),
    (".gitignore", "Git Ignore"),
    (".gitattributes", "Git Attributes"),
    (".gitconfig", "Git Config"),
    (".gitmodules", "Git Modules"),
    (".dockerignore", "Docker Ignore"),
    (".hgignore", "Hg Ignore"),
    (".editorconfig", "EditorConfig"),
    (".npmrc", "npm Config"),
    (".yarnrc", "Yarn Config"),
    (".prettierrc", "Prettier Config"),
    (".eslintrc", "ESLint Config"),
    (".babelrc", "Babel Config"),
    (".flake8", "Flake8 Config"),
    (".pylintrc", "Pylint Config"),
    (".coveragerc", "Coverage Config"),
    (".htaccess", "Apache Config"),
    (".bashrc", "Bash Config"),
    (".bash_profile", "Bash Profile"),
    (".bash_aliases", "Bash Aliases"),
    (".zshrc", "Zsh Config"),
    (".zprofile", "Zsh Profile"),
    (".zshenv", "Zsh Env"),
    (".profile", "Shell Profile"),
    (".inputrc", "Readline Config"),
    (".env", "Environment"),
];

/// Find the best syntax definition for a given file path.
///
/// Lookup order:
/// 1. Syntect native: `find_syntax_by_extension(ext)`
/// 2. Extra extension mapping → syntect extension
/// 3. Syntect native: `find_syntax_by_extension(filename)` (catches `.bashrc`, `Gemfile`, etc.)
/// 4. Filename mapping → syntect extension
/// 5. Prefix patterns (`.env.*` → sh)
/// 6. Fallback: Plain Text
pub fn find_syntax_for_path<'a>(path: &Path, ss: &'a SyntaxSet) -> &'a SyntaxReference {
    let ext = path.extension().and_then(|e| e.to_str());
    let filename = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("");
    let lower_name = filename.to_lowercase();

    // 1. Syntect native by extension
    if let Some(ext) = ext {
        if let Some(syn) = ss.find_syntax_by_extension(ext) {
            return syn;
        }

        // 2. Extra extension mapping
        let lower_ext = ext.to_lowercase();
        for &(mapped_ext, target) in EXTRA_EXTENSION_MAPPINGS {
            if lower_ext == mapped_ext {
                if let Some(syn) = ss.find_syntax_by_extension(target) {
                    return syn;
                }
            }
        }
    }

    // 3. Syntect native by full filename (catches .bashrc, Makefile, etc.)
    if let Some(syn) = ss.find_syntax_by_extension(filename) {
        return syn;
    }

    // 4. Filename mapping
    for &(mapped_name, target) in FILENAME_MAPPINGS {
        if lower_name == mapped_name {
            if let Some(syn) = ss.find_syntax_by_extension(target) {
                return syn;
            }
        }
    }

    // 5. Prefix patterns
    if lower_name.starts_with(".env") {
        if let Some(syn) = ss.find_syntax_by_extension("sh") {
            return syn;
        }
    }

    // 6. Fallback
    ss.find_syntax_plain_text()
}

/// Check if a file path can be recognized as a text file for preview/highlighting.
///
/// Returns true if syntect knows the extension natively, or if we have a mapping for it.
pub fn is_known_text_file(path: &Path, ss: &SyntaxSet) -> bool {
    let ext = path.extension().and_then(|e| e.to_str());
    let filename = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("");
    let lower_name = filename.to_lowercase();

    // 1. Syntect native by extension
    if let Some(ext) = ext {
        if ss.find_syntax_by_extension(ext).is_some() {
            return true;
        }
        // 2. Extra extension mapping
        let lower_ext = ext.to_lowercase();
        for &(mapped_ext, _) in EXTRA_EXTENSION_MAPPINGS {
            if lower_ext == mapped_ext {
                return true;
            }
        }
    }

    // 3. Syntect native by full filename
    if ss.find_syntax_by_extension(filename).is_some() {
        return true;
    }

    // 4. Filename mapping
    for &(mapped_name, _) in FILENAME_MAPPINGS {
        if lower_name == mapped_name {
            return true;
        }
    }

    // 5. Prefix patterns
    if lower_name.starts_with(".env") {
        return true;
    }

    false
}

/// Get a human-readable display name for the file's language/type.
pub fn display_name_for_path(path: &Path, ss: &SyntaxSet) -> String {
    let ext = path.extension().and_then(|e| e.to_str());
    let filename = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("");
    let lower_name = filename.to_lowercase();

    // 1. Syntect native by extension → use syntect's name
    if let Some(ext) = ext {
        if let Some(syn) = ss.find_syntax_by_extension(ext) {
            return syn.name.clone();
        }
        // 2. Extra extension display name
        let lower_ext = ext.to_lowercase();
        for &(mapped_ext, display) in EXTENSION_DISPLAY_NAMES {
            if lower_ext == mapped_ext {
                return display.to_string();
            }
        }
    }

    // 3. Syntect native by full filename
    if let Some(syn) = ss.find_syntax_by_extension(filename) {
        return syn.name.clone();
    }

    // 4. Filename display name
    for &(mapped_name, display) in FILENAME_DISPLAY_NAMES {
        if lower_name == mapped_name {
            return display.to_string();
        }
    }

    // 5. Prefix patterns
    if lower_name.starts_with(".env") {
        return "Environment".to_string();
    }

    "Plain Text".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ss() -> SyntaxSet {
        SyntaxSet::load_defaults_newlines()
    }

    #[test]
    fn test_all_extension_mappings_resolve() {
        let ss = ss();
        for &(ext, target) in EXTRA_EXTENSION_MAPPINGS {
            assert!(
                ss.find_syntax_by_extension(target).is_some(),
                "EXTRA_EXTENSION_MAPPINGS: extension '{}' maps to '{}' which syntect cannot find",
                ext,
                target
            );
        }
    }

    #[test]
    fn test_all_filename_mappings_resolve() {
        let ss = ss();
        for &(name, target) in FILENAME_MAPPINGS {
            assert!(
                ss.find_syntax_by_extension(target).is_some(),
                "FILENAME_MAPPINGS: filename '{}' maps to '{}' which syntect cannot find",
                name,
                target
            );
        }
    }

    #[test]
    fn test_is_known_text_file_new_extensions() {
        let ss = ss();
        let cases = [
            "config.toml",
            "nginx.conf",
            "settings.cfg",
            "php.ini",
            "my.cnf",
            "app.env",
            "Cargo.lock",
            "unit.service",
            "unit.socket",
            "unit.timer",
            "main.tf",
            "vars.tfvars",
            "script.ps1",
            "schema.graphql",
            "api.gql",
            "message.proto",
            ".htaccess",
            "app.desktop",
            "default.nix",
            "init.vim",
            "Info.plist",
            "readme.adoc",
            "script.awk",
            "script.sed",
            "jobs.crontab",
            "data.csv",
            "data.tsv",
            "app.log",
            "settings.reg",
        ];
        for case in &cases {
            assert!(
                is_known_text_file(Path::new(case), &ss),
                "Expected '{}' to be recognized as a known text file",
                case
            );
        }
    }

    #[test]
    fn test_is_known_text_file_syntect_native() {
        let ss = ss();
        // These should already be known by syntect
        let native = [
            "test.rs",
            "test.py",
            "test.js",
            "test.java",
            "test.c",
            "test.cpp",
            "test.h",
            "test.rb",
            "config.yaml",
            "config.json",
            "script.sh",
            "page.html",
            "style.css",
            "data.xml",
        ];
        for case in &native {
            assert!(
                is_known_text_file(Path::new(case), &ss),
                "Expected '{}' (syntect native) to be recognized",
                case
            );
        }
    }

    #[test]
    fn test_known_filenames() {
        let ss = ss();
        let filenames = [
            ".dockerignore",
            ".npmrc",
            ".gitconfig",
            ".gitignore",
            ".gitattributes",
            ".gitmodules",
            ".bashrc",
            ".zshrc",
            ".profile",
            ".editorconfig",
            ".flake8",
            ".pylintrc",
            ".coveragerc",
            ".htaccess",
            ".prettierrc",
            ".eslintrc",
            ".babelrc",
            "Makefile",
            "Dockerfile",
            "Gemfile",
            "Rakefile",
            "Vagrantfile",
            "Justfile",
            "Procfile",
            "Brewfile",
            "CMakeLists.txt",
        ];
        for name in &filenames {
            assert!(
                is_known_text_file(Path::new(name), &ss),
                "Expected filename '{}' to be recognized",
                name
            );
        }
    }

    #[test]
    fn test_display_names() {
        let ss = ss();
        assert_eq!(
            display_name_for_path(Path::new("config.toml"), &ss),
            "TOML"
        );
        assert_eq!(
            display_name_for_path(Path::new("nginx.conf"), &ss),
            "Config"
        );
        assert_eq!(
            display_name_for_path(Path::new("settings.ini"), &ss),
            "INI"
        );
        assert_eq!(
            display_name_for_path(Path::new("data.csv"), &ss),
            "CSV"
        );
        assert_eq!(
            display_name_for_path(Path::new("app.log"), &ss),
            "Log"
        );
        assert_eq!(
            display_name_for_path(Path::new("main.tf"), &ss),
            "Terraform"
        );
        assert_eq!(
            display_name_for_path(Path::new("schema.graphql"), &ss),
            "GraphQL"
        );
        assert_eq!(
            display_name_for_path(Path::new("Makefile"), &ss),
            "Makefile"
        );
        assert_eq!(
            display_name_for_path(Path::new("Dockerfile"), &ss),
            "Dockerfile"
        );
        // .bashrc is recognized natively by syntect as "Bourne Again Shell (bash)"
        assert_eq!(
            display_name_for_path(Path::new(".bashrc"), &ss),
            "Bourne Again Shell (bash)"
        );
        // .gitignore is recognized natively by syntect as "Git Ignore"
        let gitignore_name = display_name_for_path(Path::new(".gitignore"), &ss);
        assert!(
            gitignore_name == "Git Ignore" || gitignore_name == "Git Common",
            "Expected Git-related name for .gitignore, got '{}'",
            gitignore_name
        );
    }

    #[test]
    fn test_env_prefix_pattern() {
        let ss = ss();
        assert!(is_known_text_file(Path::new(".env"), &ss));
        assert!(is_known_text_file(Path::new(".env.local"), &ss));
        assert!(is_known_text_file(Path::new(".env.production"), &ss));
        assert!(is_known_text_file(Path::new(".env.development"), &ss));

        assert_eq!(
            display_name_for_path(Path::new(".env.local"), &ss),
            "Environment"
        );
    }

    #[test]
    fn test_non_text_files_not_recognized() {
        let ss = ss();
        assert!(!is_known_text_file(Path::new("image.png"), &ss));
        assert!(!is_known_text_file(Path::new("binary.exe"), &ss));
        assert!(!is_known_text_file(Path::new("document.pdf"), &ss));
        assert!(!is_known_text_file(Path::new("archive.zip"), &ss));
        assert!(!is_known_text_file(Path::new("video.mp4"), &ss));
    }

    #[test]
    fn test_find_syntax_returns_non_plain_for_known() {
        let ss = ss();
        let plain = ss.find_syntax_plain_text();

        // These should all resolve to something other than plain text
        let non_plain = [
            "test.rs",
            "config.toml",
            "nginx.conf",
            "script.sh",
            ".bashrc",
            "Makefile",
        ];
        for case in &non_plain {
            let syn = find_syntax_for_path(Path::new(case), &ss);
            assert_ne!(
                syn.name, plain.name,
                "Expected '{}' to get non-plain-text syntax, got '{}'",
                case, syn.name
            );
        }
    }

    #[test]
    fn test_find_syntax_fallback_to_plain() {
        let ss = ss();
        let plain = ss.find_syntax_plain_text();
        let syn = find_syntax_for_path(Path::new("image.png"), &ss);
        assert_eq!(syn.name, plain.name);
    }
}
