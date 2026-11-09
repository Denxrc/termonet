//! Wallpaper query implementations for Wayland compositors and daemons.

use std::collections::HashMap;
use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use crate::config::paths::AppPaths;
use crate::error::{Result, TermonetError};
use crate::wallpaper::monitor::MonitorWallpapers;

/// Helper function to extract an image file path from a string that may contain flags,
/// crop modes, scaling filters, or surrounding quotes (e.g. `crop:center Lanczos3 /path/to/wall.png`).
#[must_use]
pub fn extract_wallpaper_path_from_str(s: &str) -> Option<PathBuf> {
    let mut trimmed = s.trim().trim_matches(|c| c == '\'' || c == '"');
    if trimmed.is_empty() {
        return None;
    }

    // If string contains trailing parentheses like (scale: 1.0), strip them
    if let Some((before_paren, _)) = trimmed.split_once('(') {
        let before_trimmed = before_paren.trim();
        if !before_trimmed.is_empty() && (before_trimmed.starts_with('/') || before_trimmed.starts_with('~')) {
            trimmed = before_trimmed;
        }
    }

    // Direct path starting with / or ~
    if trimmed.starts_with('/') || trimmed.starts_with('~') {
        return Some(AppPaths::expand_tilde(trimmed));
    }

    // Search for the first occurrence of '~/' or '/' (prioritize '~/' so flags don't strip tilde)
    let path_start_idx = match (trimmed.find("~/"), trimmed.find('/')) {
        (Some(tilde_idx), Some(slash_idx)) => Some(tilde_idx.min(slash_idx)),
        (Some(tilde_idx), None) => Some(tilde_idx),
        (None, Some(slash_idx)) => Some(slash_idx),
        (None, None) => None,
    };
    if let Some(idx) = path_start_idx {
        let candidate = trimmed[idx..].trim_matches(|c| c == '\'' || c == '"' || c == ' ');
        if !candidate.is_empty() {
            let clean_candidate = candidate.split_once('(').map(|(b, _)| b.trim()).unwrap_or(candidate);
            return Some(AppPaths::expand_tilde(clean_candidate));
        }
    }

    // Fallback: check whitespace-separated tokens in reverse
    for token in trimmed.split_whitespace().rev() {
        let clean = token.trim_matches(|c| c == '\'' || c == '"' || c == '(' || c == ')');
        if clean.starts_with('/') || clean.starts_with('~') {
            return Some(AppPaths::expand_tilde(clean));
        }
    }

    None
}

/// Queries active wallpapers from `awww` or `swww` CLI or disk cache.
/// Prioritizes the modern `awww` binary, falling back to `swww` and finally inspecting ~/.cache/awww.
pub fn query_awww_or_swww() -> Result<MonitorWallpapers> {
    // 1. Try awww query CLI
    if let Ok(output) = Command::new("awww").arg("query").output()
        && output.status.success()
    {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let parsed = parse_awww_or_swww_output(&stdout);
        if !parsed.is_empty() {
            return Ok(MonitorWallpapers { outputs: parsed });
        }
    }

    // 2. Try swww query fallback CLI
    if let Ok(output) = Command::new("swww").arg("query").output()
        && output.status.success()
    {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let parsed = parse_awww_or_swww_output(&stdout);
        if !parsed.is_empty() {
            return Ok(MonitorWallpapers { outputs: parsed });
        }
    }

    // 3. Fallback: inspect disk cache in ~/.cache/awww and ~/.cache/swww
    if let Ok(cached) = query_awww_or_swww_cache() {
        return Ok(cached);
    }

    Err(TermonetError::WallpaperNotFound(
        "No active awww or swww wallpaper detected (checked awww query, swww query, and ~/.cache/awww)".into(),
    ))
}

