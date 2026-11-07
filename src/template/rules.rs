//! Program rules definition and TOML deserialization.

use std::path::PathBuf;
use serde::{Deserialize, Serialize};

/// Position where the import line should be injected into target config file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ImportPosition {
    #[default]
    Top,
    Bottom,
}

/// Rule definition for an individual target application.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProgramRule {
    /// Identifier or display name of the program (e.g. "kitty", "waybar").
    pub name: String,

    /// Filename of the minijinja template (e.g. "kitty.tmpl").
    pub template: String,

    /// Destination path where the rendered color palette should be saved.
    pub output_path: PathBuf,

    /// Optional target main configuration file to inject `import_line` into.
    pub target_config: Option<PathBuf>,

    /// Optional import directive (e.g. "include colors-termonet.conf" or "@import 'colors.css';").
    pub import_line: Option<String>,

    /// Where to place `import_line` if not already present.
    #[serde(default)]
    pub import_position: ImportPosition,

    /// Shell command to reload the application without restarting it.
    pub reload_cmd: Option<String>,
}
