//! Wallpaper query implementations for Desktop Environments (GNOME, KDE Plasma).

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::config::settings::is_system_dark;
use crate::error::{Result, TermonetError};
use crate::wallpaper::monitor::MonitorWallpapers;
use crate::wallpaper::path::sanitize_wallpaper_path;

pub use crate::wallpaper::path::percent_decode;

/// Parses and cleans a GNOME or generic desktop wallpaper URI string.
/// Handles stripping quotes, `file://` scheme, and percent decoding.
#[must_use]
pub fn parse_gnome_uri(raw: &str) -> Option<PathBuf> {
    sanitize_wallpaper_path(raw)
}

/// Queries active wallpaper in GNOME desktop via `gsettings`.
pub fn query_gnome() -> Result<MonitorWallpapers> {
    // 0. Check if wallpaper rendering is completely disabled (picture-options == 'none')
    if let Ok(out) = Command::new("gsettings")
        .args(["get", "org.gnome.desktop.background", "picture-options"])
        .output()
        && out.status.success()
    {
        let opt = String::from_utf8_lossy(&out.stdout);
        let trimmed_opt = opt.trim().trim_matches(|c| c == '\'' || c == '"');
        if trimmed_opt == "none" {
            return Err(TermonetError::WallpaperNotFound(
                "GNOME wallpaper rendering is disabled (picture-options is 'none')".into(),
            ));
        }
    }

    // 1. Detect light vs dark preference
    let scheme_output = Command::new("gsettings")
        .args(["get", "org.gnome.desktop.interface", "color-scheme"])
        .output();

    let is_dark = match scheme_output {
        Ok(out) if out.status.success() => {
            String::from_utf8_lossy(&out.stdout).contains("prefer-dark")
        }
        _ => false,
    };

    let (primary_key, secondary_key) = if is_dark {
        ("picture-uri-dark", "picture-uri")
    } else {
        ("picture-uri", "picture-uri-dark")
    };

    // 2. Query primary key
    if let Some(path) = query_gsettings_key(primary_key)
        && path.exists()
    {
        let mut outputs = HashMap::new();
        outputs.insert("default".to_string(), path);
        return Ok(MonitorWallpapers { outputs });
    }

    // 3. Fallback to secondary key
    if let Some(path) = query_gsettings_key(secondary_key)
        && path.exists()
    {
        let mut outputs = HashMap::new();
        outputs.insert("default".to_string(), path);
        return Ok(MonitorWallpapers { outputs });
    }

    Err(TermonetError::WallpaperNotFound(
        "No active GNOME wallpaper found via gsettings or wallpaper file does not exist".into(),
    ))
}

fn query_gsettings_key(key: &str) -> Option<PathBuf> {
    let output = Command::new("gsettings")
        .args(["get", "org.gnome.desktop.background", key])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_gnome_uri(&stdout)
}

/// Parses KDE Plasma desktop containment configurations from `plasma-org.kde.plasma.desktop-appletsrc`.
///
/// Rules:
/// - Only inspects containments where `plugin=org.kde.desktopcontainment`.
/// - Filters out disabled screens (`lastScreen=-1`, keeps `lastScreen >= 0`).
/// - Extracts `Image=` under `[Containments][ID][Wallpaper]...` or `General`.
/// - Explicitly aliases `lastScreen=0` to `"default"`.
#[must_use]
pub fn parse_kde_containments(content: &str) -> HashMap<String, PathBuf> {
    #[derive(Default)]
    struct ContainmentEntry {
        last_screen: Option<i32>,
        plugin: Option<String>,
        image: Option<String>,
    }

    let mut containments: HashMap<String, ContainmentEntry> = HashMap::new();
    let mut current_id: Option<String> = None;
    let mut in_wallpaper_sub: bool = false;
    let mut in_root_containment: bool = false;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            let section = &trimmed[1..trimmed.len() - 1];
            if let Some(after) = section.strip_prefix("Containments][") {
                if let Some((id_str, rest)) = after.split_once(']') {
                    current_id = Some(id_str.to_string());
                    in_root_containment = rest.is_empty();
                    in_wallpaper_sub = rest.contains("Wallpaper") || rest.contains("Configuration][General") || rest.contains("General");
                } else {
                    current_id = Some(after.to_string());
                    in_root_containment = true;
                    in_wallpaper_sub = false;
                }
            } else {
                current_id = None;
                in_root_containment = false;
                in_wallpaper_sub = false;
            }
            continue;
        }

        let Some(ref id) = current_id else {
            continue;
        };

        if let Some((k, v)) = trimmed.split_once('=') {
            let key = k.trim();
            let val = v.trim();
            let entry = containments.entry(id.clone()).or_default();

            if in_root_containment {
                if key == "lastScreen" {
                    entry.last_screen = val.parse::<i32>().ok();
                } else if key == "plugin" {
                    entry.plugin = Some(val.to_string());
                }
            } else if in_wallpaper_sub && (key == "Image" || key == "usersWallpapers") {
                entry.image = Some(val.to_string());
            }
        }
    }

    let mut result = HashMap::new();
    let mut primary_path = None;

    for entry in containments.values() {
        if let Some(ref plug) = entry.plugin
            && (plug.contains("desktopcontainment")
                || plug.contains("plasma.folder")
                || (!plug.contains("panel") && !plug.contains("dock")))
            && let Some(screen) = entry.last_screen
            && screen >= 0
            && let Some(ref img_str) = entry.image
            && let Some(path) = sanitize_wallpaper_path(img_str)
        {
            let screen_key = screen.to_string();
            if screen == 0 {
                primary_path = Some(path.clone());
            }
            result.insert(screen_key, path);
        }
    }

    if let Some(pri) = primary_path {
        result.insert("default".to_string(), pri);
    } else if let Some((_, first_path)) = result.iter().next() {
        result.insert("default".to_string(), first_path.clone());
    }

    result
}