/// Fallback: scans ~/.cache/awww/ and ~/.cache/swww/ for cached active outputs.
pub fn query_awww_or_swww_cache() -> Result<MonitorWallpapers> {
    let home = std::env::var_os("HOME").map(PathBuf::from);
    let Some(home) = home else {
        return Err(TermonetError::WallpaperNotFound("HOME directory not set".into()));
    };

    let cache_dirs = [
        home.join(".cache/awww"),
        home.join(".cache/swww"),
    ];

    let mut outputs = HashMap::new();

    for base_dir in cache_dirs {
        if !base_dir.is_dir() {
            continue;
        }

        let Ok(entries) = std::fs::read_dir(&base_dir) else {
            continue;
        };

        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir() {
                // e.g. ~/.cache/awww/0.12.1/
                if let Ok(sub_entries) = std::fs::read_dir(&p) {
                    for sub in sub_entries.flatten() {
                        let file_name = sub.file_name().to_string_lossy().to_string();
                        // Ignore internal cache bitmaps/gifs starting with '_' or '.'
                        if file_name.starts_with('_') || file_name.starts_with('.') {
                            continue;
                        }
                        if let Ok(content) = std::fs::read_to_string(sub.path())
                            && let Some(path) = extract_wallpaper_path_from_str(&content)
                            && path.exists()
                        {
                            outputs.insert(file_name, path);
                        }
                    }
                }
            } else {
                let file_name = p.file_name().unwrap_or_default().to_string_lossy().to_string();
                if !file_name.starts_with('_')
                    && !file_name.starts_with('.')
                    && let Ok(content) = std::fs::read_to_string(&p)
                    && let Some(path) = extract_wallpaper_path_from_str(&content)
                    && path.exists()
                {
                    outputs.insert(file_name, path);
                }
            }
        }

        if !outputs.is_empty() {
            return Ok(MonitorWallpapers { outputs });
        }
    }

    Err(TermonetError::WallpaperNotFound("No active awww/swww wallpapers found in cache".into()))
}

/// Backwards compatibility alias for `query_awww_or_swww`.
pub fn query_swww() -> Result<MonitorWallpapers> {
    query_awww_or_swww()
}

/// Parses output from `awww query` or `swww query`.
/// Example formats:
/// - `DP-1: 2560x1440, scale: 1, currently displaying: /path/to/image.png`
/// - `DP-1: 2560x1440\t, scale: 1, currently displaying:  crop:center Lanczos3 /path/to/image.png`
/// - `eDP-1: currently displaying: /path/to/image.png`
/// - `HDMI-A-1: /path/to/image.png`
#[must_use]
pub fn parse_awww_or_swww_output(raw: &str) -> HashMap<String, PathBuf> {
    let mut map = HashMap::new();
    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.to_lowercase().starts_with("error") {
            continue;
        }

        let Some((mon_part, rest)) = trimmed.split_once(':') else {
            continue;
        };
        let monitor = mon_part.trim().to_string();

        let path_str = if let Some((_, path_part)) = rest.split_once("currently displaying:") {
            path_part.trim()
        } else {
            let parts: Vec<&str> = rest.split(',').map(str::trim).collect();
            parts.last().copied().unwrap_or(rest.trim())
        };

        if let Some(path) = extract_wallpaper_path_from_str(path_str) {
            map.insert(monitor, path);
        }
    }
    map
}

/// Queries active wallpapers from `hyprpaper` via UNIX IPC socket, `hyprctl`, or static config.
pub fn query_hyprpaper() -> Result<MonitorWallpapers> {
    // 1. Try direct Unix domain socket IPC if HYPRLAND_INSTANCE_SIGNATURE is set
    if let Some(signature) = std::env::var_os("HYPRLAND_INSTANCE_SIGNATURE") {
        let runtime_dir = std::env::var_os("XDG_RUNTIME_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                use std::os::unix::fs::MetadataExt;
                let uid = std::fs::metadata("/proc/self")
                    .map(|m| m.uid())
                    .unwrap_or(1000);
                PathBuf::from(format!("/run/user/{}", uid))
            });
        let socket_path = runtime_dir
            .join("hypr")
            .join(signature)
            .join(".hyprpaper.sock");

        if socket_path.exists()
            && let Ok(mut stream) = UnixStream::connect(&socket_path)
        {
            let _ = stream.set_read_timeout(Some(Duration::from_millis(150)));
            let _ = stream.set_write_timeout(Some(Duration::from_millis(150)));
            if stream.write_all(b"listactive\n").is_ok() {
                let mut response = String::new();
                if stream.read_to_string(&mut response).is_ok() {
                    let parsed = parse_hyprpaper_active_output(&response);
                    if !parsed.is_empty() {
                        return Ok(MonitorWallpapers { outputs: parsed });
                    }
                }
            }
        }
    }

    // 2. Try hyprctl hyprpaper listactive CLI
    if let Ok(output) = Command::new("hyprctl")
        .args(["hyprpaper", "listactive"])
        .output()
        && output.status.success()
    {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let parsed = parse_hyprpaper_active_output(&stdout);
        if !parsed.is_empty() {
            return Ok(MonitorWallpapers { outputs: parsed });
        }
    }

    // 3. Fallback: Parse ~/.config/hypr/hyprpaper.conf
    let home = std::env::var_os("HOME").map(PathBuf::from);
    if let Some(h) = home {
        let conf_path = h.join(".config/hypr/hyprpaper.conf");
        if conf_path.exists()
            && let Ok(content) = std::fs::read_to_string(&conf_path)
        {
            let parsed = parse_hyprpaper_conf(&content);
            if !parsed.is_empty() {
                return Ok(MonitorWallpapers { outputs: parsed });
            }
        }
    }

    Err(TermonetError::WallpaperNotFound(
        "No active hyprpaper wallpaper detected (socket, hyprctl, and hyprpaper.conf failed)".into(),
    ))
}

