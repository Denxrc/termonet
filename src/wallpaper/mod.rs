//! Wallpaper detection and extraction providers across Wayland, Desktop Environments, and X11.

pub mod de;
pub mod monitor;
pub mod wayland;

pub use monitor::MonitorWallpapers;

use std::path::PathBuf;
use crate::config::WallpaperBackend;
use crate::error::{Result, TermonetError};

/// Resolves the current wallpaper path based on configured backend and multi-monitor policy.
pub fn get_current_wallpaper(
    backend: &WallpaperBackend,
    manual_path: Option<&str>,
    primary_monitor: Option<&str>,
) -> Result<PathBuf> {
    match backend {
        WallpaperBackend::Manual => {
            let path = manual_path.ok_or_else(|| {
                TermonetError::WallpaperNotFound("Manual wallpaper path not specified in configuration".into())
            })?;
            let p = crate::config::paths::AppPaths::expand_tilde(path);
            if p.exists() {
                Ok(p)
            } else {
                Err(TermonetError::WallpaperNotFound(format!("Manual wallpaper file not found: {}", path)))
            }
        }
        WallpaperBackend::Auto => query_auto(primary_monitor),
        WallpaperBackend::Awww | WallpaperBackend::Swww => {
            let monitors = wayland::query_awww_or_swww()?;
            monitors.resolve_primary(primary_monitor)
                .ok_or_else(|| TermonetError::WallpaperNotFound("No active awww/swww wallpaper detected".into()))
        }
        WallpaperBackend::Hyprpaper => {
            let monitors = wayland::query_hyprpaper()?;
            monitors.resolve_primary(primary_monitor)
                .ok_or_else(|| TermonetError::WallpaperNotFound("No active hyprpaper wallpaper detected".into()))
        }
        WallpaperBackend::Swaybg => {
            let monitors = wayland::query_swaybg()?;
            monitors.resolve_primary(primary_monitor)
                .ok_or_else(|| TermonetError::WallpaperNotFound("No active swaybg wallpaper detected".into()))
        }
        WallpaperBackend::Wpaperd => {
            let monitors = wayland::query_wpaperd()?;
            monitors.resolve_primary(primary_monitor)
                .ok_or_else(|| TermonetError::WallpaperNotFound("No active wpaperd wallpaper detected".into()))
        }
        WallpaperBackend::Gnome => {
            let monitors = de::query_gnome()?;
            monitors.resolve_primary(primary_monitor)
                .ok_or_else(|| TermonetError::WallpaperNotFound("No active GNOME wallpaper detected".into()))
        }
        WallpaperBackend::Kde => {
            let monitors = de::query_kde()?;
            monitors.resolve_primary(primary_monitor)
                .ok_or_else(|| TermonetError::WallpaperNotFound("No active KDE Plasma wallpaper detected".into()))
        }
        WallpaperBackend::Feh => {
            if let Some(home) = std::env::var_os("HOME").map(PathBuf::from) {
                let fehbg = home.join(".fehbg");
                if fehbg.exists()
                    && let Ok(content) = std::fs::read_to_string(&fehbg)
                {
                    for token in content.split_whitespace() {
                        let clean = token.trim_matches(|c| c == '\'' || c == '"');
                        if clean.starts_with('/') || clean.starts_with('~') {
                            let p = crate::config::paths::AppPaths::expand_tilde(clean);
                            if p.exists() {
                                return Ok(p);
                            }
                        }
                    }
                }
            }
            Err(TermonetError::WallpaperNotFound("No active feh wallpaper found in ~/.fehbg".into()))
        }
        WallpaperBackend::Nitrogen => {
            if let Some(home) = std::env::var_os("HOME").map(PathBuf::from) {
                let nitrogen_cfg = home.join(".config/nitrogen/bg-saved.cfg");
                if nitrogen_cfg.exists()
                    && let Ok(content) = std::fs::read_to_string(&nitrogen_cfg)
                {
                    for line in content.lines() {
                        if let Some((k, v)) = line.split_once('=')
                            && k.trim() == "file"
                        {
                            let p = crate::config::paths::AppPaths::expand_tilde(v.trim());
                            if p.exists() {
                                return Ok(p);
                            }
                        }
                    }
                }
            }
            Err(TermonetError::WallpaperNotFound(
                "No active nitrogen wallpaper found in ~/.config/nitrogen/bg-saved.cfg".into(),
            ))
        }
    }
}

