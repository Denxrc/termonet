//! Templating, rule processing, and safe file writes.

pub mod backup;
pub mod engine;
pub mod rules;

pub use backup::{create_safety_backup, ensure_master_backup, safe_write};
pub use engine::TemplateEngine;
pub use rules::{
    CommentStyle, ImportPosition, ProgramRule, extract_template_version, inject_import,
};
