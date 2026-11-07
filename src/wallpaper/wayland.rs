//! Wallpaper query implementations for Wayland compositors and daemons.

use crate::error::{Result, TermonetError};
use crate::wallpaper::monitor::MonitorWallpapers;

/// Queries active wallpapers from `swww` or `awww` CLI.
pub fn query_swww() -> Result<MonitorWallpapers> {
    // Stub: To be implemented in v1.x via CLI query output parsing
    Err(TermonetError::WallpaperNotFound("swww query not yet implemented".into()))
}

/// Queries active wallpapers from `hyprpaper` via UNIX IPC socket.
pub fn query_hyprpaper() -> Result<MonitorWallpapers> {
    // Stub: To be implemented in v1.x via hyprpaper socket IPC
    Err(TermonetError::WallpaperNotFound("hyprpaper IPC not yet implemented".into()))
}

/// Queries active wallpapers from `wpaperd` by inspecting symlinks in its cache directory.
pub fn query_wpaperd() -> Result<MonitorWallpapers> {
    // Stub: To be implemented in v1.x via std::fs::read_link
    Err(TermonetError::WallpaperNotFound("wpaperd inspection not yet implemented".into()))
}

/// Queries active wallpapers from `swaybg` by inspecting process commandline args in `/proc`.
pub fn query_swaybg() -> Result<MonitorWallpapers> {
    // Stub: To be implemented in v1.x via /proc/<pid>/cmdline parsing
    Err(TermonetError::WallpaperNotFound("swaybg cmdline not yet implemented".into()))
}
