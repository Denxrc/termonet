//! Core graphics and color generation modules.

pub mod color;
pub mod image;
pub mod palette;

pub use color::{MaterialScheme, RgbColor, TerminalPalette};
pub use image::load_and_preprocess_image;
pub use palette::GeneratedPalette;