/// Automatically detects the active wallpaper by concurrently querying supported backends.
/// Dispatches parallel queries across worker threads and evaluates results according
/// to the detected compositor and desktop environment context.
pub fn query_auto(primary_monitor: Option<&str>) -> Result<PathBuf> {
    let desktop = std::env::var("XDG_CURRENT_DESKTOP")
        .unwrap_or_default()
        .to_uppercase();
    let is_gnome = desktop.contains("GNOME");
    let is_kde = desktop.contains("KDE");
    let is_hyprland = std::env::var_os("HYPRLAND_INSTANCE_SIGNATURE").is_some() || desktop.contains("HYPRLAND");
    let is_sway = std::env::var_os("SWAYSOCK").is_some() || desktop.contains("SWAY");

    // Execute backend detectors concurrently in parallel threads
    let (res_awww, res_hypr, res_swaybg, res_wpaperd, res_gnome, res_kde) =
        std::thread::scope(|s| {
            let h_awww = s.spawn(wayland::query_awww_or_swww);
            let h_hypr = s.spawn(wayland::query_hyprpaper);
            let h_swaybg = s.spawn(wayland::query_swaybg);
            let h_wpaperd = s.spawn(wayland::query_wpaperd);
            let h_gnome = s.spawn(|| {
                // Only query GNOME if session indicates GNOME or as fallback
                if is_gnome {
                    de::query_gnome()
                } else {
                    Err(TermonetError::WallpaperNotFound("Not in GNOME session".into()))
                }
            });
            let h_kde = s.spawn(|| {
                // Only query KDE if session indicates KDE
                if is_kde {
                    de::query_kde()
                } else {
                    Err(TermonetError::WallpaperNotFound("Not in KDE session".into()))
                }
            });

            (
                h_awww.join().unwrap_or_else(|_| Err(TermonetError::WallpaperNotFound("awww thread panicked".into()))),
                h_hypr.join().unwrap_or_else(|_| Err(TermonetError::WallpaperNotFound("hyprpaper thread panicked".into()))),
                h_swaybg.join().unwrap_or_else(|_| Err(TermonetError::WallpaperNotFound("swaybg thread panicked".into()))),
                h_wpaperd.join().unwrap_or_else(|_| Err(TermonetError::WallpaperNotFound("wpaperd thread panicked".into()))),
                h_gnome.join().unwrap_or_else(|_| Err(TermonetError::WallpaperNotFound("gnome thread panicked".into()))),
                h_kde.join().unwrap_or_else(|_| Err(TermonetError::WallpaperNotFound("kde thread panicked".into()))),
            )
        });

    // Determine candidate evaluation order based on active session
    let prioritized: Vec<&Result<MonitorWallpapers>> = if is_gnome {
        vec![&res_gnome, &res_awww, &res_hypr, &res_swaybg, &res_wpaperd, &res_kde]
    } else if is_kde {
        vec![&res_kde, &res_awww, &res_hypr, &res_swaybg, &res_wpaperd, &res_gnome]
    } else if is_hyprland {
        vec![&res_awww, &res_hypr, &res_swaybg, &res_wpaperd]
    } else if is_sway {
        vec![&res_awww, &res_swaybg, &res_wpaperd, &res_hypr]
    } else {
        vec![&res_awww, &res_hypr, &res_swaybg, &res_wpaperd, &res_gnome, &res_kde]
    };

    for candidate in prioritized {
        if let Ok(monitors) = candidate
            && let Some(path) = monitors.resolve_primary(primary_monitor)
            && path.exists()
        {
            return Ok(path);
        }
    }

    // Secondary fallbacks for X11 tools (feh, nitrogen)
    if let Ok(path) = get_current_wallpaper(&WallpaperBackend::Feh, None, primary_monitor) {
        return Ok(path);
    }
    if let Ok(path) = get_current_wallpaper(&WallpaperBackend::Nitrogen, None, primary_monitor) {
        return Ok(path);
    }

    Err(TermonetError::WallpaperNotFound(
        "Automatic wallpaper detection failed: no active wallpapers found from awww/swww, hyprpaper, swaybg, wpaperd, GNOME, KDE, feh, or nitrogen".into(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_manual_wallpaper_resolution_success() {
        let temp_file = std::env::temp_dir().join("termonet_test_manual_wall.png");
        std::fs::write(&temp_file, b"fake").unwrap();

        let resolved = get_current_wallpaper(
            &WallpaperBackend::Manual,
            Some(temp_file.to_str().unwrap()),
            None,
        );
        assert!(resolved.is_ok());
        assert_eq!(resolved.unwrap(), temp_file);

        let _ = std::fs::remove_file(temp_file);
    }

    #[test]
    fn test_manual_wallpaper_resolution_missing() {
        let resolved = get_current_wallpaper(
            &WallpaperBackend::Manual,
            Some("/non/existent/wallpaper_file.png"),
            None,
        );
        assert!(resolved.is_err());
    }
}
