//! Wallpaper query implementations for Wayland compositors and daemons.

use std::collections::HashMap;
use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use crate::error::{Result, TermonetError};
use crate::wallpaper::monitor::MonitorWallpapers;

/// Helper function to extract an image file path from a string that may contain flags,
/// crop modes, scaling filters, or surrounding quotes (e.g. `crop:center Lanczos3 /path/to/wall.png`).
#[must_use]
pub fn extract_wallpaper_path_from_str(s: &str) -> Option<PathBuf> {
    crate::wallpaper::path::sanitize_wallpaper_path(s)
}

/// Queries active wallpapers from `awww` or `swww` CLI or disk cache.
/// Prioritizes the modern `awww` binary, falling back to `swww` and finally inspecting ~/.cache/awww.
pub fn query_awww_or_swww() -> Result<MonitorWallpapers> {
    // 1. Try awww query CLI (with --all to capture all namespaces)
    if let Ok(output) = Command::new("awww").args(["query", "--all"]).output()
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

    // 3. Fallback: inspect disk cache in ~/.cache/awww and ~/.cache/swww only if runtime socket/dir exists
    let runtime_dir = std::env::var_os("XDG_RUNTIME_DIR").map(PathBuf::from);
    let daemon_running = runtime_dir.as_ref().is_some_and(|rt| {
        rt.join("awww.sock").exists()
            || rt.join("swww.sock").exists()
            || rt.join("swww").is_dir()
            || rt.join("awww").is_dir()
    });
    if daemon_running && let Ok(cached) = query_awww_or_swww_cache() {
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

/// Parses output from `awww query --all` or `swww query`.
/// Example formats:
/// - `DP-1: 2560x1440, scale: 1, currently displaying: /path/to/image.png`
/// - `DP-1: 2560x1440\t, scale: 1, currently displaying:  crop:center Lanczos3 /path/to/image.png`
/// - `: DP-1: 2560x1440, scale: 1, currently displaying: image: /path/to/image.png`
/// - `test: DP-1: 2560x1440, scale: 1, currently displaying: color: 000000`
/// - `HDMI-A-1: /path/to/image.png`
#[must_use]
pub fn parse_awww_or_swww_output(raw: &str) -> HashMap<String, PathBuf> {
    let mut default_ns_map = HashMap::new();
    let mut custom_ns_map = HashMap::new();

    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.to_lowercase().starts_with("error") {
            continue;
        }

        // Ignore solid color layers
        if trimmed.contains("currently displaying: color:") {
            continue;
        }

        // Extract path portion
        let (prefix, path_part) = if let Some((pfx, path)) = trimmed.split_once("currently displaying:") {
            let clean_path = path.trim().strip_prefix("image:").unwrap_or(path).trim();
            (pfx.trim(), clean_path)
        } else {
            // Short or comma-separated format
            let parts: Vec<&str> = trimmed.split(',').map(str::trim).collect();
            let last = parts.last().copied().unwrap_or(trimmed);
            (trimmed, last)
        };

        let Some(path) = extract_wallpaper_path_from_str(path_part) else {
            continue;
        };

        // Determine namespace and monitor name from prefix
        // Format examples for prefix:
        // 1. ": DP-1: 2560x1440, scale: 1," -> namespace is default (""), monitor is "DP-1"
        // 2. "test: DP-1: 2560x1440, scale: 1," -> namespace is "test", monitor is "DP-1"
        // 3. "DP-1: 2560x1440, scale: 1," -> namespace is default (""), monitor is "DP-1"
        // 4. "HDMI-A-1: /path" -> monitor is "HDMI-A-1"
        let (is_default_ns, monitor) = if prefix.starts_with(':') {
            let after = prefix.strip_prefix(':').unwrap_or(prefix).trim();
            let mon = after.split_once(':').map(|(m, _)| m.trim()).unwrap_or(after);
            (true, mon.to_string())
        } else {
            let parts: Vec<&str> = prefix.split(':').map(str::trim).collect();
            let is_geometry = parts.get(1).is_some_and(|p| {
                p.contains(',')
                    || p.contains("scale")
                    || (p.contains('x') && p.split('x').all(|s| s.trim().chars().any(|c| c.is_ascii_digit())))
            });
            if parts.len() >= 3 && !is_geometry && !parts[1].is_empty() {
                (false, parts[1].to_string())
            } else if let Some(first) = parts.first() && !first.is_empty() {
                (true, first.to_string())
            } else {
                continue;
            }
        };

        if is_default_ns {
            default_ns_map.insert(monitor, path);
        } else {
            custom_ns_map.insert(monitor, path);
        }
    }

    // Default namespace has precedence on conflict
    let mut map = custom_ns_map;
    for (mon, path) in default_ns_map {
        map.insert(mon, path);
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
/// Supports:
/// - Single-line legacy syntax: `wallpaper = DP-1,/path` or `wallpaper = ,/path`
/// - Modern block syntax: `wallpaper { monitor = DP-1 \n path = /path }`
///
/// Ignores `preload = ...` and comments starting with `#`.
#[must_use]
pub fn parse_hyprpaper_conf(content: &str) -> HashMap<String, PathBuf> {
    let mut map = HashMap::new();
    let mut in_wallpaper_block = false;
    let mut current_block_monitor: Option<String> = None;
    let mut current_block_path: Option<String> = None;

    for line in content.lines() {
        let line_clean = if let Some((before_hash, _)) = line.split_once('#') {
            before_hash
        } else {
            line
        };
        let trimmed = line_clean.trim();
        if trimmed.is_empty() {
            continue;
        }

        if in_wallpaper_block {
            if trimmed == "}" || trimmed.starts_with('}') {
                in_wallpaper_block = false;
                if let Some(path_str) = current_block_path.take() {
                    let mon = current_block_monitor.take().unwrap_or_else(|| "default".to_string());
                    let key = if mon.is_empty() { "default".to_string() } else { mon };
                    if let Some(path) = extract_wallpaper_path_from_str(&path_str) {
                        map.insert(key, path);
                    }
                }
                continue;
            }
            if let Some((k, v)) = trimmed.split_once('=') {
                let k = k.trim();
                let v = v.trim();
                if k == "monitor" {
                    current_block_monitor = Some(v.to_string());
                } else if k == "path" {
                    current_block_path = Some(v.to_string());
                }
            }
            continue;
        }

        if trimmed.starts_with("wallpaper") && trimmed.contains('{') {
            in_wallpaper_block = true;
            current_block_monitor = None;
            current_block_path = None;
            continue;
        }

        if let Some(rest) = trimmed.strip_prefix("wallpaper") {
            let rest = rest.trim();
            if let Some(after_eq) = rest.strip_prefix('=') {
                let after_eq = after_eq.trim();
                let (mon, path_str) = if let Some((m, p)) = after_eq.split_once(',') {
                    (m.trim(), p)
                } else {
                    ("default", after_eq)
                };
                let key = if mon.is_empty() { "default".to_string() } else { mon.to_string() };
                if let Some(path) = extract_wallpaper_path_from_str(path_str) {
                    map.insert(key, path);
                }
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
    use std::os::unix::fs::MetadataExt;

    let entries = std::fs::read_dir(proc_root)
        .map_err(|e| TermonetError::WallpaperNotFound(format!("Failed to read {}: {e}", proc_root.display())))?;

    let current_uid = std::fs::metadata("/proc/self")
        .map(|m| m.uid())
        .unwrap_or(1000);

    let mut outputs = HashMap::new();

    for entry in entries.flatten() {
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if !name_str.chars().all(|c| c.is_ascii_digit()) {
            continue;
        }

        // Verify process UID ownership matches current user
        if let Ok(meta) = entry.metadata()
            && meta.uid() != current_uid
        {
            continue;
        }

        let cmdline_path = entry.path().join("cmdline");
        let Ok(cmdline_bytes) = std::fs::read(&cmdline_path) else {
            continue;
        };

        if cmdline_bytes.is_empty() {
            continue;
        }

        // Fast-path rejection: check if the binary is swaybg or path ends with /swaybg
        let first_arg = cmdline_bytes.split(|&b| b == 0).next().unwrap_or(&[]);
        if !(first_arg == b"swaybg" || first_arg.ends_with(b"/swaybg")) {
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
        if exec == "swaybg" || exec.ends_with("/swaybg") {
            let proc_outputs = parse_swaybg_args(&args[1..]);
            for (mon, path) in proc_outputs {
                outputs.insert(mon, path);
            }
        }
    }

    if outputs.is_empty() {
        Err(TermonetError::WallpaperNotFound("No active swaybg process found in proc".into()))
    } else {
        Ok(MonitorWallpapers { outputs })
    }
}

/// Parses CLI arguments passed to `swaybg`.
/// Handles `-o <output> -i <path>`, `--output=<output> --image=<path>`, and `-i <path>`.
/// Ignores color arguments (`-c`, `--color`, `--color=...`).
#[must_use]
pub fn parse_swaybg_args(args: &[String]) -> HashMap<String, PathBuf> {
    let mut outputs = HashMap::new();
    let mut current_output = None;
    let mut i = 0;

    while i < args.len() {
        let arg = &args[i];
        if let Some(val) = arg.strip_prefix("--output=") {
            let mon = val.trim();
            current_output = Some(if mon == "*" { "default".to_string() } else { mon.to_string() });
        } else if arg == "-o" || arg == "--output" {
            if i + 1 < args.len() {
                let mon = args[i + 1].trim();
                current_output = Some(if mon == "*" { "default".to_string() } else { mon.to_string() });
                i += 1;
            }
        } else if let Some(val) = arg.strip_prefix("--image=") {
            if let Some(img_path) = extract_wallpaper_path_from_str(val) {
                let mon = current_output.take().unwrap_or_else(|| "default".to_string());
                outputs.insert(mon, img_path);
            }
        } else if arg == "-i" || arg == "--image" {
            if i + 1 < args.len() {
                if let Some(img_path) = extract_wallpaper_path_from_str(&args[i + 1]) {
                    let mon = current_output.take().unwrap_or_else(|| "default".to_string());
                    outputs.insert(mon, img_path);
                }
                i += 1;
            }
        } else if (arg == "-c" || arg == "--color") && i + 1 < args.len() {
            i += 1;
        }
        i += 1;
    }

    outputs
}

/// Parses output from `wpaperctl all-wallpapers`.
/// Format: `MONITOR: /path/to/wallpaper`
#[must_use]
pub fn parse_wpaperctl_output(raw: &str) -> HashMap<String, PathBuf> {
    let mut map = HashMap::new();
    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.to_lowercase().starts_with("error") {
            continue;
        }
        if let Some((mon, path_str)) = trimmed.split_once(':') {
            let mon = mon.trim();
            if let Some(path) = extract_wallpaper_path_from_str(path_str) {
                map.insert(mon.to_string(), path);
            }
        }
    }
    map
}

/// Queries active wallpapers from `wpaperd` by trying `wpaperctl all-wallpapers` first,
/// falling back to inspecting symlinks in its state/cache directory.
pub fn query_wpaperd() -> Result<MonitorWallpapers> {
    // 1. Try wpaperctl all-wallpapers CLI
    if let Ok(output) = Command::new("wpaperctl").arg("all-wallpapers").output()
        && output.status.success()
    {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let parsed = parse_wpaperctl_output(&stdout);
        if !parsed.is_empty() {
            return Ok(MonitorWallpapers { outputs: parsed });
        }
    }

    // 2. Fallback: inspect symlinks in state directory
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

    #[test]
    fn test_parse_awww_namespaces_and_color_skipping() {
        let raw = "\
test: DP-1: 2560x1440, scale: 1, currently displaying: color: 000000
test: HDMI-A-1: 960x540, scale: 2, currently displaying: color: 000000
: DP-1: 2560x1440, scale: 1, currently displaying: image: /home/user/Pictures/wall_dp.png
: HDMI-A-1: 960x540, scale: 2, currently displaying: image: /home/user/Pictures/wall_hdmi.gif
";
        let parsed = parse_awww_or_swww_output(raw);
        assert_eq!(parsed.get("DP-1"), Some(&PathBuf::from("/home/user/Pictures/wall_dp.png")));
        assert_eq!(parsed.get("HDMI-A-1"), Some(&PathBuf::from("/home/user/Pictures/wall_hdmi.gif")));
    }

    #[test]
    fn test_parse_hyprpaper_block_and_legacy_syntax() {
        let raw = "\
# Hyprpaper config sample
preload = /home/user/Pictures/preload.png

wallpaper {
    monitor = DP-1
    path = /home/user/Pictures/block_dp.png
}

wallpaper {
    monitor =
    path = /home/user/Pictures/default_wall.png
}
";
        let parsed = parse_hyprpaper_conf(raw);
        assert_eq!(parsed.get("DP-1"), Some(&PathBuf::from("/home/user/Pictures/block_dp.png")));
        assert_eq!(parsed.get("default"), Some(&PathBuf::from("/home/user/Pictures/default_wall.png")));
    }

    #[test]
    fn test_parse_wpaperctl_output() {
        let raw = "\
HDMI-A-1: /home/user/Pictures/wallp.gif
DP-1: /home/user/Pictures/wallp.png
";
        let parsed = parse_wpaperctl_output(raw);
        assert_eq!(parsed.get("DP-1"), Some(&PathBuf::from("/home/user/Pictures/wallp.png")));
        assert_eq!(parsed.get("HDMI-A-1"), Some(&PathBuf::from("/home/user/Pictures/wallp.gif")));
    }

    #[test]
    fn test_parse_swaybg_equals_and_colors() {
        let args = vec![
            "--output=DP-1".into(),
            "--image=/home/user/wall1.png".into(),
            "-c".into(),
            "#000000".into(),
            "-o".into(),
            "HDMI-A-1".into(),
            "-i".into(),
            "/home/user/wall2.png".into(),
        ];
        let parsed = parse_swaybg_args(&args);
        assert_eq!(parsed.get("DP-1"), Some(&PathBuf::from("/home/user/wall1.png")));
        assert_eq!(parsed.get("HDMI-A-1"), Some(&PathBuf::from("/home/user/wall2.png")));
    }
}
