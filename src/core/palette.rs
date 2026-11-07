//! Palette extraction and Material You scheme generation.

use crate::core::color::{MaterialScheme, RgbColor, TerminalPalette};
use serde::{Deserialize, Serialize};

/// High-level structure containing both the Material You theme and the 16-color ANSI terminal palette.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeneratedPalette {
    /// Dominant seed color extracted from the wallpaper.
    pub seed_color: RgbColor,
    /// Light theme variant of Material You scheme.
    pub light: MaterialScheme,
    /// Dark theme variant of Material You scheme.
    pub dark: MaterialScheme,
    /// Terminal ANSI colors derived from tonal palettes.
    pub terminal: TerminalPalette,
}
