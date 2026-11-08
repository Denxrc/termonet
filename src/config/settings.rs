//! Application configuration schemas and persistence.

use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use crate::config::AppPaths;
use crate::error::{Result, TermonetError};

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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
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
    Manual,
}

/// Global Termonet configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppConfig {
    #[serde(default)]
    pub language: Language,
    #[serde(default)]
    pub tui_theme: TuiTheme,
    #[serde(default)]
    pub font_mode: FontMode,
    #[serde(default)]
    pub display_server: DisplayServer,
    #[serde(default)]
    pub wallpaper_backend: WallpaperBackend,
    #[serde(default)]
    pub primary_monitor: Option<String>,
    #[serde(default)]
    pub enabled_programs: Vec<String>,
    #[serde(default)]
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
            enabled_programs: vec![
                "kitty".to_string(),
                "waybar".to_string(),
                "rofi".to_string(),
                "hyprland".to_string(),
            ],
            manual_wallpaper_path: None,
        }
    }
}

impl AppConfig {
    /// Checks whether the configuration file exists on disk.
    pub fn config_exists() -> Result<bool> {
        let path = AppPaths::config_file()?;
        Ok(path.exists())
    }

    /// Generates a well-documented, commented TOML configuration string.
    #[must_use]
    pub fn to_commented_toml(&self) -> String {
        let lang = match self.language {
            Language::Ru => "ru",
            Language::En => "en",
        };
        let theme = match self.tui_theme {
            TuiTheme::Dark => "dark",
            TuiTheme::Light => "light",
            TuiTheme::System => "system",
        };
        let font = match self.font_mode {
            FontMode::Pixel => "pixel",
            FontMode::Standard => "standard",
        };
        let server = match self.display_server {
            DisplayServer::Wayland => "wayland",
            DisplayServer::X11 => "x11",
        };
        let backend = match self.wallpaper_backend {
            WallpaperBackend::Auto => "auto",
            WallpaperBackend::Swww => "swww",
            WallpaperBackend::Hyprpaper => "hyprpaper",
            WallpaperBackend::Awww => "awww",
            WallpaperBackend::Wpaperd => "wpaperd",
            WallpaperBackend::Swaybg => "swaybg",
            WallpaperBackend::Gnome => "gnome",
            WallpaperBackend::Kde => "kde",
            WallpaperBackend::Feh => "feh",
            WallpaperBackend::Nitrogen => "nitrogen",
            WallpaperBackend::Manual => "manual",
        };

        let primary_mon_line = match &self.primary_monitor {
            Some(mon) => format!("primary_monitor = \"{}\"", mon),
            None => "# primary_monitor = \"eDP-1\"".to_string(),
        };

        let manual_path_line = match &self.manual_wallpaper_path {
            Some(p) => format!("manual_wallpaper_path = \"{}\"", p),
            None => "# manual_wallpaper_path = \"/path/to/wallpaper.png\"".to_string(),
        };

        let programs_formatted = if self.enabled_programs.is_empty() {
            String::new()
        } else {
            let items: Vec<String> = self
                .enabled_programs
                .iter()
                .map(|p| format!("    \"{}\"", p))
                .collect();
            format!("{}\n", items.join(",\n"))
        };

        format!(
r#"# ==============================================================================
# Termonet Configuration
# Fast, Dynamic Material You System Theme & Palette Generator for Linux
# Path: ~/.config/termonet/config.toml
# ==============================================================================

# [General & Interface]
# Interface language.
# Allowed options:
#   "ru" - Russian (Русский)
#   "en" - English
language = "{lang}"

# [TUI Appearance]
# Visual color theme variant for the TUI interface.
# Allowed options:
#   "dark"   - Dark theme (recommended for terminal emulators)
#   "light"  - Light theme
#   "system" - Follow terminal background luminance
tui_theme = "{theme}"

# Header logo rendering mode.
# Allowed options:
#   "pixel"    - 5px bitmap pixel font rendered via Unicode half-blocks (█, ▀, ▄)
#   "standard" - Compact single-line text header for smaller terminal windows
font_mode = "{font}"

# [Desktop Environment]
# Display server protocol running on your system.
# Allowed options:
#   "wayland" - Modern Wayland compositors (Hyprland, Sway, wlroots, GNOME Wayland)
#   "x11"     - Classic X11 window managers (i3, bspwm, XFCE, classic desktop)
display_server = "{server}"

# [Wallpaper Engine]
# Active wallpaper backend engine used to detect the current wallpaper.
# Allowed options:
#   "auto"      - Automatic detection based on running desktop / compositor
#   "awww"      - Modern Wayland animated wallpaper daemon (successor to swww)
#   "swww"      - Wayland animated wallpaper daemon (compatible with awww)
#   "hyprpaper" - Hyprland native IPC wallpaper utility
#   "wpaperd"   - Wayland wallpaper daemon via symlinks
#   "swaybg"    - Sway / wlroots wallpaper background process
#   "gnome"     - GNOME desktop (gsettings org.gnome.desktop.background)
#   "kde"       - KDE Plasma desktop (plasma-org.kde.plasma.desktop-appletsrc)
#   "feh"       - X11 ~/.fehbg script parsing
#   "nitrogen"  - X11 ~/.config/nitrogen/bg-saved.cfg parsing
#   "manual"    - Explicit file path (uses manual_wallpaper_path below)
wallpaper_backend = "{backend}"

# Primary monitor name.
# Used when different monitors display different wallpapers to select the master screen.
# Uncomment to specify a dedicated display (e.g. "eDP-1", "DP-1", "HDMI-A-1").
{primary_mon_line}

# Manual path to a wallpaper image file.
# Used when wallpaper_backend = "manual", or as fallback when auto-detection is unavailable.
# Supports PNG, JPG, GIF (first frame extracted), and WebP.
{manual_path_line}

# [Target Applications]
# List of applications for which Termonet will generate Material You themes.
# Available built-in targets:
#   "kitty"     - Kitty terminal (~/.config/kitty/colors-termonet.conf)
#   "waybar"    - Waybar status bar (~/.config/waybar/colors.css)
#   "rofi"      - Rofi app launcher (~/.config/rofi/colors.rasi)
#   "hyprland"  - Hyprland 0.55+ Lua configuration (~/.config/hypr/colors.lua)
#   "alacritty" - Alacritty terminal (~/.config/alacritty/colors.toml)
#   "dunst"     - Dunst notification daemon (~/.config/dunst/dunstrc.d/colors.conf)
#   "sway"      - Sway window manager (~/.config/sway/colors.conf)
enabled_programs = [
{programs_formatted}]
"#
        )
    }

