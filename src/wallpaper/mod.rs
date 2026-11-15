//! Wallpaper detection and extraction providers across Wayland, Desktop Environments, and X11.

pub mod de;
pub mod monitor;
pub mod path;
pub mod provider;
pub mod wayland;

pub use monitor::MonitorWallpapers;
pub use path::{clean_wallpaper_path, percent_decode, sanitize_wallpaper_path, validate_wallpaper_path};
pub use provider::{all_providers, WallpaperProvider};

use std::path::PathBuf;
use crate::config::WallpaperBackend;
use crate::error::{Result, TermonetError};

/// Resolves the current wallpaper path based on configured backend and multi-monitor policy.
pub fn get_current_wallpaper(
    backend: &WallpaperBackend,
    manual_path: Option<&str>,
    primary_monitor: Option<&str>,
) -> Result<PathBuf> {
    let monitors = get_wallpaper_monitors(backend, manual_path)?;
    monitors
        .resolve_primary(primary_monitor)
        .ok_or_else(|| {
            let backend_name = match backend {
                WallpaperBackend::Manual => "manual path",
                WallpaperBackend::Auto => "automatic detection",
                WallpaperBackend::Awww | WallpaperBackend::Swww => "awww/swww",
                WallpaperBackend::Hyprpaper => "hyprpaper",
                WallpaperBackend::Swaybg => "swaybg",
                WallpaperBackend::Wpaperd => "wpaperd",
                WallpaperBackend::Gnome => "GNOME",
                WallpaperBackend::Kde => "KDE Plasma",
                WallpaperBackend::Feh => "feh",
                WallpaperBackend::Nitrogen => "nitrogen",
            };
            TermonetError::WallpaperNotFound(format!("No active {} wallpaper detected", backend_name))
        })
}

/// Discovers wallpaper paths mapped per monitor for the specified backend without resolving a primary monitor.
pub fn get_wallpaper_monitors(
    backend: &WallpaperBackend,
    manual_path: Option<&str>,
) -> Result<MonitorWallpapers> {
    match backend {
        WallpaperBackend::Manual => {
            let path = manual_path.ok_or_else(|| {
                TermonetError::WallpaperNotFound("Manual wallpaper path not specified in configuration".into())
            })?;
            let p = crate::config::paths::AppPaths::expand_tilde(path);
            if p.exists() {
                Ok(MonitorWallpapers::single(p))
            } else {
                Err(TermonetError::WallpaperNotFound(format!("Manual wallpaper file not found: {}", path)))
            }
        }
        WallpaperBackend::Auto => query_auto_monitors(),
        WallpaperBackend::Awww | WallpaperBackend::Swww => wayland::query_awww_or_swww(),
        WallpaperBackend::Hyprpaper => wayland::query_hyprpaper(),
        WallpaperBackend::Swaybg => wayland::query_swaybg(),
        WallpaperBackend::Wpaperd => wayland::query_wpaperd(),
        WallpaperBackend::Gnome => de::query_gnome(),
        WallpaperBackend::Kde => de::query_kde(),
        WallpaperBackend::Feh => provider::FehProvider.query(),
        WallpaperBackend::Nitrogen => provider::NitrogenProvider.query(),
    }
}

/// Automatically detects all active monitors and wallpapers by concurrently querying supported backends.
pub fn query_auto_monitors() -> Result<MonitorWallpapers> {
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
            let h_gnome = if is_gnome {
                Some(s.spawn(de::query_gnome))
            } else {
                None
            };
            let h_kde = if is_kde {
                Some(s.spawn(de::query_kde))
            } else {
                None
            };

            (
                h_awww.join().unwrap_or_else(|_| Err(TermonetError::WallpaperNotFound("awww thread panicked".into()))),
                h_hypr.join().unwrap_or_else(|_| Err(TermonetError::WallpaperNotFound("hyprpaper thread panicked".into()))),
                h_swaybg.join().unwrap_or_else(|_| Err(TermonetError::WallpaperNotFound("swaybg thread panicked".into()))),
                h_wpaperd.join().unwrap_or_else(|_| Err(TermonetError::WallpaperNotFound("wpaperd thread panicked".into()))),
                h_gnome.map(|h| h.join().unwrap_or_else(|_| Err(TermonetError::WallpaperNotFound("gnome thread panicked".into()))))
                    .unwrap_or_else(|| Err(TermonetError::WallpaperNotFound("Not in GNOME session".into()))),
                h_kde.map(|h| h.join().unwrap_or_else(|_| Err(TermonetError::WallpaperNotFound("kde thread panicked".into()))))
                    .unwrap_or_else(|| Err(TermonetError::WallpaperNotFound("Not in KDE session".into()))),
            )
        });

    // Determine candidate evaluation order based on active session
    let prioritized: Vec<Result<MonitorWallpapers>> = if is_gnome {
        vec![res_gnome, res_awww, res_hypr, res_swaybg, res_wpaperd, res_kde]
    } else if is_kde {
        vec![res_kde, res_awww, res_hypr, res_swaybg, res_wpaperd, res_gnome]
    } else if is_hyprland {
        vec![res_awww, res_hypr, res_swaybg, res_wpaperd]
    } else if is_sway {
        vec![res_awww, res_swaybg, res_wpaperd, res_hypr]
    } else {
        vec![res_awww, res_hypr, res_swaybg, res_wpaperd, res_gnome, res_kde]
    };

    for monitors in prioritized.into_iter().flatten() {
        if monitors.outputs.values().any(|p| p.exists()) {
            return Ok(monitors);
        }
    }

    // Secondary fallbacks for X11 tools (feh, nitrogen)
    if let Ok(monitors) = provider::FehProvider.query()
        && monitors.outputs.values().any(|p| p.exists())
    {
        return Ok(monitors);
    }
    if let Ok(monitors) = provider::NitrogenProvider.query()
        && monitors.outputs.values().any(|p| p.exists())
    {
        return Ok(monitors);
    }

    Err(TermonetError::WallpaperNotFound(
        "Automatic wallpaper detection failed: no active wallpapers found from awww/swww, hyprpaper, swaybg, wpaperd, GNOME, KDE, feh, or nitrogen".into(),
    ))
}

