//! Multi-monitor wallpaper resolution and deduplication.

use std::collections::HashMap;
use std::path::PathBuf;

/// Wallpaper mapping across connected outputs/monitors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MonitorWallpapers {
    /// Mapping from monitor output name (e.g. "eDP-1") to wallpaper file path.
    pub outputs: HashMap<String, PathBuf>,
}

impl MonitorWallpapers {
    /// Creates a new empty wallpaper mapping.
    #[must_use]
    pub fn new() -> Self {
        Self {
            outputs: HashMap::new(),
        }
    }

    /// Resolves a single wallpaper path according to multi-monitor rules:
    /// 1. If all monitors use identical wallpapers, returns that path.
    /// 2. If monitors have different wallpapers, selects the specified `primary_monitor`.
    /// 3. If primary monitor is not found or not set, falls back to the first available monitor.
    #[must_use]
    pub fn resolve_primary(&self, primary_monitor: Option<&str>) -> Option<PathBuf> {
        if self.outputs.is_empty() {
            return None;
        }

        // Check if all monitors share the same wallpaper path
        let mut paths = self.outputs.values();
        if let Some(first) = paths.next()
            && paths.all(|p| p == first)
        {
            return Some(first.clone());
        }

        // Look for the configured primary monitor
        if let Some(primary) = primary_monitor
            && let Some(path) = self.outputs.get(primary)
        {
            return Some(path.clone());
        }

        // Check for general fallback key "default"
        if let Some(path) = self.outputs.get("default") {
            return Some(path.clone());
        }

        // Fallback to first available active monitor
        self.outputs.values().next().cloned()
    }
}

impl Default for MonitorWallpapers {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;

    #[test]
    fn test_monitor_resolution_identical() {
        let mut monitors = MonitorWallpapers::new();
        monitors.outputs.insert("DP-1".into(), PathBuf::from("/path/to/wall.png"));
        monitors.outputs.insert("HDMI-A-1".into(), PathBuf::from("/path/to/wall.png"));

        assert_eq!(
            monitors.resolve_primary(None),
            Some(PathBuf::from("/path/to/wall.png"))
        );
    }

    #[test]
    fn test_monitor_resolution_primary_match() {
        let mut monitors = MonitorWallpapers::new();
        monitors.outputs.insert("DP-1".into(), PathBuf::from("/path/to/wall1.png"));
        monitors.outputs.insert("HDMI-A-1".into(), PathBuf::from("/path/to/wall2.png"));

        assert_eq!(
            monitors.resolve_primary(Some("HDMI-A-1")),
            Some(PathBuf::from("/path/to/wall2.png"))
        );
    }

    #[test]
    fn test_monitor_resolution_fallback_to_default_key() {
        let mut monitors = MonitorWallpapers::new();
        monitors.outputs.insert("default".into(), PathBuf::from("/path/to/default.png"));

        assert_eq!(
            monitors.resolve_primary(Some("NonExistent")),
            Some(PathBuf::from("/path/to/default.png"))
        );
    }
}
