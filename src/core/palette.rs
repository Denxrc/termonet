//! Palette extraction and Material You scheme generation.

use std::path::Path;
use material_colors::{
    blend::harmonize,
    color::Rgb as MaterialRgb,
    dynamic_color::Variant,
    hct::Hct,
    quantize::{Quantizer, QuantizerCelebi},
    score::Score,
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
    /// Terminal ANSI colors derived from tonal palettes and HCT color space (Dark theme).
    pub terminal: TerminalPalette,
    /// Terminal ANSI colors derived for Light theme.
    #[serde(default)]
    pub terminal_light: TerminalPalette,
}

impl GeneratedPalette {
    /// Returns the active MaterialScheme based on dark vs light preference.
    #[must_use]
    pub fn scheme_for(&self, is_dark: bool) -> &MaterialScheme {
        if is_dark {
            &self.dark
        } else {
            &self.light
        }
    }

    /// Returns the active TerminalPalette based on dark vs light preference.
    #[must_use]
    pub fn terminal_for(&self, is_dark: bool) -> &TerminalPalette {
        if is_dark {
            &self.terminal
        } else {
            &self.terminal_light
        }
    }
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
        let seed_rgb = Self::extract_best_seed(pixels);
        Self::from_seed_color(seed_rgb)
    }

    /// Extracts the most representative and vibrant seed color from an image.
    ///
    /// In Google MCU, `Score::score` ranks colors with 70% weight on pixel population.
    /// This causes dark or muted backgrounds (like clouds, dark asphalt, or shadows)
    /// to occasionally rank above the true vibrant theme accent (e.g. sunsets, neon).
    /// This method prioritizes top-ranked candidates with vibrant chroma (>= 25.0),
    /// gracefully falling back to the top candidate for low-contrast/monochrome images.
    #[must_use]
    pub fn extract_best_seed(pixels: &[MaterialRgb]) -> MaterialRgb {
        if pixels.is_empty() {
            return MaterialRgb::new(0x3d, 0x66, 0x47); // Default fallback slate color
        }

        let result = QuantizerCelebi::quantize(pixels, 128);
        let ranked = Score::score(&result.color_to_count, Some(5), None, Some(true));

        if ranked.is_empty() {
            return MaterialRgb::new(0x3d, 0x66, 0x47);
        }

        // Search in rank order: if #0 has sufficient chroma (>= 25.0), it is immediately returned.
        // Otherwise, if a vibrant accent is ranked among the top candidates, prefer it.
        ranked
            .iter()
            .find(|&&c| {
                let hct: Hct = c.into();
                hct.get_chroma() >= 25.0
            })
            .copied()
            .unwrap_or(ranked[0])
    }

    /// Generates light/dark schemes and harmonized terminal colors from a given seed color.
    #[must_use]
    pub fn from_seed_color(seed: MaterialRgb) -> Self {
        // Use Variant::Content to preserve natural image fidelity and avoid artificial oversaturation
        let theme = ThemeBuilder::with_source(seed)
            .variant(Variant::Content)
            .build();
        let seed_hct = Hct::new(seed);
        // Tasteful chroma clamp (16.0..36.0) to match the atmospheric saturation of the wallpaper
        let chroma = seed_hct.get_chroma().clamp(16.0, 36.0);

        let dark_scheme = theme.schemes.dark;
        let light_scheme = theme.schemes.light;

        // Base semantic ANSI hues (Red, Green, Yellow, Blue, Magenta, Cyan)
        let base_hues = [
            25.0,  // Red
            140.0, // Green
            85.0,  // Yellow
            250.0, // Blue
            310.0, // Magenta
            195.0, // Cyan
        ];

        // Harmonize each semantic hue towards the wallpaper seed color via Google MCU
        let mut harm_hues = [0.0f64; 6];
        for (i, &base_hue) in base_hues.iter().enumerate() {
            let base_rgb: MaterialRgb = Hct::from(base_hue, 40.0, 50.0).into();
            let harmonized_rgb = harmonize(base_rgb, seed);
            let harmonized_hct = Hct::new(harmonized_rgb);
            harm_hues[i] = harmonized_hct.get_hue();
        }

        let normal_tone = 72.0;
        let bright_tone = 86.0;
        let bright_chroma = chroma + 6.0;

        let red_normal: MaterialRgb = Hct::from(harm_hues[0], chroma, normal_tone).into();
        let red_bright: MaterialRgb = Hct::from(harm_hues[0], bright_chroma, bright_tone).into();

        let green_normal: MaterialRgb = Hct::from(harm_hues[1], chroma, normal_tone).into();
        let green_bright: MaterialRgb = Hct::from(harm_hues[1], bright_chroma, bright_tone).into();

        let yellow_normal: MaterialRgb = Hct::from(harm_hues[2], chroma, normal_tone).into();
        let yellow_bright: MaterialRgb = Hct::from(harm_hues[2], bright_chroma, bright_tone).into();

        let blue_normal: MaterialRgb = Hct::from(harm_hues[3], chroma, normal_tone).into();
        let blue_bright: MaterialRgb = Hct::from(harm_hues[3], bright_chroma, bright_tone).into();

        let magenta_normal: MaterialRgb = Hct::from(harm_hues[4], chroma, normal_tone).into();
        let magenta_bright: MaterialRgb = Hct::from(harm_hues[4], bright_chroma, bright_tone).into();

        let cyan_normal: MaterialRgb = Hct::from(harm_hues[5], chroma, normal_tone).into();
        let cyan_bright: MaterialRgb = Hct::from(harm_hues[5], bright_chroma, bright_tone).into();

        let muted_black: MaterialRgb = Hct::from(seed_hct.get_hue(), chroma * 0.3, 45.0).into();
        let bright_white: MaterialRgb = Hct::from(seed_hct.get_hue(), chroma * 0.1, 95.0).into();

        let terminal = TerminalPalette {
            // Normal 0..7
            color0: dark_scheme.surface_container_lowest.into(),
            color1: red_normal.into(),
            color2: green_normal.into(),
            color3: yellow_normal.into(),
            color4: blue_normal.into(),
            color5: magenta_normal.into(),
            color6: cyan_normal.into(),
            color7: dark_scheme.on_surface.into(),

            // Bright 8..15
            color8: muted_black.into(),
            color9: red_bright.into(),
            color10: green_bright.into(),
            color11: yellow_bright.into(),
            color12: blue_bright.into(),
            color13: magenta_bright.into(),
            color14: cyan_bright.into(),
            color15: bright_white.into(),

            // Special
            background: dark_scheme.background.into(),
            foreground: dark_scheme.on_surface.into(),
            cursor: dark_scheme.primary.into(),
        };

        // --- Light Terminal ANSI Colors ---
        let normal_tone_light = 38.0;
        let bright_tone_light = 52.0;

        let red_normal_light: MaterialRgb = Hct::from(harm_hues[0], chroma + 4.0, normal_tone_light).into();
        let red_bright_light: MaterialRgb = Hct::from(harm_hues[0], bright_chroma, bright_tone_light).into();

        let green_normal_light: MaterialRgb = Hct::from(harm_hues[1], chroma + 4.0, normal_tone_light).into();
        let green_bright_light: MaterialRgb = Hct::from(harm_hues[1], bright_chroma, bright_tone_light).into();

        let yellow_normal_light: MaterialRgb = Hct::from(harm_hues[2], chroma + 4.0, normal_tone_light).into();
        let yellow_bright_light: MaterialRgb = Hct::from(harm_hues[2], bright_chroma, bright_tone_light).into();

        let blue_normal_light: MaterialRgb = Hct::from(harm_hues[3], chroma + 4.0, normal_tone_light).into();
        let blue_bright_light: MaterialRgb = Hct::from(harm_hues[3], bright_chroma, bright_tone_light).into();

        let magenta_normal_light: MaterialRgb = Hct::from(harm_hues[4], chroma + 4.0, normal_tone_light).into();
        let magenta_bright_light: MaterialRgb = Hct::from(harm_hues[4], bright_chroma, bright_tone_light).into();

        let cyan_normal_light: MaterialRgb = Hct::from(harm_hues[5], chroma + 4.0, normal_tone_light).into();
        let cyan_bright_light: MaterialRgb = Hct::from(harm_hues[5], bright_chroma, bright_tone_light).into();

        let dark_gray_light: MaterialRgb = Hct::from(seed_hct.get_hue(), chroma * 0.2, 20.0).into();
        let soft_gray_light: MaterialRgb = Hct::from(seed_hct.get_hue(), chroma * 0.2, 50.0).into();
        let light_gray_light: MaterialRgb = Hct::from(seed_hct.get_hue(), chroma * 0.1, 85.0).into();
        let pure_white_light: MaterialRgb = Hct::from(seed_hct.get_hue(), chroma * 0.05, 98.0).into();

        let terminal_light = TerminalPalette {
            // Normal 0..7
            color0: dark_gray_light.into(),
            color1: red_normal_light.into(),
            color2: green_normal_light.into(),
            color3: yellow_normal_light.into(),
            color4: blue_normal_light.into(),
            color5: magenta_normal_light.into(),
            color6: cyan_normal_light.into(),
            color7: light_gray_light.into(),

            // Bright 8..15
            color8: soft_gray_light.into(),
            color9: red_bright_light.into(),
            color10: green_bright_light.into(),
            color11: yellow_bright_light.into(),
            color12: blue_bright_light.into(),
            color13: magenta_bright_light.into(),
            color14: cyan_bright_light.into(),
            color15: pure_white_light.into(),

            // Special
            background: light_scheme.background.into(),
            foreground: light_scheme.on_surface.into(),
            cursor: light_scheme.primary.into(),
        };

        Self {
            seed_color: seed.into(),
            light: light_scheme.into(),
            dark: dark_scheme.into(),
            terminal,
            terminal_light,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{Rgb, RgbImage};

    #[test]
    fn test_gif_decode() {
        let mut gif_data = Vec::new();
        {
            let mut encoder = image::codecs::gif::GifEncoder::new(&mut gif_data);
            let frame_img = image::RgbaImage::from_pixel(16, 16, image::Rgba([0x2a, 0x85, 0x55, 0xff]));
            let frame = image::Frame::new(frame_img);
            encoder.encode_frame(frame).expect("encode frame");
        }
        let temp_path = std::env::temp_dir().join("termonet_test_synthetic.gif");
        std::fs::write(&temp_path, &gif_data).expect("write temp gif");

        let p = GeneratedPalette::from_image_path(&temp_path).expect("gif decode failed");
        assert_eq!(p.seed_color.to_hex().len(), 7);
        let _ = std::fs::remove_file(&temp_path);
    }

    #[test]
    fn test_vibrant_seed_extraction() {
        // High population of muted background, small cluster of vibrant accent
        let mut pixels = vec![MaterialRgb::new(50, 50, 50); 900];
        pixels.extend(vec![MaterialRgb::new(255, 30, 30); 100]);

        let seed = GeneratedPalette::extract_best_seed(&pixels);
        let hct: Hct = seed.into();
        assert!(hct.get_chroma() >= 25.0);
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

        // Verify light terminal palette contrast and properties
        assert_ne!(palette.terminal_light.color0.to_hex(), "");
        assert_ne!(palette.terminal_light.background.to_hex(), "");
        assert_ne!(palette.terminal.background.to_hex(), palette.terminal_light.background.to_hex());

        // Verify scheme_for and terminal_for
        assert_eq!(palette.scheme_for(true).primary, palette.dark.primary);
        assert_eq!(palette.scheme_for(false).primary, palette.light.primary);
        assert_eq!(palette.terminal_for(true).background, palette.terminal.background);
        assert_eq!(palette.terminal_for(false).background, palette.terminal_light.background);
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
