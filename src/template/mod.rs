//! Templating, rule processing, and safe file writes.

pub mod backup;
pub mod embedded;
pub mod engine;
pub mod rules;

pub use backup::{create_safety_backup, ensure_master_backup, safe_write};
pub use embedded::{sync_default_assets, unpack_default_assets, DEFAULT_RULES, DEFAULT_TEMPLATES};
pub use engine::TemplateEngine;
pub use rules::{
    extract_template_version, inject_import, CommentStyle, ImportPosition, ProgramRule,
};
