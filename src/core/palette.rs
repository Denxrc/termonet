//! Palette extraction and Material You scheme generation.

use std::path::Path;
use material_colors::{
    color::Rgb as MaterialRgb,
    dynamic_color::Variant,
    hct::Hct,
    image::extract_color,
    theme::ThemeBuilder,
};
use serde::{Deserialize, Serialize};

use crate::core::color::{MaterialScheme, RgbColor, TerminalPalette};
use crate::core::image::{load_and_preprocess_image, to_material_pixels};
use crate::error::Result;

/// High-level structure containing both the Material You theme and the 16-color ANSI terminal palette.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeneratedPalette {
    /// Dominant seed color extracted from the wallpaper.
    pub seed_color: RgbColor,
    /// Light theme variant of Material You scheme.
    pub light: MaterialScheme,
    /// Dark theme variant of Material You scheme.
    pub dark: MaterialScheme,
    /// Terminal ANSI colors derived from tonal palettes and HCT color space.
    pub terminal: TerminalPalette,
}

impl GeneratedPalette {
    /// Extracts colors and generates a complete Material You palette from an image file.
    ///
    /// # Arguments
    /// * `image_path` - Path to the image file (supports PNG, JPG, GIF, WebP).
    ///
    /// # Errors
    /// Returns `TermonetError` if file reading or decoding fails.
    pub fn from_image_path<P: AsRef<Path>>(image_path: P) -> Result<Self> {
        let image = load_and_preprocess_image(image_path)?;
        let pixels = to_material_pixels(&image);
        Ok(Self::from_pixels(&pixels))
    }

    /// Generates a palette directly from a slice of Material RGB pixels.
    #[must_use]
    pub fn from_pixels(pixels: &[MaterialRgb]) -> Self {
        let seed_rgb = if pixels.is_empty() {
            MaterialRgb::new(0x38, 0x5e, 0x72) // Default balanced fallback slate color
        } else {
            extract_color(pixels)
        };

        Self::from_seed_color(seed_rgb)
    }

    /// Generates light/dark schemes and terminal colors from a given seed color.
    #[must_use]
    pub fn from_seed_color(seed: MaterialRgb) -> Self {
        // Use Variant::Content to preserve natural image fidelity and avoid artificial oversaturation
        let theme = ThemeBuilder::with_source(seed)
            .variant(Variant::Content)
            .build();
        let seed_hct = Hct::new(seed);
        // Subtle chroma clamp (6.0..32.0) to preserve muted/moody tones without color distortion
        let chroma = seed_hct.get_chroma().clamp(6.0, 32.0);

        let dark_scheme = theme.schemes.dark;
        let light_scheme = theme.schemes.light;

        // Perceptually uniform terminal colors generated via HCT space
        let green_normal: MaterialRgb = Hct::from(142.0, chroma, 75.0).into();
        let green_bright: MaterialRgb = Hct::from(142.0, chroma + 4.0, 85.0).into();

        let yellow_normal: MaterialRgb = Hct::from(95.0, chroma, 80.0).into();
        let yellow_bright: MaterialRgb = Hct::from(95.0, chroma + 4.0, 90.0).into();

        let cyan_normal: MaterialRgb = Hct::from(195.0, chroma, 80.0).into();
        let cyan_bright: MaterialRgb = Hct::from(195.0, chroma + 4.0, 90.0).into();

        let terminal = TerminalPalette {
            // Normal 0..7
            color0: dark_scheme.surface_container_lowest.into(),
            color1: dark_scheme.error.into(),
            color2: green_normal.into(),
            color3: yellow_normal.into(),
            color4: dark_scheme.primary.into(),
            color5: dark_scheme.tertiary.into(),
            color6: cyan_normal.into(),
            color7: dark_scheme.on_surface.into(),

            // Bright 8..15
            color8: dark_scheme.outline.into(),
            color9: dark_scheme.error_container.into(),
            color10: green_bright.into(),
            color11: yellow_bright.into(),
            color12: dark_scheme.primary_container.into(),
            color13: dark_scheme.tertiary_container.into(),
            color14: cyan_bright.into(),
            color15: dark_scheme.on_primary_fixed.into(),

            // Special
            background: dark_scheme.background.into(),
            foreground: dark_scheme.on_surface.into(),
            cursor: dark_scheme.primary.into(),
        };

        Self {
            seed_color: seed.into(),
            light: light_scheme.into(),
            dark: dark_scheme.into(),
            terminal,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{Rgb, RgbImage};

    #[test]
    fn test_gif_decode() {
        let p = GeneratedPalette::from_image_path("/tmp/test_anim.gif").expect("gif decode failed");
        println!("EXTRACTED GIF SEED: {:?}", p.seed_color.to_hex());
    }

    #[test]
    fn test_palette_generation_from_seed() {
        let seed = MaterialRgb::new(0x2a, 0x85, 0x55);
        let palette = GeneratedPalette::from_seed_color(seed);

        assert_eq!(palette.seed_color.r, 0x2a);
        assert_eq!(palette.seed_color.g, 0x85);
        assert_eq!(palette.seed_color.b, 0x55);

        // Check HEX formats
        assert!(palette.seed_color.to_hex().starts_with('#'));
        assert_eq!(palette.seed_color.to_hex().len(), 7);
        assert_eq!(palette.seed_color.to_stripped_hex().len(), 6);

        // Verify dark scheme roles exist
        assert_ne!(palette.dark.primary.to_hex(), "");
        assert_ne!(palette.dark.background.to_hex(), "");

        // Verify terminal colors exist
        assert_ne!(palette.terminal.color0.to_hex(), "");
        assert_ne!(palette.terminal.color15.to_hex(), "");
    }

    #[test]
    fn test_palette_generation_from_synthetic_image() {
        let mut img = RgbImage::new(128, 128);
        for pixel in img.pixels_mut() {
            *pixel = Rgb([0x1f, 0x4e, 0x79]);
        }

        let dynamic_img = image::DynamicImage::ImageRgb8(img);
        let pixels = to_material_pixels(&dynamic_img);
        let palette = GeneratedPalette::from_pixels(&pixels);

        assert_eq!(palette.seed_color.to_hex().len(), 7);
        assert_eq!(palette.terminal.foreground.to_hex().len(), 7);
    }
}