/// Parses output from `hyprpaper` IPC / `hyprctl hyprpaper listactive`.
/// Expected formats:
/// - `DP-1 = /home/user/Pictures/wallpaper.png`
/// - `eDP-1 = /home/user/Pictures/wallpaper.png (scale: 1.0)`
/// - `/home/user/Pictures/wallpaper.png`
#[must_use]
pub fn parse_hyprpaper_active_output(raw: &str) -> HashMap<String, PathBuf> {
    let mut map = HashMap::new();
    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty()
            || trimmed.to_lowercase().starts_with("error")
            || trimmed.to_lowercase().starts_with("ok")
        {
            continue;
        }

        if let Some((mon, path_part)) = trimmed.split_once('=').or_else(|| trimmed.split_once(':')) {
            let mon = mon.trim();
            if let Some(path) = extract_wallpaper_path_from_str(path_part) {
                map.insert(mon.to_string(), path);
            }
        } else if let Some(path) = extract_wallpaper_path_from_str(trimmed) {
            map.insert("default".to_string(), path);
        }
    }
    map
}

/// Parses static wallpaper declarations from `hyprpaper.conf`.
/// Example: `wallpaper = DP-1,/path/to/wallpaper.png` or `wallpaper = ,/path/to/wallpaper.png`
#[must_use]
pub fn parse_hyprpaper_conf(content: &str) -> HashMap<String, PathBuf> {
    let mut map = HashMap::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('#') || !trimmed.starts_with("wallpaper") {
            continue;
        }

        let Some((_, rest)) = trimmed.split_once('=') else {
            continue;
        };
        let rest = rest.trim();
        if let Some((mon, path_str)) = rest.split_once(',') {
            let mon = mon.trim();
            let key = if mon.is_empty() { "default".to_string() } else { mon.to_string() };
            if let Some(path) = extract_wallpaper_path_from_str(path_str) {
                map.insert(key, path);
            }
        }
    }
    map
}

/// Queries active wallpapers from `swaybg` by inspecting process commandline args in `/proc`.
pub fn query_swaybg() -> Result<MonitorWallpapers> {
    query_swaybg_from_proc(Path::new("/proc"))
}

/// Helper function to inspect a custom `/proc` directory (facilitates mock unit testing).
pub fn query_swaybg_from_proc(proc_root: &Path) -> Result<MonitorWallpapers> {
    let entries = std::fs::read_dir(proc_root)
        .map_err(|e| TermonetError::WallpaperNotFound(format!("Failed to read {}: {e}", proc_root.display())))?;

    for entry in entries.flatten() {
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if !name_str.chars().all(|c| c.is_ascii_digit()) {
            continue;
        }

        let cmdline_path = entry.path().join("cmdline");
        let Ok(cmdline_bytes) = std::fs::read(&cmdline_path) else {
            continue;
        };

        if cmdline_bytes.is_empty() {
            continue;
        }

        // Fast-path rejection: check if the binary ends with swaybg before allocating Vec<String>
        let first_arg = cmdline_bytes.split(|&b| b == 0).next().unwrap_or(&[]);
        if !first_arg.ends_with(b"swaybg") {
            continue;
        }

        let args: Vec<String> = cmdline_bytes
            .split(|&b| b == 0)
            .filter(|slice| !slice.is_empty())
            .map(|slice| String::from_utf8_lossy(slice).trim().to_string())
            .collect();

        if args.is_empty() {
            continue;
        }

        let exec = &args[0];
        if exec.ends_with("swaybg") {
            let outputs = parse_swaybg_args(&args[1..]);
            if !outputs.is_empty() {
                return Ok(MonitorWallpapers { outputs });
            }
        }
    }

    Err(TermonetError::WallpaperNotFound("No active swaybg process found in proc".into()))
}

