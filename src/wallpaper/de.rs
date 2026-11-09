//! Wallpaper query implementations for Desktop Environments (GNOME, KDE Plasma).

use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;

use crate::config::paths::AppPaths;
use crate::error::{Result, TermonetError};
use crate::wallpaper::monitor::MonitorWallpapers;

/// Helper function to decode percent-encoded URI strings (e.g. `%20` -> ` `).
#[must_use]
pub fn percent_decode(input: &str) -> String {
    if !input.contains('%') {
        return input.to_string();
    }

    let bytes = input.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len()
            && let (Some(v1), Some(v2)) = (hex_digit(bytes[i + 1]), hex_digit(bytes[i + 2]))
        {
            decoded.push((v1 << 4) | v2);
            i += 3;
            continue;
        }
        decoded.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&decoded).into_owned()
}

fn hex_digit(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

/// Parses and cleans a GNOME or generic desktop wallpaper URI string.
/// Handles stripping quotes, `file://` scheme, and percent decoding.
#[must_use]
pub fn parse_gnome_uri(raw: &str) -> Option<PathBuf> {
    let trimmed = raw.trim().trim_matches(|c| c == '\'' || c == '"' || c == ' ');
    if trimmed.is_empty() {
        return None;
    }

    let without_scheme = if let Some(stripped) = trimmed.strip_prefix("file://") {
        stripped
    } else {
        trimmed
    };

    let decoded = percent_decode(without_scheme);
    if decoded.is_empty() {
        None
    } else {
        Some(AppPaths::expand_tilde(&decoded))
    }
}

/// Queries active wallpaper in GNOME desktop via `gsettings`.
pub fn query_gnome() -> Result<MonitorWallpapers> {
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

/// Parses KDE Plasma applet configuration for wallpaper image paths.
/// Looks for `Image=` or `usersWallpapers=` properties.
#[must_use]
pub fn parse_kde_appletsrc(content: &str) -> Option<PathBuf> {
    let mut candidate = None;
    for line in content.lines() {
        let trimmed = line.trim();
        if let Some((key, val)) = trimmed.split_once('=') {
            let key = key.trim();
            if (key == "Image" || key == "usersWallpapers")
                && let Some(path) = parse_gnome_uri(val.trim())
            {
                candidate = Some(path);
            }
        }
    }
    candidate
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

    for path in candidates {
        if path.exists()
            && let Ok(content) = std::fs::read_to_string(&path)
            && let Some(wallpaper_path) = parse_kde_appletsrc(&content)
            && wallpaper_path.exists()
        {
            let mut outputs = HashMap::new();
            outputs.insert("default".to_string(), wallpaper_path);
            return Ok(MonitorWallpapers { outputs });
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
}
