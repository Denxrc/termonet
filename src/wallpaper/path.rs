//! Unified path sanitization, percent-decoding, and filesystem validation.

use std::path::{Path, PathBuf};
use crate::config::paths::AppPaths;

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
        if bytes[i] == b'%'
            && i + 2 < bytes.len()
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

/// Expands environment variables like `$HOME` or `${HOME}` and user tilde `~`.
#[must_use]
pub fn expand_env_and_tilde(input: &str) -> PathBuf {
    let mut res = input.to_string();
    if let Some(home) = std::env::var_os("HOME") {
        let home_str = home.to_string_lossy();
        if res.contains("$HOME") {
            res = res.replace("$HOME", &home_str);
        }
        if res.contains("${HOME}") {
            res = res.replace("${HOME}", &home_str);
        }
    }
    AppPaths::expand_tilde(&res)
}

/// Sanitizes an input string to extract a candidate wallpaper file path without performing disk I/O.
///
/// Steps:
/// 1. Strips leading and trailing whitespace and quotes (`'`, `"`).
/// 2. Rejects solid color indicators (`color:`, `#`, `0x`, `rgb:`).
/// 3. Safely strips `file://` scheme prefix.
/// 4. Strips trailing geometry or filter tags like `(scale: 1.0)`.
/// 5. Applies URL percent-decoding (`%20` -> space, UTF-8 sequences).
/// 6. Expands `~` and environment variables (`$HOME`).
#[must_use]
pub fn sanitize_wallpaper_path(raw: &str) -> Option<PathBuf> {
    let mut trimmed = raw.trim().trim_matches(|c| c == '\'' || c == '"');
    if trimmed.is_empty() {
        return None;
    }

    // Reject solid color definitions or keywords
    let lower = trimmed.to_lowercase();
    if lower.starts_with("color:")
        || lower.starts_with('#')
        || lower.starts_with("0x")
        || lower.starts_with("rgb:")
        || lower == "none"
    {
        return None;
    }

    // Safely remove `file://` prefix
    if let Some(without_file) = trimmed.strip_prefix("file://") {
        trimmed = without_file;
    }

    // Strip trailing tags like (scale: 1.0) only if the string ends with a tag in parentheses
    if trimmed.ends_with(')')
        && let Some((before_paren, tag)) = trimmed.rsplit_once('(')
    {
        let tag_inner = tag.trim_end_matches(')').trim();
        if tag_inner.starts_with("scale:")
            || tag_inner.starts_with("fit:")
            || tag_inner.starts_with("mode:")
            || tag_inner.contains(':')
        {
            let before_trimmed = before_paren.trim();
            if !before_trimmed.is_empty() {
                trimmed = before_trimmed;
            }
        }
    }

    // Percent-decode
    let decoded = percent_decode(trimmed);
    let mut clean = decoded.trim().trim_matches(|c| c == '\'' || c == '"');

    // If candidate contains crop / scaling flags (e.g. `crop:center Lanczos3 /path/to/wall.png`)
    if !clean.starts_with('/')
        && !clean.starts_with('~')
        && !clean.starts_with("$HOME")
        && !clean.starts_with("${HOME}")
    {
        let path_start_idx = [
            clean.find("~/"),
            clean.find('/'),
            clean.find("$HOME"),
            clean.find("${HOME}"),
        ]
        .into_iter()
        .flatten()
        .min();
        if let Some(idx) = path_start_idx {
            clean = clean[idx..].trim_matches(|c| c == '\'' || c == '"' || c == ' ');
        }
    }

    if clean.is_empty() {
        return None;
    }

    Some(expand_env_and_tilde(clean))
}

/// Validates whether a given path physically exists on disk and is a regular file.
#[must_use]
pub fn validate_wallpaper_path(path: &Path) -> bool {
    path.exists() && path.is_file()
}

/// Executes full sanitization and filesystem validation pipeline.
#[must_use]
pub fn clean_wallpaper_path(raw: &str) -> Option<PathBuf> {
    sanitize_wallpaper_path(raw).filter(|p| validate_wallpaper_path(p))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_quotes_and_file_scheme() {
        assert_eq!(
            sanitize_wallpaper_path("'file:///usr/share/wallpapers/image.png'"),
            Some(PathBuf::from("/usr/share/wallpapers/image.png"))
        );
        assert_eq!(
            sanitize_wallpaper_path("\"/usr/share/wallpapers/image.png\""),
            Some(PathBuf::from("/usr/share/wallpapers/image.png"))
        );
    }

    #[test]
    fn test_sanitize_percent_decoding_and_spaces() {
        assert_eq!(
            sanitize_wallpaper_path("file:///home/user/My%20Wallpapers/nature%20view.png"),
            Some(PathBuf::from("/home/user/My Wallpapers/nature view.png"))
        );
    }

    #[test]
    fn test_sanitize_color_rejection() {
        assert_eq!(sanitize_wallpaper_path("color: 000000"), None);
        assert_eq!(sanitize_wallpaper_path("#1a2b3c"), None);
        assert_eq!(sanitize_wallpaper_path("0x000000"), None);
    }

    #[test]
    fn test_sanitize_trailing_tags_and_whitespace() {
        assert_eq!(
            sanitize_wallpaper_path("  /path/to/wall.png (scale: 1.0)  "),
            Some(PathBuf::from("/path/to/wall.png"))
        );
    }

    #[test]
    fn test_validate_wallpaper_path() {
        let temp_file = std::env::temp_dir().join("termonet_test_validate.png");
        std::fs::write(&temp_file, b"test").unwrap();
        assert!(validate_wallpaper_path(&temp_file));

        let non_existent = std::env::temp_dir().join("termonet_test_does_not_exist.png");
        assert!(!validate_wallpaper_path(&non_existent));

        let _ = std::fs::remove_file(&temp_file);
    }

    #[test]
    fn test_percent_decode_cyrillic() {
        // %D0%BF%D1%80%D0%B8%D0%B2%D0%B5%D1%82 -> привет
        assert_eq!(percent_decode("%D0%BF%D1%80%D0%B8%D0%B2%D0%B5%D1%82"), "привет");
    }

    #[test]
    fn test_sanitize_parens_in_filename() {
        assert_eq!(
            sanitize_wallpaper_path("/home/user/Pictures/wallpaper (1).png"),
            Some(PathBuf::from("/home/user/Pictures/wallpaper (1).png"))
        );
        assert_eq!(
            sanitize_wallpaper_path("/home/user/Pictures/wall (copy).jpg"),
            Some(PathBuf::from("/home/user/Pictures/wall (copy).jpg"))
        );
        assert_eq!(
            sanitize_wallpaper_path("/home/user/Pictures/wall.png (scale: 1.0)"),
            Some(PathBuf::from("/home/user/Pictures/wall.png"))
        );
    }

    #[test]
    fn test_sanitize_braced_home_variable() {
        if let Ok(home) = std::env::var("HOME") {
            assert_eq!(
                sanitize_wallpaper_path("${HOME}/Pictures/wall.png"),
                Some(PathBuf::from(home).join("Pictures/wall.png"))
            );
        }
    }
}