    /// Loads configuration from an explicit file path.
    pub fn load_from_path(path: &std::path::Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| TermonetError::Config(format!("Failed to read {}: {}", path.display(), e)))?;

        let config: AppConfig = toml::from_str(&content)
            .map_err(|e| TermonetError::Config(format!("Failed to parse {}: {}", path.display(), e)))?;

        Ok(config)
    }

    /// Saves configuration to an explicit file path, creating parent directories if needed.
    pub fn save_to_path(&self, path: &std::path::Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| TermonetError::Config(format!("Failed to create directory {}: {}", parent.display(), e)))?;
        }

        let toml_string = self.to_commented_toml();

        std::fs::write(path, toml_string)
            .map_err(|e| TermonetError::Config(format!("Failed to write {}: {}", path.display(), e)))?;

        Ok(())
    }

    /// Loads the configuration from standard XDG path (`~/.config/termonet/config.toml`).
    /// Returns `Ok(None)` if the configuration file does not exist yet.
    pub fn load() -> Result<Option<Self>> {
        let path = AppPaths::config_file()?;
        if !path.exists() {
            return Ok(None);
        }
        Ok(Some(Self::load_from_path(&path)?))
    }

    /// Saves the configuration to standard XDG path (`~/.config/termonet/config.toml`).
    /// Automatically creates parent directories (`templates/`, `rules/`) if missing.
    pub fn save(&self) -> Result<PathBuf> {
        let path = AppPaths::config_file()?;

        // Initialize templates and rules subdirectories
        if let Ok(tpl_dir) = AppPaths::templates_dir() {
            let _ = std::fs::create_dir_all(&tpl_dir);
        }
        if let Ok(rules_dir) = AppPaths::rules_dir() {
            let _ = std::fs::create_dir_all(&rules_dir);
        }

        self.save_to_path(&path)?;
        Ok(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_serialization_roundtrip() {
        let config = AppConfig::default();
        let serialized = toml::to_string_pretty(&config).expect("Serialize to TOML");
        let parsed: AppConfig = toml::from_str(&serialized).expect("Deserialize from TOML");
        assert_eq!(config, parsed);
    }

    #[test]
    fn test_commented_toml_roundtrip() {
        let mut config = AppConfig::default();
        config.language = Language::En;
        config.tui_theme = TuiTheme::Dark;
        config.font_mode = FontMode::Pixel;
        config.display_server = DisplayServer::Wayland;
        config.wallpaper_backend = WallpaperBackend::Swww;
        config.primary_monitor = Some("DP-1".to_string());
        config.manual_wallpaper_path = Some("/home/user/bg.png".to_string());
        config.enabled_programs = vec!["kitty".into(), "rofi".into()];

        let commented = config.to_commented_toml();
        assert!(commented.contains("# [General & Interface]"));
        assert!(commented.contains("# [Wallpaper Engine]"));
        assert!(commented.contains("primary_monitor = \"DP-1\""));

        let parsed: AppConfig = toml::from_str(&commented).expect("Deserialize commented TOML");
        assert_eq!(config, parsed);
    }

    #[test]
    fn test_config_save_and_load_from_path() {
        let temp_dir = std::env::temp_dir().join("termonet_test_cfg");
        let test_file = temp_dir.join("sub").join("test_config.toml");

        let mut config = AppConfig::default();
        config.language = Language::En;
        config.tui_theme = TuiTheme::Light;
        config.font_mode = FontMode::Standard;
        config.wallpaper_backend = WallpaperBackend::Hyprpaper;
        config.enabled_programs = vec!["kitty".into(), "waybar".into()];

        config.save_to_path(&test_file).expect("Save to test path");
        assert!(test_file.exists());

        let loaded = AppConfig::load_from_path(&test_file).expect("Load from test path");
        assert_eq!(loaded, config);

        // Cleanup
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_partial_config_fallback() {
        let minimal_toml = r#"
            language = "en"
            tui_theme = "light"
        "#;
        let parsed: AppConfig = toml::from_str(minimal_toml).expect("Deserialize partial");
        assert_eq!(parsed.language, Language::En);
        assert_eq!(parsed.tui_theme, TuiTheme::Light);
        assert_eq!(parsed.font_mode, FontMode::Pixel);
        assert_eq!(parsed.display_server, DisplayServer::Wayland);
    }
}
