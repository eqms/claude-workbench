//! Intelligent filtering for terminal output
//!
//! Filters shell prompts, preserves error messages and stack traces,
//! detects syntax for code blocks.

use regex::Regex;
use std::sync::LazyLock;

/// Filter options for terminal output
#[derive(Debug, Clone)]
pub struct FilterOptions {
    /// Remove shell prompts (user@host:path$, >, %, etc.)
    pub filter_prompts: bool,
    /// Collapse consecutive blank lines to max 2
    pub collapse_blanks: bool,
    /// Preserve Python tracebacks and error messages
    pub preserve_tracebacks: bool,
    /// Filter directory listing noise (drwx, total N)
    pub filter_dir_listings: bool,
    /// Detect programming language for syntax highlighting
    pub detect_syntax: bool,
}

impl Default for FilterOptions {
    fn default() -> Self {
        Self {
            filter_prompts: true,
            collapse_blanks: true,
            preserve_tracebacks: true,
            filter_dir_listings: true,
            detect_syntax: true,
        }
    }
}

/// Result of filtering terminal output
#[derive(Debug, Clone)]
pub struct FilteredOutput {
    /// Filtered lines
    pub lines: Vec<String>,
    /// Detected syntax hint for code block (e.g., "python", "rust")
    pub syntax_hint: Option<String>,
    /// Whether the output contains error messages
    pub contains_error: bool,
}

// Shell prompt patterns
static PROMPT_PATTERNS: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    vec![
        // user@host:path$ or user@host:path#
        Regex::new(r"^[a-zA-Z0-9_-]+@[a-zA-Z0-9._-]+:[^\$#]*[\$#]\s*$")
            .expect("static regex pattern must compile"),
        // Simple prompts: $, >, %, >>> (Python)
        Regex::new(r"^[\$>%]\s*$").expect("static regex pattern must compile"),
        Regex::new(r"^>>>\s*$").expect("static regex pattern must compile"),
        // Zsh themes: ➜, ❯, →
        Regex::new(r"^[➜❯→]\s+").expect("static regex pattern must compile"),
        // Fish prompt with abbreviated path
        Regex::new(r"^[a-zA-Z0-9_-]+@[a-zA-Z0-9._-]+\s+[~\w/]+\s*[\$#>]\s*$")
            .expect("static regex pattern must compile"),
        // PS1 with colors (stripped)
        Regex::new(r"^\[[^\]]*\][a-zA-Z0-9_-]+@[a-zA-Z0-9._-]+")
            .expect("static regex pattern must compile"),
        // Time-prefixed prompts
        Regex::new(r"^\[\d{2}:\d{2}(:\d{2})?\]\s*[\$#>]")
            .expect("static regex pattern must compile"),
        // Just a directory path ending with prompt
        Regex::new(r"^[~\w/\-\.]+\s*[\$#>%]\s*$").expect("static regex pattern must compile"),
    ]
});

// Error patterns to preserve
static ERROR_PATTERNS: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    vec![
        // Python traceback
        Regex::new(r"Traceback \(most recent call last\)")
            .expect("static regex pattern must compile"),
        Regex::new(r#"^\s+File "[^"]+", line \d+"#).expect("static regex pattern must compile"),
        Regex::new(r"^\s+raise ").expect("static regex pattern must compile"),
        Regex::new(r"^[A-Z][a-zA-Z]*Error:").expect("static regex pattern must compile"),
        Regex::new(r"^[A-Z][a-zA-Z]*Exception:").expect("static regex pattern must compile"),
        // Odoo errors
        Regex::new(r"odoo\.exceptions\.").expect("static regex pattern must compile"),
        Regex::new(r"psycopg2\.").expect("static regex pattern must compile"),
        Regex::new(r"^\d{4}-\d{2}-\d{2}\s+\d{2}:\d{2}:\d{2},\d+.*ERROR")
            .expect("static regex pattern must compile"),
        // Rust errors
        Regex::new(r"^error\[E\d+\]:").expect("static regex pattern must compile"),
        Regex::new(r"^\s+-->\s+").expect("static regex pattern must compile"),
        // JavaScript/Node errors
        Regex::new(r"^\s+at\s+").expect("static regex pattern must compile"),
        Regex::new(r"^(TypeError|ReferenceError|SyntaxError):")
            .expect("static regex pattern must compile"),
        // Generic errors
        Regex::new(r"(?i)^error:").expect("static regex pattern must compile"),
        Regex::new(r"(?i)^fatal:").expect("static regex pattern must compile"),
        Regex::new(r"(?i)^panic:").expect("static regex pattern must compile"),
    ]
});

