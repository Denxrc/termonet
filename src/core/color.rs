//! Color structures and conversions for Material You and terminal palettes.

use serde::{Deserialize, Serialize};

const HEX_DIGITS: &[u8; 16] = b"0123456789abcdef";

/// Represents an sRGB color with 8-bit channels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
pub struct RgbColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Serialize for RgbColor {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("RgbColor", 6)?;
        state.serialize_field("r", &self.r)?;
        state.serialize_field("g", &self.g)?;
        state.serialize_field("b", &self.b)?;

        // Zero-allocation HEX string slice
        let hex_bytes = [
            b'#',
            HEX_DIGITS[(self.r >> 4) as usize],
            HEX_DIGITS[(self.r & 0xf) as usize],
            HEX_DIGITS[(self.g >> 4) as usize],
            HEX_DIGITS[(self.g & 0xf) as usize],
            HEX_DIGITS[(self.b >> 4) as usize],
            HEX_DIGITS[(self.b & 0xf) as usize],
        ];
        let hex_str = std::str::from_utf8(&hex_bytes).unwrap_or("");
        state.serialize_field("hex", hex_str)?;

        // Zero-allocation stripped HEX string slice
        let strip_bytes = [
            HEX_DIGITS[(self.r >> 4) as usize],
            HEX_DIGITS[(self.r & 0xf) as usize],
            HEX_DIGITS[(self.g >> 4) as usize],
            HEX_DIGITS[(self.g & 0xf) as usize],
            HEX_DIGITS[(self.b >> 4) as usize],
            HEX_DIGITS[(self.b & 0xf) as usize],
        ];
        let strip_str = std::str::from_utf8(&strip_bytes).unwrap_or("");
        state.serialize_field("strip", strip_str)?;

        // Zero-allocation RGB comma-separated string slice via stack cursor
        use std::io::Write;
        let mut rgb_buf = [0u8; 16];
        let mut cursor = std::io::Cursor::new(&mut rgb_buf[..]);
        let _ = write!(cursor, "{}, {}, {}", self.r, self.g, self.b);
        let len = cursor.position() as usize;
        let rgb_str = std::str::from_utf8(&rgb_buf[..len]).unwrap_or("");
        state.serialize_field("rgb", rgb_str)?;

        state.end()
    }
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
        let bytes = [
            b'#',
            HEX_DIGITS[(self.r >> 4) as usize],
            HEX_DIGITS[(self.r & 0xf) as usize],
            HEX_DIGITS[(self.g >> 4) as usize],
            HEX_DIGITS[(self.g & 0xf) as usize],
            HEX_DIGITS[(self.b >> 4) as usize],
            HEX_DIGITS[(self.b & 0xf) as usize],
        ];
        String::from_utf8(bytes.to_vec()).unwrap_or_default()
    }

    /// Formats the color as a lowercase HEX string without a leading hash (e.g. "1a2b3c").
    #[must_use]
    pub fn to_stripped_hex(&self) -> String {
        let bytes = [
            HEX_DIGITS[(self.r >> 4) as usize],
            HEX_DIGITS[(self.r & 0xf) as usize],
            HEX_DIGITS[(self.g >> 4) as usize],
            HEX_DIGITS[(self.g & 0xf) as usize],
            HEX_DIGITS[(self.b >> 4) as usize],
            HEX_DIGITS[(self.b & 0xf) as usize],
        ];
        String::from_utf8(bytes.to_vec()).unwrap_or_default()
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

impl From<RgbColor> for ratatui::style::Color {
    fn from(c: RgbColor) -> Self {
        ratatui::style::Color::Rgb(c.r, c.g, c.b)
    }
}

/// Represents the standard 16 terminal ANSI colors (color0 through color15).
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rgb_color_formatting_and_lut() {
        let color = RgbColor::new(0x1a, 0x2b, 0x3c);
        assert_eq!(color.to_hex(), "#1a2b3c");
        assert_eq!(color.to_stripped_hex(), "1a2b3c");
        assert_eq!(color.to_rgb_string(), "26, 43, 60");

        let black = RgbColor::new(0, 0, 0);
        assert_eq!(black.to_hex(), "#000000");
        assert_eq!(black.to_stripped_hex(), "000000");
        assert_eq!(black.to_rgb_string(), "0, 0, 0");

        let white = RgbColor::new(255, 255, 255);
        assert_eq!(white.to_hex(), "#ffffff");
        assert_eq!(white.to_stripped_hex(), "ffffff");
        assert_eq!(white.to_rgb_string(), "255, 255, 255");
    }

    #[test]
    fn test_rgb_color_serialization_roundtrip() {
        let color = RgbColor::new(0x21, 0x96, 0xf3);
        let serialized = toml::to_string(&color).expect("serialize color");
        assert!(serialized.contains("hex = \"#2196f3\""));
        assert!(serialized.contains("strip = \"2196f3\""));
        assert!(serialized.contains("rgb = \"33, 150, 243\""));
        assert!(serialized.contains("r = 33"));
        assert!(serialized.contains("g = 150"));
        assert!(serialized.contains("b = 243"));
    }
}

