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
