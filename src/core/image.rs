//! Image loading, animation frame extraction, and in-memory downscaling.

use std::path::Path;
use image::{DynamicImage, ImageReader};
use crate::error::Result;

/// Target dimension for palette extraction downscaling (128x128).
pub const EXTRACTION_SIZE: u32 = 128;

/// Loads an image file, extracts the first frame if animated (GIF, WebP),
/// and resizes it to 128x128 in-memory for fast and accurate color quantization.
///
/// # Arguments
/// * `path` - Filesystem path to the wallpaper image.
///
/// # Errors
/// Returns `TermonetError::Image` or `TermonetError::Io` on reading or decoding failure.
pub fn load_and_preprocess_image<P: AsRef<Path>>(path: P) -> Result<DynamicImage> {
    let path_ref = path.as_ref();
    let reader = ImageReader::open(path_ref)?
        .with_guessed_format()?;

    // Decoding via ImageReader extracts the first frame for GIF / WebP animations
    let image = reader.decode()?;

    // Resize image in memory to 128x128 using thumbnail_exact for maximum performance
    // and optimal color distribution during MCU quantization.
    let resized = image.thumbnail_exact(EXTRACTION_SIZE, EXTRACTION_SIZE);
    Ok(resized)
}

/// Converts a `DynamicImage` into a collection of `material_colors::color::Rgb` pixels.
#[must_use]
pub fn to_material_pixels(image: &DynamicImage) -> Vec<material_colors::color::Rgb> {
    let rgb_image = image.to_rgb8();
    rgb_image
        .pixels()
        .map(|p| material_colors::color::Rgb::new(p[0], p[1], p[2]))
        .collect()
}
