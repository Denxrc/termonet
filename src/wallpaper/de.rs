//! Wallpaper query implementations for Desktop Environments (GNOME, KDE Plasma).

use crate::error::{Result, TermonetError};
use crate::wallpaper::monitor::MonitorWallpapers;

/// Queries active wallpaper in GNOME desktop via `gsettings`.
pub fn query_gnome() -> Result<MonitorWallpapers> {
    // Stub: To be implemented in v1.x via gsettings check of color-scheme and picture-uri
    Err(TermonetError::WallpaperNotFound("GNOME gsettings query not yet implemented".into()))
}

/// Queries active wallpaper in KDE Plasma via config parsing.
pub fn query_kde() -> Result<MonitorWallpapers> {
    // Stub: To be implemented in v1.x via plasma-org.kde.plasma.desktop-appletsrc parsing
    Err(TermonetError::WallpaperNotFound("KDE config query not yet implemented".into()))
}
