//! Image loading, animation frame extraction, and in-memory downscaling.

use std::path::Path;
use image::{DynamicImage, GenericImageView, ImageReader};
use crate::error::Result;

/// Target dimension for palette extraction downscaling (128x128).
pub const EXTRACTION_SIZE: u32 = 128;

/// Loads an image file, extracts the first frame if animated, and resizes it to 128x128 in-memory.
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

    let image = reader.decode()?;

    // Resize image in memory to 128x128 using Fast Nearest/Triangle filtering
    // to provide optimal speed and sufficient color distribution for MCU quantization.
    let resized = image.thumbnail_exact(EXTRACTION_SIZE, EXTRACTION_SIZE);
    Ok(resized)
}

/// Converts a `DynamicImage` into raw ARGB/RGBA pixel buffer for quantization.
#[must_use]
pub fn extract_pixels(image: &DynamicImage) -> Vec<[u8; 4]> {
    let (width, height) = image.dimensions();
    let mut pixels = Vec::with_capacity((width * height) as usize);

    for (_, _, pixel) in image.pixels() {
        pixels.push(pixel.0);
    }

    pixels
}
