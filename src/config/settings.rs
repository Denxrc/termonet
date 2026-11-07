//! Application configuration schemas and persistence.

use serde::{Deserialize, Serialize};

/// Supported interface languages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    #[default]
    Ru,
    En,
}

/// TUI visual theme variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum TuiTheme {
    #[default]
    Dark,
    Light,
    System,
}

/// Header font rendering mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum FontMode {
    /// 5px bitmap pixel font rendered with half-blocks.
    #[default]
    Pixel,
    /// Standard single-line terminal text.
    Standard,
}

/// Display server type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum DisplayServer {
    #[default]
    Wayland,
    X11,
}

/// Known wallpaper backends.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum WallpaperBackend {
    #[default]
    Auto,
    Swww,
    Awww,
    Hyprpaper,
    Wpaperd,
    Swaybg,
    Gnome,
    Kde,
    Feh,
    Nitrogen,
    Manual(String),
}

/// Global Termonet configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppConfig {
    pub language: Language,
    pub tui_theme: TuiTheme,
    pub font_mode: FontMode,
    pub display_server: DisplayServer,
    pub wallpaper_backend: WallpaperBackend,
    pub primary_monitor: Option<String>,
    pub enabled_programs: Vec<String>,
    pub manual_wallpaper_path: Option<String>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            language: Language::Ru,
            tui_theme: TuiTheme::Dark,
            font_mode: FontMode::Pixel,
            display_server: DisplayServer::Wayland,
            wallpaper_backend: WallpaperBackend::Auto,
            primary_monitor: None,
            enabled_programs: Vec::new(),
            manual_wallpaper_path: None,
        }
    }
}