/// Automatically detects the active wallpaper by concurrently querying supported backends.
/// Dispatches parallel queries across worker threads and evaluates results according
/// to the detected compositor and desktop environment context.
pub fn query_auto(primary_monitor: Option<&str>) -> Result<PathBuf> {
    query_auto_monitors()?.resolve_primary(primary_monitor)
        .ok_or_else(|| TermonetError::WallpaperNotFound("No active wallpaper detected from automatic backends".into()))
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

    #[test]
    fn test_get_wallpaper_monitors_manual_success() {
        let temp_file = std::env::temp_dir().join("termonet_test_monitors_manual.png");
        std::fs::write(&temp_file, b"fake").unwrap();

        let resolved = get_wallpaper_monitors(
            &WallpaperBackend::Manual,
            Some(temp_file.to_str().unwrap()),
        );
        assert!(resolved.is_ok());
        let monitors = resolved.unwrap();
        assert_eq!(monitors.outputs.get("default"), Some(&temp_file));
        assert!(!monitors.has_divergent_wallpapers());

        let _ = std::fs::remove_file(temp_file);
    }

    #[test]
    fn test_get_wallpaper_monitors_manual_none() {
        let resolved = get_wallpaper_monitors(&WallpaperBackend::Manual, None);
        assert!(resolved.is_err());
    }

    #[test]
    fn test_stress_daemon_offline_queries() {
        let offline_backends = [
            WallpaperBackend::Swww,
            WallpaperBackend::Hyprpaper,
            WallpaperBackend::Awww,
            WallpaperBackend::Wpaperd,
            WallpaperBackend::Swaybg,
            WallpaperBackend::Gnome,
            WallpaperBackend::Kde,
            WallpaperBackend::Feh,
            WallpaperBackend::Nitrogen,
        ];

        for backend in offline_backends {
            let res = get_wallpaper_monitors(&backend, None);
            match res {
                Ok(monitors) => {
                    assert!(!monitors.outputs.is_empty(), "Resolved monitors map must not be empty");
                }
                Err(TermonetError::WallpaperNotFound(msg)) => {
                    assert!(!msg.is_empty(), "Error message must not be empty");
                }
                Err(other) => {
                    panic!("Daemon query must return WallpaperNotFound when offline, got: {other:?}");
                }
            }

            let curr_res = get_current_wallpaper(&backend, None, None);
            match curr_res {
                Ok(path) => {
                    assert!(!path.as_os_str().is_empty(), "Resolved wallpaper path must not be empty");
                }
                Err(TermonetError::WallpaperNotFound(msg)) => {
                    assert!(!msg.is_empty(), "Error message must not be empty");
                }
                Err(other) => {
                    panic!("Current wallpaper query must return WallpaperNotFound when offline, got: {other:?}");
                }
            }
        }
    }

    #[test]
    fn test_stress_concurrent_auto_query_and_sanitizer() {
        use std::thread;

        let mut handles = Vec::new();
        for thread_idx in 0..30 {
            let h = thread::spawn(move || {
                for iter in 0..50 {
                    let candidate = format!("   file:///home/user/wallpapers/image_{}_{}.png (fit: contain)   ", thread_idx, iter);
                    let sanitized = sanitize_wallpaper_path(&candidate);
                    assert!(sanitized.is_some_and(|p| p.to_string_lossy().starts_with("/home/user/wallpapers/image_")));

                    let manual_file = format!("/tmp/non_existent_manual_{}_{}.png", thread_idx, iter);
                    let _ = get_wallpaper_monitors(&WallpaperBackend::Manual, Some(&manual_file));
                }
            });
            handles.push(h);
        }

        for h in handles {
            h.join().expect("Concurrent sanitization thread panicked!");
        }
    }
}
