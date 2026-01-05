//! Browser preview module
//!
//! Provides functionality to open files in the system browser
//! and convert Markdown to HTML for preview.

pub mod markdown;
pub mod opener;

pub use markdown::*;
pub use opener::*;
