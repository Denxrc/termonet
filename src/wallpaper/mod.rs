//! Wallpaper detection and extraction providers across Wayland and X11.

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
            let p = PathBuf::from(path);
            if p.exists() {
                Ok(p)
            } else {
                Err(TermonetError::WallpaperNotFound(format!("Manual wallpaper file not found: {}", path)))
            }
        }
        WallpaperBackend::Swww => {
            let monitors = wayland::query_swww()?;
            monitors.resolve_primary(primary_monitor)
                .ok_or_else(|| TermonetError::WallpaperNotFound("No active swww wallpaper detected".into()))
        }
        WallpaperBackend::Hyprpaper => {
            let monitors = wayland::query_hyprpaper()?;
            monitors.resolve_primary(primary_monitor)
                .ok_or_else(|| TermonetError::WallpaperNotFound("No active hyprpaper wallpaper detected".into()))
        }
        WallpaperBackend::Auto => {
            // In v0.x, auto defaults to error unless manual path is given, full auto detection comes in v1.x
            Err(TermonetError::WallpaperNotFound("Automatic wallpaper detection will be activated in v1.x. Please provide a manual path.".into()))
        }
        _ => Err(TermonetError::WallpaperNotFound("Specified wallpaper backend is not yet supported".into())),
    }
}
