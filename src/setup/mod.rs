//! Setup module for installation wizard, dependency checking, and templates

pub mod dependency_checker;
pub mod templates;
pub mod wizard;

pub use dependency_checker::{DependencyReport, DependencyStatus};
pub use templates::{apply_template, get_builtin_templates, Template, TemplateCategory};
pub use wizard::{WizardState, WizardStep};