/// Parses CLI arguments passed to `swaybg`.
/// Handles `-o <output> -i <path>`, `--output <output> --image <path>`, and `-i <path>`.
#[must_use]
pub fn parse_swaybg_args(args: &[String]) -> HashMap<String, PathBuf> {
    let mut outputs = HashMap::new();
    let mut current_output = None;
    let mut i = 0;

    while i < args.len() {
        match args[i].as_str() {
            "-o" | "--output" if i + 1 < args.len() => {
                current_output = Some(args[i + 1].clone());
                i += 1;
            }
            "-i" | "--image" if i + 1 < args.len() => {
                if let Some(img_path) = extract_wallpaper_path_from_str(&args[i + 1]) {
                    let mon = current_output.take().unwrap_or_else(|| "default".to_string());
                    outputs.insert(mon, img_path);
                }
                i += 1;
            }
            _ => {}
        }
        i += 1;
    }

    outputs
}

/// Queries active wallpapers from `wpaperd` by inspecting symlinks in its state/cache directory.
pub fn query_wpaperd() -> Result<MonitorWallpapers> {
    let mut candidates = Vec::new();
    if let Some(state_dir) = std::env::var_os("XDG_STATE_HOME").map(PathBuf::from) {
        candidates.push(state_dir.join("wpaperd/wallpapers"));
    }
    if let Some(home) = std::env::var_os("HOME").map(PathBuf::from) {
        candidates.push(home.join(".local/state/wpaperd/wallpapers"));
        candidates.push(home.join(".cache/wpaperd"));
    }

    for dir in candidates {
        if dir.is_dir()
            && let Ok(monitors) = query_wpaperd_from_dir(&dir)
        {
            return Ok(monitors);
        }
    }

    Err(TermonetError::WallpaperNotFound("No active wpaperd wallpapers found".into()))
}

