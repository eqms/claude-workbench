//! Setup module for installation wizard, dependency checking, and templates

pub mod dependency_checker;
pub mod templates;
pub mod wizard;

pub use dependency_checker::{DependencyReport, DependencyStatus};
pub use templates::{Template, TemplateCategory, get_builtin_templates, apply_template};
pub use wizard::{WizardState, WizardStep};
