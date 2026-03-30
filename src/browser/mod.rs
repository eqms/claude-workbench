//! Browser preview module
//!
//! Provides functionality to open files in the system browser
//! and convert Markdown to HTML for preview.

pub mod markdown;
pub mod opener;
pub mod pdf_export;
pub mod syntax;
pub mod template;
pub mod typst_pdf;

pub use markdown::*;
pub use opener::*;
pub use syntax::*;

/// Convert heading or anchor text to a URL-style slug (GitHub-flavored Markdown convention).
///
/// Used by both the HTML exporter (for `id=` attributes) and the Typst/PDF exporter
/// (for Typst labels). Shared here to keep both exporters in sync.
pub(crate) fn slugify(text: &str) -> String {
    text.to_lowercase()
        .chars()
        .map(|c| {
            if c.is_alphanumeric() {
                c
            } else if c == ' ' || c == '-' || c == '_' {
                '-'
            } else {
                '\0'
            }
        })
        .filter(|c| *c != '\0')
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}