/// Helper function to inspect symlinks in a wpaperd output directory.
pub fn query_wpaperd_from_dir(dir: &Path) -> Result<MonitorWallpapers> {
    if !dir.exists() || !dir.is_dir() {
        return Err(TermonetError::WallpaperNotFound(format!(
            "wpaperd directory {} does not exist",
            dir.display()
        )));
    }

    let mut outputs = HashMap::new();
    let entries = std::fs::read_dir(dir)
        .map_err(|e| TermonetError::WallpaperNotFound(e.to_string()))?;

    for entry in entries.flatten() {
        let file_name = entry.file_name().to_string_lossy().to_string();
        let path = entry.path();
        if let Ok(target) = std::fs::read_link(&path) {
            let resolved = if target.is_absolute() {
                target
            } else if let Some(parent) = path.parent() {
                parent.join(target)
            } else {
                target
            };
            if resolved.exists() {
                outputs.insert(file_name, resolved);
            }
        }
    }

    if outputs.is_empty() {
        Err(TermonetError::WallpaperNotFound("No valid symlinks found in wpaperd directory".into()))
    } else {
        Ok(MonitorWallpapers { outputs })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_wallpaper_path_with_crop_and_filter() {
        let raw = " crop:center Lanczos3 /home/Denxrc/Pictures/wallp.png";
        assert_eq!(
            extract_wallpaper_path_from_str(raw),
            Some(PathBuf::from("/home/Denxrc/Pictures/wallp.png"))
        );

        if let Ok(home) = std::env::var("HOME") {
            let tilde_with_crop = " crop:center Lanczos3 ~/Pictures/wallp.png";
            assert_eq!(
                extract_wallpaper_path_from_str(tilde_with_crop),
                Some(PathBuf::from(home).join("Pictures/wallp.png"))
            );
        }

        let with_spaces = "image: '/home/user/Pictures/wall with space.jpg'";
        assert_eq!(
            extract_wallpaper_path_from_str(with_spaces),
            Some(PathBuf::from("/home/user/Pictures/wall with space.jpg"))
        );
    }

    #[test]
    fn test_parse_awww_or_swww_output_standard() {
        let raw = "\
DP-1: 2560x1440, scale: 1, currently displaying: /home/user/Pictures/wallpaper1.png
HDMI-A-1: 1920x1080, scale: 1, currently displaying: /home/user/Pictures/wallpaper2.jpg
";
        let parsed = parse_awww_or_swww_output(raw);
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed.get("DP-1"), Some(&PathBuf::from("/home/user/Pictures/wallpaper1.png")));
        assert_eq!(parsed.get("HDMI-A-1"), Some(&PathBuf::from("/home/user/Pictures/wallpaper2.jpg")));
    }

    #[test]
    fn test_parse_awww_with_modern_crop_output() {
        let raw = "DP-1: 2560x1440\t, scale: 1, currently displaying:  crop:center Lanczos3 /home/Denxrc/Pictures/wallp.png\n";
        let parsed = parse_awww_or_swww_output(raw);
        assert_eq!(parsed.get("DP-1"), Some(&PathBuf::from("/home/Denxrc/Pictures/wallp.png")));
    }

    #[test]
    fn test_parse_awww_or_swww_with_errors_and_quotes() {
        let raw = "\
Error: socket not connected
eDP-1: currently displaying: '/home/user/wall space.png'
";
        let parsed = parse_awww_or_swww_output(raw);
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed.get("eDP-1"), Some(&PathBuf::from("/home/user/wall space.png")));
    }

    #[test]
    fn test_parse_hyprpaper_active_output() {
        let raw = "\
DP-1 = /home/user/wall.png
eDP-1 = /home/user/wall2.png (scale: 1.0)
";
        let parsed = parse_hyprpaper_active_output(raw);
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed.get("DP-1"), Some(&PathBuf::from("/home/user/wall.png")));
        assert_eq!(parsed.get("eDP-1"), Some(&PathBuf::from("/home/user/wall2.png")));
    }

    #[test]
    fn test_parse_hyprpaper_conf() {
        let conf = "\
preload = /home/user/wall.png
wallpaper = DP-1,/home/user/wall.png
wallpaper = ,/home/user/fallback.png
# comment
";
        let parsed = parse_hyprpaper_conf(conf);
        assert_eq!(parsed.get("DP-1"), Some(&PathBuf::from("/home/user/wall.png")));
        assert_eq!(parsed.get("default"), Some(&PathBuf::from("/home/user/fallback.png")));
    }

    #[test]
    fn test_parse_swaybg_args() {
        let args = vec![
            "-o".into(),
            "eDP-1".into(),
            "-i".into(),
            "/home/user/wall.png".into(),
            "-m".into(),
            "fill".into(),
        ];
        let parsed = parse_swaybg_args(&args);
        assert_eq!(parsed.get("eDP-1"), Some(&PathBuf::from("/home/user/wall.png")));

        let args_global = vec!["-i".into(), "/home/user/global.png".into()];
        let parsed_global = parse_swaybg_args(&args_global);
        assert_eq!(parsed_global.get("default"), Some(&PathBuf::from("/home/user/global.png")));
    }

    #[test]
    fn test_query_swaybg_from_mock_proc() {
        let temp_dir = std::env::temp_dir().join("termonet_test_proc");
        let _ = std::fs::remove_dir_all(&temp_dir);
        let pid_dir = temp_dir.join("1234");
        std::fs::create_dir_all(&pid_dir).unwrap();

        let cmdline_content = b"/usr/bin/swaybg\0-o\0DP-2\0-i\0/tmp/swaywall.png\0";
        std::fs::write(pid_dir.join("cmdline"), cmdline_content).unwrap();

        let res = query_swaybg_from_proc(&temp_dir).unwrap();
        assert_eq!(res.outputs.get("DP-2"), Some(&PathBuf::from("/tmp/swaywall.png")));

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