/// Backwards compatibility wrapper for parsing KDE applet configuration.
#[must_use]
pub fn parse_kde_appletsrc(content: &str) -> Option<PathBuf> {
    let parsed = parse_kde_containments(content);
    if let Some(p) = parsed.get("default") {
        return Some(p.clone());
    }
    if let Some(p) = parsed.values().next() {
        return Some(p.clone());
    }

    // Fallback: flat search across any Image= line
    for line in content.lines() {
        let trimmed = line.trim();
        if let Some((key, val)) = trimmed.split_once('=') {
            let key = key.trim();
            if (key == "Image" || key == "usersWallpapers")
                && let Some(path) = sanitize_wallpaper_path(val.trim())
            {
                return Some(path);
            }
        }
    }
    None
}

/// Resolves a KDE Plasma theme package directory (e.g. `/usr/share/wallpapers/Next/`)
/// to a physical image file inside `contents/images/` or `contents/images_dark/`.
#[must_use]
pub fn resolve_kde_theme_package(theme_dir: &Path, is_dark: bool) -> Option<PathBuf> {
    if !theme_dir.exists() || !theme_dir.is_dir() {
        return None;
    }

    let candidates = if is_dark {
        vec![
            theme_dir.join("contents/images_dark"),
            theme_dir.join("contents/images"),
        ]
    } else {
        vec![
            theme_dir.join("contents/images"),
            theme_dir.join("contents/images_dark"),
        ]
    };

    for dir in candidates {
        if !dir.is_dir() {
            continue;
        }

        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };

        let mut image_files: Vec<PathBuf> = entries
            .flatten()
            .map(|e| e.path())
            .filter(|p| {
                p.is_file()
                    && p.extension().is_some_and(|ext| {
                        let e = ext.to_string_lossy().to_lowercase();
                        e == "png" || e == "jpg" || e == "jpeg" || e == "webp" || e == "jxl"
                    })
            })
            .collect();

        // Sort descending to prefer higher resolutions (e.g. 3840x2160 over 800x600)
        image_files.sort_by(|a, b| {
            parse_resolution(b)
                .cmp(&parse_resolution(a))
                .then_with(|| b.file_name().cmp(&a.file_name()))
        });

        if let Some(best) = image_files.into_iter().next() {
            return Some(best);
        }
    }

    None
}

fn parse_resolution(p: &Path) -> u64 {
    let stem = p.file_stem().and_then(|s| s.to_str()).unwrap_or_default();
    if let Some((w, h)) = stem.split_once('x')
        && let (Ok(width), Ok(height)) = (w.parse::<u64>(), h.parse::<u64>())
    {
        return width * height;
    }
    0
}

