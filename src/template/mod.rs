//! Templating, rule processing, and safe file writes.

pub mod backup;
pub mod embedded;
pub mod engine;
pub mod rules;

pub use backup::{create_safety_backup, ensure_master_backup, safe_write};
pub use engine::TemplateEngine;
pub use rules::{ImportPosition, ProgramRule};
