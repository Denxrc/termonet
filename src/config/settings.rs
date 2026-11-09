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

impl TuiTheme {
    /// Resolves whether this theme setting effectively corresponds to Dark mode.
    #[must_use]
    pub fn is_dark(&self) -> bool {
        match self {
            Self::Dark => true,
            Self::Light => false,
            Self::System => is_system_dark(),
        }
    }
}

/// Detects if dark mode is preferred in the desktop environment (GNOME/Wayland portal).
#[must_use]
pub fn is_system_dark() -> bool {
    // 1. Check GNOME / Freedesktop gsettings interface
    if let Ok(out) = std::process::Command::new("gsettings")
        .args(["get", "org.gnome.desktop.interface", "color-scheme"])
        .output()
        && out.status.success()
    {
        let s = String::from_utf8_lossy(&out.stdout);
        if s.contains("prefer-light") {
            return false;
        }
        if s.contains("prefer-dark") {
            return true;
        }
    }

    // 2. Check XDG Desktop Portal via dbus-send
    if let Ok(out) = std::process::Command::new("dbus-send")
        .args([
            "--session",
            "--print-reply=literal",
            "--dest=org.freedesktop.portal.Desktop",
            "/org/freedesktop/portal/desktop",
            "org.freedesktop.portal.Settings.Read",
            "string:org.freedesktop.appearance",
            "string:color-scheme",
        ])
        .output()
        && out.status.success()
    {
        let s = String::from_utf8_lossy(&out.stdout);
        // 1: prefer-dark, 2: prefer-light
        if s.contains("uint32 2") {
            return false;
        }
        if s.contains("uint32 1") {
            return true;
        }
    }

    // Default to dark theme for terminal environments
    true
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

/// Current configuration schema version.
pub const CONFIG_SCHEMA_VERSION: u32 = 2;

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
    #[serde(default)]
    pub create_backups: bool,
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
            create_backups: false,
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

        let create_backups = self.create_backups;

        format!(
r#"# ==============================================================================
# Termonet Configuration
# Fast, Dynamic Material You System Theme & Palette Generator for Linux
# Path: ~/.config/termonet/config.toml
# Schema Version: 2
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
# Built-in supported targets:
#   "kitty"     - Kitty terminal (~/.config/kitty/colors-termonet.conf)
#   "waybar"    - Waybar status bar (~/.config/waybar/colors.css)
#   "rofi"      - Rofi app launcher (~/.config/rofi/colors.rasi)
#   "hyprland"  - Hyprland 0.55+ Lua configuration (~/.config/hypr/colors.lua)
#
# Custom application targets can be easily added by creating a rule TOML file in
# ~/.config/termonet/rules/ and a corresponding template in ~/.config/termonet/templates/.
enabled_programs = [
{programs_formatted}]

# [Safety & Backups]
# Create backup copies (.bak, .original) before modifying target configurations and templates.
# When set to false, files are updated directly without littering your config directories with old versions.
create_backups = {create_backups}
"#
        )
    }

    /// Checks whether the raw configuration file on disk needs an automatic migration/sync.
    /// This detects configs from older versions that lack newly added parameters
    /// (such as `create_backups`), lack modern schema version headers, or contain outdated documentation.
    #[must_use]
    pub fn needs_migration(content: &str) -> bool {
        !content.contains("create_backups")
            || !content.contains("Schema Version: 2")
            || content.contains("alacritty")
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
    /// If an older configuration schema is detected, automatically migrates and syncs it in-place
    /// with the latest parameters and documentation while preserving all user customizations.
    pub fn load() -> Result<Option<Self>> {
        let path = AppPaths::config_file()?;
        if !path.exists() {
            return Ok(None);
        }

        let content = std::fs::read_to_string(&path)
            .map_err(|e| TermonetError::Config(format!("Failed to read {}: {}", path.display(), e)))?;

        let config: AppConfig = toml::from_str(&content)
            .map_err(|e| TermonetError::Config(format!("Failed to parse {}: {}", path.display(), e)))?;

        // Automatic config evolution: migrate outdated config files seamlessly
        if Self::needs_migration(&content) {
            let _ = config.save_to_path(&path);
        }

        Ok(Some(config))
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
        let config = AppConfig {
            language: Language::En,
            tui_theme: TuiTheme::Dark,
            font_mode: FontMode::Pixel,
            display_server: DisplayServer::Wayland,
            wallpaper_backend: WallpaperBackend::Swww,
            primary_monitor: Some("DP-1".to_string()),
            manual_wallpaper_path: Some("/home/user/bg.png".to_string()),
            enabled_programs: vec!["kitty".into(), "rofi".into()],
            create_backups: false,
        };

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

        let config = AppConfig {
            language: Language::En,
            tui_theme: TuiTheme::Light,
            font_mode: FontMode::Standard,
            wallpaper_backend: WallpaperBackend::Hyprpaper,
            enabled_programs: vec!["kitty".into(), "waybar".into()],
            ..Default::default()
        };

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
        assert!(!parsed.create_backups);
    }

    #[test]
    fn test_config_migration_on_outdated_schema() {
        let legacy_v1_toml = r#"
# Termonet Configuration
language = "en"
tui_theme = "light"
font_mode = "standard"
display_server = "wayland"
wallpaper_backend = "hyprpaper"
# Available built-in targets:
#   "alacritty" - Alacritty terminal
enabled_programs = [
    "kitty",
    "waybar"
]
"#;
        assert!(AppConfig::needs_migration(legacy_v1_toml));

        let parsed: AppConfig = toml::from_str(legacy_v1_toml).expect("parse legacy config");
        assert_eq!(parsed.language, Language::En);
        assert_eq!(parsed.tui_theme, TuiTheme::Light);
        assert!(!parsed.create_backups);

        let upgraded = parsed.to_commented_toml();
        assert!(upgraded.contains("Schema Version: 2"));
        assert!(upgraded.contains("create_backups = false"));
        assert!(!upgraded.contains("alacritty"));
        assert!(!AppConfig::needs_migration(&upgraded));
    }
}