// Directory listing patterns to filter
static DIR_LISTING_PATTERNS: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    vec![
        // Permission strings: drwxr-xr-x, -rw-r--r--
        Regex::new(r"^[d\-][rwx\-]{9}").expect("static regex pattern must compile"),
        // Total line: total 123
        Regex::new(r"^total\s+\d+").expect("static regex pattern must compile"),
    ]
});

// Syntax detection patterns
static PYTHON_PATTERNS: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    vec![
        Regex::new(r"^def\s+\w+\s*\(").expect("static regex pattern must compile"),
        Regex::new(r"^class\s+\w+").expect("static regex pattern must compile"),
        Regex::new(r"^import\s+\w+").expect("static regex pattern must compile"),
        Regex::new(r"^from\s+\w+\s+import").expect("static regex pattern must compile"),
        Regex::new(r"self\.\w+").expect("static regex pattern must compile"),
        Regex::new(r"__init__").expect("static regex pattern must compile"),
    ]
});

static RUST_PATTERNS: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    vec![
        Regex::new(r"^fn\s+\w+").expect("static regex pattern must compile"),
        Regex::new(r"^pub\s+(fn|struct|enum|mod)").expect("static regex pattern must compile"),
        Regex::new(r"^impl\s+").expect("static regex pattern must compile"),
        Regex::new(r"^let\s+(mut\s+)?\w+").expect("static regex pattern must compile"),
        Regex::new(r"^use\s+\w+::").expect("static regex pattern must compile"),
    ]
});

static JAVASCRIPT_PATTERNS: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    vec![
        Regex::new(r"^const\s+\w+\s*=").expect("static regex pattern must compile"),
        Regex::new(r"^let\s+\w+\s*=").expect("static regex pattern must compile"),
        Regex::new(r"^function\s+\w+").expect("static regex pattern must compile"),
        Regex::new(r"^(export\s+)?(default\s+)?class\s+")
            .expect("static regex pattern must compile"),
        Regex::new(r"=>\s*\{?").expect("static regex pattern must compile"),
        Regex::new(r"^import\s+.*\s+from\s+").expect("static regex pattern must compile"),
    ]
});

static BASH_PATTERNS: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    vec![
        Regex::new(r"^#!/bin/(ba)?sh").expect("static regex pattern must compile"),
        Regex::new(r"^export\s+\w+=").expect("static regex pattern must compile"),
        Regex::new(r"^if\s+\[").expect("static regex pattern must compile"),
        Regex::new(r"^\$\{?\w+").expect("static regex pattern must compile"),
    ]
});

static XML_PATTERNS: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    vec![
        Regex::new(r"^<\?xml").expect("static regex pattern must compile"),
        Regex::new(r"^<!DOCTYPE").expect("static regex pattern must compile"),
        Regex::new(r"^<[a-zA-Z_][\w\-]*(\s|>|/)").expect("static regex pattern must compile"),
    ]
});

/// Check if a line matches any shell prompt pattern
fn is_prompt_line(line: &str) -> bool {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return false;
    }
    PROMPT_PATTERNS.iter().any(|p| p.is_match(trimmed))
}

/// Check if a line is part of an error or traceback
fn is_error_line(line: &str) -> bool {
    ERROR_PATTERNS.iter().any(|p| p.is_match(line))
}

/// Check if a line is directory listing noise
fn is_dir_listing_line(line: &str) -> bool {
    DIR_LISTING_PATTERNS.iter().any(|p| p.is_match(line))
}

