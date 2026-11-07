//! Color structures and conversions for Material You and terminal palettes.

use serde::{Deserialize, Serialize};

/// Represents an sRGB color with 8-bit channels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RgbColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl RgbColor {
    /// Creates a new RGB color.
    #[must_use]
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    /// Formats the color as a standard lowercase HEX string with a leading hash (e.g. "#1a2b3c").
    #[must_use]
    pub fn to_hex(&self) -> String {
        format!("#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
    }

    /// Formats the color as a lowercase HEX string without a leading hash (e.g. "1a2b3c").
    #[must_use]
    pub fn to_stripped_hex(&self) -> String {
        format!("{:02x}{:02x}{:02x}", self.r, self.g, self.b)
    }

    /// Formats the color as a comma-separated RGB string (e.g. "26, 43, 60").
    #[must_use]
    pub fn to_rgb_string(&self) -> String {
        format!("{}, {}, {}", self.r, self.g, self.b)
    }
}

impl From<material_colors::color::Rgb> for RgbColor {
    fn from(c: material_colors::color::Rgb) -> Self {
        Self::new(c.red, c.green, c.blue)
    }
}

/// Represents the standard 16 terminal ANSI colors (color0 through color15).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerminalPalette {
    pub color0: RgbColor,
    pub color1: RgbColor,
    pub color2: RgbColor,
    pub color3: RgbColor,
    pub color4: RgbColor,
    pub color5: RgbColor,
    pub color6: RgbColor,
    pub color7: RgbColor,
    pub color8: RgbColor,
    pub color9: RgbColor,
    pub color10: RgbColor,
    pub color11: RgbColor,
    pub color12: RgbColor,
    pub color13: RgbColor,
    pub color14: RgbColor,
    pub color15: RgbColor,
    pub background: RgbColor,
    pub foreground: RgbColor,
    pub cursor: RgbColor,
}

/// Material You scheme role colors.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MaterialScheme {
    pub primary: RgbColor,
    pub on_primary: RgbColor,
    pub primary_container: RgbColor,
    pub on_primary_container: RgbColor,
    pub secondary: RgbColor,
    pub on_secondary: RgbColor,
    pub secondary_container: RgbColor,
    pub on_secondary_container: RgbColor,
    pub tertiary: RgbColor,
    pub on_tertiary: RgbColor,
    pub error: RgbColor,
    pub on_error: RgbColor,
    pub surface: RgbColor,
    pub on_surface: RgbColor,
    pub surface_variant: RgbColor,
    pub on_surface_variant: RgbColor,
    pub background: RgbColor,
    pub on_background: RgbColor,
    pub outline: RgbColor,
}

impl From<material_colors::scheme::Scheme> for MaterialScheme {
    fn from(s: material_colors::scheme::Scheme) -> Self {
        Self {
            primary: s.primary.into(),
            on_primary: s.on_primary.into(),
            primary_container: s.primary_container.into(),
            on_primary_container: s.on_primary_container.into(),
            secondary: s.secondary.into(),
            on_secondary: s.on_secondary.into(),
            secondary_container: s.secondary_container.into(),
            on_secondary_container: s.on_secondary_container.into(),
            tertiary: s.tertiary.into(),
            on_tertiary: s.on_tertiary.into(),
            error: s.error.into(),
            on_error: s.on_error.into(),
            surface: s.surface.into(),
            on_surface: s.on_surface.into(),
            surface_variant: s.surface_variant.into(),
            on_surface_variant: s.on_surface_variant.into(),
            background: s.background.into(),
            on_background: s.on_background.into(),
            outline: s.outline.into(),
        }
    }
}