/// Queries active wallpaper in KDE Plasma via config parsing.
pub fn query_kde() -> Result<MonitorWallpapers> {
    let mut candidates = Vec::new();
    if let Some(config_dir) = std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from) {
        candidates.push(config_dir.join("plasma-org.kde.plasma.desktop-appletsrc"));
    }
    if let Some(home) = std::env::var_os("HOME").map(PathBuf::from) {
        candidates.push(home.join(".config/plasma-org.kde.plasma.desktop-appletsrc"));
    }

    let is_dark = is_system_dark();

    for path in candidates {
        if path.exists()
            && let Ok(content) = std::fs::read_to_string(&path)
        {
            let parsed = parse_kde_containments(&content);
            if !parsed.is_empty() {
                let mut resolved_outputs = HashMap::new();
                for (mon, mut wall_path) in parsed {
                    if wall_path.is_dir()
                        && let Some(img) = resolve_kde_theme_package(&wall_path, is_dark)
                    {
                        wall_path = img;
                    }
                    if wall_path.is_file() {
                        resolved_outputs.insert(mon, wall_path);
                    }
                }
                if !resolved_outputs.is_empty() {
                    return Ok(MonitorWallpapers { outputs: resolved_outputs });
                }
            }
        }
    }

    Err(TermonetError::WallpaperNotFound(
        "No active KDE Plasma wallpaper found in plasma-org.kde.plasma.desktop-appletsrc".into(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_percent_decode() {
        assert_eq!(percent_decode("hello%20world"), "hello world");
        assert_eq!(percent_decode("/path/with%2Fslash"), "/path/with/slash");
        assert_eq!(percent_decode("simple_text.png"), "simple_text.png");
        assert_eq!(percent_decode("invalid%2"), "invalid%2");
    }

    #[test]
    fn test_parse_gnome_uri() {
        let raw = "'file:///usr/share/backgrounds/gnome/adwaita-d.jxl'";
        assert_eq!(
            parse_gnome_uri(raw),
            Some(PathBuf::from("/usr/share/backgrounds/gnome/adwaita-d.jxl"))
        );

        let with_spaces = "'file:///home/user/My%20Photos/wall.jpg'";
        assert_eq!(
            parse_gnome_uri(with_spaces),
            Some(PathBuf::from("/home/user/My Photos/wall.jpg"))
        );

        let plain_path = "\"/home/user/wall.png\"";
        assert_eq!(
            parse_gnome_uri(plain_path),
            Some(PathBuf::from("/home/user/wall.png"))
        );
    }

    #[test]
    fn test_gnome_none_picture_options_rejected() {
        assert_eq!(parse_gnome_uri("none"), None);
        assert_eq!(parse_gnome_uri("'none'"), None);
        assert_eq!(parse_gnome_uri("\"none\""), None);
    }

    #[test]
    fn test_parse_kde_appletsrc() {
        let content = "\
[Containments][1][Applets][2][Configuration][General]
Image=file:///usr/share/wallpapers/Next/contents/images/3840x2160.png
SlidePaths=/usr/share/wallpapers
";
        let parsed = parse_kde_appletsrc(content);
        assert_eq!(
            parsed,
            Some(PathBuf::from("/usr/share/wallpapers/Next/contents/images/3840x2160.png"))
        );
    }

    #[test]
    fn test_parse_kde_containments_filters_panels_and_disabled_screens() {
        let content = "\
[Containments][1]
lastScreen=-1
plugin=org.kde.desktopcontainment
wallpaperplugin=org.kde.image

[Containments][1][Wallpaper][org.kde.image][General]
Image=file:///usr/share/wallpapers/Disabled.png

[Containments][2]
lastScreen=0
plugin=org.kde.panel

[Containments][3]
lastScreen=0
plugin=org.kde.desktopcontainment
wallpaperplugin=org.kde.image

[Containments][3][Wallpaper][org.kde.image][General]
Image=file:///usr/share/wallpapers/Primary.png
";
        let parsed = parse_kde_containments(content);
        assert_eq!(parsed.get("0"), Some(&PathBuf::from("/usr/share/wallpapers/Primary.png")));
        assert_eq!(parsed.get("default"), Some(&PathBuf::from("/usr/share/wallpapers/Primary.png")));
        assert!(!parsed.contains_key("-1"));
    }

    #[test]
    fn test_resolve_kde_theme_package() {
        let temp_theme = std::env::temp_dir().join("termonet_test_kde_theme");
        let dark_dir = temp_theme.join("contents/images_dark");
        let light_dir = temp_theme.join("contents/images");
        let _ = std::fs::create_dir_all(&dark_dir);
        let _ = std::fs::create_dir_all(&light_dir);

        let dark_img = dark_dir.join("3840x2160.png");
        let light_img = light_dir.join("1920x1080.png");
        std::fs::write(&dark_img, b"fake dark").unwrap();
        std::fs::write(&light_img, b"fake light").unwrap();

        let resolved_dark = resolve_kde_theme_package(&temp_theme, true);
        assert_eq!(resolved_dark, Some(dark_img));

        let resolved_light = resolve_kde_theme_package(&temp_theme, false);
        assert_eq!(resolved_light, Some(light_img));

        let _ = std::fs::remove_dir_all(&temp_theme);
    }

    #[test]
    fn test_parse_kde_containments_plasma_folder_view() {
        let content = "\
[Containments][1]
lastScreen=0
plugin=org.kde.plasma.folder
wallpaperplugin=org.kde.image

[Containments][1][Wallpaper][org.kde.image][General]
Image=file:///usr/share/wallpapers/FolderView.png
";
        let parsed = parse_kde_containments(content);
        assert_eq!(parsed.get("0"), Some(&PathBuf::from("/usr/share/wallpapers/FolderView.png")));
        assert_eq!(parsed.get("default"), Some(&PathBuf::from("/usr/share/wallpapers/FolderView.png")));
    }

    #[test]
    fn test_resolve_kde_theme_package_resolution_priority() {
        let temp_theme = std::env::temp_dir().join("termonet_test_kde_theme_res");
        let img_dir = temp_theme.join("contents/images");
        let _ = std::fs::create_dir_all(&img_dir);

        let low_res = img_dir.join("800x600.png");
        let high_res = img_dir.join("3840x2160.png");
        std::fs::write(&low_res, b"low").unwrap();
        std::fs::write(&high_res, b"high").unwrap();

        let resolved = resolve_kde_theme_package(&temp_theme, false);
        assert_eq!(resolved, Some(high_res));

        let _ = std::fs::remove_dir_all(&temp_theme);
    }
}