/// Detect the programming language from the content
fn detect_language(lines: &[String]) -> Option<String> {
    let mut scores: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();

    for line in lines {
        // Python
        for pattern in PYTHON_PATTERNS.iter() {
            if pattern.is_match(line) {
                *scores.entry("python").or_insert(0) += 1;
            }
        }
        // Rust
        for pattern in RUST_PATTERNS.iter() {
            if pattern.is_match(line) {
                *scores.entry("rust").or_insert(0) += 1;
            }
        }
        // JavaScript
        for pattern in JAVASCRIPT_PATTERNS.iter() {
            if pattern.is_match(line) {
                *scores.entry("javascript").or_insert(0) += 1;
            }
        }
        // Bash
        for pattern in BASH_PATTERNS.iter() {
            if pattern.is_match(line) {
                *scores.entry("bash").or_insert(0) += 1;
            }
        }
        // XML
        for pattern in XML_PATTERNS.iter() {
            if pattern.is_match(line) {
                *scores.entry("xml").or_insert(0) += 1;
            }
        }
    }

    // Return the language with the highest score (if any)
    scores
        .into_iter()
        .max_by_key(|(_, score)| *score)
        .filter(|(_, score)| *score >= 2) // Require at least 2 matches
        .map(|(lang, _)| lang.to_string())
}

/// Filter terminal output according to options
pub fn filter_lines(input: Vec<String>, options: &FilterOptions) -> FilteredOutput {
    let mut filtered_lines = Vec::new();
    let mut contains_error = false;
    let mut consecutive_blanks = 0;
    let mut in_traceback = false;

    for line in &input {
        let trimmed = line.trim();

        // Check for error/traceback patterns
        if options.preserve_tracebacks && is_error_line(line) {
            contains_error = true;
            in_traceback = true;
        }

        // Reset traceback state on blank line after traceback
        if in_traceback && trimmed.is_empty() {
            // Keep the blank line but check if we're exiting traceback
            if consecutive_blanks > 0 {
                in_traceback = false;
            }
        }

        // Filter prompts (unless in traceback)
        if options.filter_prompts && !in_traceback && is_prompt_line(line) {
            continue;
        }

        // Filter directory listings (unless in traceback)
        if options.filter_dir_listings && !in_traceback && is_dir_listing_line(line) {
            continue;
        }

        // Collapse consecutive blank lines
        if options.collapse_blanks {
            if trimmed.is_empty() {
                consecutive_blanks += 1;
                if consecutive_blanks > 2 {
                    continue;
                }
            } else {
                consecutive_blanks = 0;
            }
        }

        filtered_lines.push(line.clone());
    }

    // Trim trailing blank lines
    while filtered_lines
        .last()
        .map(|l| l.trim().is_empty())
        .unwrap_or(false)
    {
        filtered_lines.pop();
    }

    // Detect syntax
    let syntax_hint = if options.detect_syntax {
        detect_language(&filtered_lines)
    } else {
        None
    };

    FilteredOutput {
        lines: filtered_lines,
        syntax_hint,
        contains_error,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prompt_filtering() {
        let input = vec![
            "user@host:~$ ".to_string(),
            "ls -la".to_string(),
            "total 123".to_string(),
            "drwxr-xr-x 2 user user 4096 Jan 1 00:00 .".to_string(),
            "$ ".to_string(),
        ];
        let options = FilterOptions::default();
        let result = filter_lines(input, &options);
        assert_eq!(result.lines.len(), 1);
        assert_eq!(result.lines[0], "ls -la");
    }

    #[test]
    fn test_traceback_preservation() {
        let input = vec![
            "Traceback (most recent call last):".to_string(),
            "  File \"test.py\", line 10".to_string(),
            "    raise ValueError()".to_string(),
            "ValueError: test error".to_string(),
        ];
        let options = FilterOptions::default();
        let result = filter_lines(input, &options);
        assert!(result.contains_error);
        assert_eq!(result.lines.len(), 4);
    }

    #[test]
    fn test_syntax_detection() {
        let input = vec![
            "def hello():".to_string(),
            "    print('Hello')".to_string(),
            "".to_string(),
            "class MyClass:".to_string(),
            "    def __init__(self):".to_string(),
            "        pass".to_string(),
        ];
        let options = FilterOptions::default();
        let result = filter_lines(input, &options);
        assert_eq!(result.syntax_hint, Some("python".to_string()));
    }

    #[test]
    fn test_blank_line_collapse() {
        let input = vec![
            "line 1".to_string(),
            "".to_string(),
            "".to_string(),
            "".to_string(),
            "".to_string(),
            "line 2".to_string(),
        ];
        let options = FilterOptions::default();
        let result = filter_lines(input, &options);
        // Should have: line 1, blank, blank, line 2 (max 2 blanks)
        assert_eq!(result.lines.len(), 4);
    }
}
