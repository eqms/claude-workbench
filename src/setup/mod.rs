//! Setup module for installation wizard and dependency checking

pub mod dependency_checker;
pub mod wizard;

pub use dependency_checker::{DependencyReport, DependencyStatus};
pub use wizard::{WizardState, WizardStep};
