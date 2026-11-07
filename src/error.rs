//! Error types and result definitions for Termonet.

use thiserror::Error;

/// Core error enumeration representing any failure in Termonet operations.
#[derive(Debug, Error)]
pub enum TermonetError {
    /// Standard I/O operations failure (e.g. file read/write, permissions).
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Image processing or decoding error.
    #[error("Image processing error: {0}")]
    Image(#[from] image::ImageError),

    /// Deserialization error when parsing TOML configurations or rules.
    #[error("TOML deserialization error: {0}")]
    TomlDe(#[from] toml::de::Error),

    /// Serialization error when generating TOML configurations.
    #[error("TOML serialization error: {0}")]
    TomlSer(#[from] toml::ser::Error),

    /// Template parsing or rendering error.
    #[error("Template engine error: {0}")]
    Template(String),

    /// Wallpaper detection failure across display servers or desktop environments.
    #[error("Wallpaper detection error: {0}")]
    WallpaperNotFound(String),

    /// Configuration validation or parsing error.
    #[error("Configuration error: {0}")]
    Config(String),

    /// Command execution failure when running reload hooks.
    #[error("Hook execution error: {0}")]
    HookExecution(String),
}

/// Global Result type alias using `TermonetError`.
pub type Result<T> = std::result::Result<T, TermonetError>;
