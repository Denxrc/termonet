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

    /// Creates a wallpaper mapping containing a single wallpaper under the "default" key.
    #[must_use]
    pub fn single(path: PathBuf) -> Self {
        let mut outputs = HashMap::new();
        outputs.insert("default".into(), path);
        Self { outputs }
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

        // Deterministic fallback: sort monitor names alphabetically to avoid SipHash hashmap iteration randomness
        let mut sorted_keys: Vec<_> = self.outputs.keys().collect();
        sorted_keys.sort();
        sorted_keys.first().and_then(|k| self.outputs.get(*k)).cloned()
    }

    /// Checks if there are two or more monitors reporting different active wallpaper paths.
    #[must_use]
    pub fn has_divergent_wallpapers(&self) -> bool {
        if self.outputs.len() <= 1 {
            return false;
        }
        let mut vals = self.outputs.values();
        let Some(first) = vals.next() else {
            return false;
        };
        vals.any(|p| p != first)
    }

    /// Returns sorted (monitor_name, path) pairs in deterministic alphabetical order.
    #[must_use]
    pub fn sorted_outputs(&self) -> Vec<(String, PathBuf)> {
        let mut list: Vec<_> = self.outputs.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
        list.sort_by(|a, b| a.0.cmp(&b.0));
        list
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
    fn test_monitor_resolution_deterministic_fallback() {
        let mut monitors = MonitorWallpapers::new();
        monitors.outputs.insert("HDMI-A-1".into(), PathBuf::from("/path/to/wall_hdmi.png"));
        monitors.outputs.insert("DP-1".into(), PathBuf::from("/path/to/wall_dp.png"));

        // "DP-1" comes before "HDMI-A-1" alphabetically
        assert_eq!(
            monitors.resolve_primary(None),
            Some(PathBuf::from("/path/to/wall_dp.png"))
        );
    }

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

    #[test]
    fn test_has_divergent_wallpapers() {
        let empty = MonitorWallpapers::new();
        assert!(!empty.has_divergent_wallpapers());

        let mut single = MonitorWallpapers::new();
        single.outputs.insert("eDP-1".into(), PathBuf::from("/path/to/wall.png"));
        assert!(!single.has_divergent_wallpapers());

        let mut identical = MonitorWallpapers::new();
        identical.outputs.insert("DP-1".into(), PathBuf::from("/path/to/wall.png"));
        identical.outputs.insert("HDMI-A-1".into(), PathBuf::from("/path/to/wall.png"));
        assert!(!identical.has_divergent_wallpapers());

        let mut different = MonitorWallpapers::new();
        different.outputs.insert("DP-1".into(), PathBuf::from("/path/to/wall1.png"));
        different.outputs.insert("HDMI-A-1".into(), PathBuf::from("/path/to/wall2.png"));
        assert!(different.has_divergent_wallpapers());
    }

    #[test]
    fn test_sorted_outputs() {
        let mut monitors = MonitorWallpapers::new();
        monitors.outputs.insert("HDMI-A-1".into(), PathBuf::from("/path/b.png"));
        monitors.outputs.insert("DP-1".into(), PathBuf::from("/path/a.png"));
        monitors.outputs.insert("eDP-1".into(), PathBuf::from("/path/c.png"));

        let sorted = monitors.sorted_outputs();
        let names: Vec<_> = sorted.into_iter().map(|(n, _)| n).collect();
        assert_eq!(names, vec!["DP-1", "HDMI-A-1", "eDP-1"]);
    }

    #[test]
    fn test_stress_empty_and_massive_monitor_map() {
        // 1. Completely empty map
        let empty_monitors = MonitorWallpapers::new();
        assert_eq!(empty_monitors.resolve_primary(None), None);
        assert_eq!(empty_monitors.resolve_primary(Some("DP-1")), None);
        assert!(!empty_monitors.has_divergent_wallpapers());
        assert!(empty_monitors.sorted_outputs().is_empty());

        // 2. Single output
        let single = MonitorWallpapers::single(PathBuf::from("/tmp/single.png"));
        assert_eq!(single.resolve_primary(None), Some(PathBuf::from("/tmp/single.png")));
        assert!(!single.has_divergent_wallpapers());
        assert_eq!(single.sorted_outputs().len(), 1);

        // 3. Massive scale: 10,000 distinct monitors
        let mut massive_monitors = MonitorWallpapers::new();
        for i in 0..10_000 {
            massive_monitors.outputs.insert(
                format!("DISPLAY-{:05}", i),
                PathBuf::from(format!("/wallpapers/image_{:05}.png", i)),
            );
        }
        assert!(massive_monitors.has_divergent_wallpapers());
        let sorted = massive_monitors.sorted_outputs();
        assert_eq!(sorted.len(), 10_000);
        assert_eq!(sorted[0].0, "DISPLAY-00000");
        assert_eq!(sorted[9_999].0, "DISPLAY-09999");

        // Primary match on massive map
        assert_eq!(
            massive_monitors.resolve_primary(Some("DISPLAY-05555")),
            Some(PathBuf::from("/wallpapers/image_05555.png"))
        );
        // Non-existent monitor falls back deterministically to DISPLAY-00000
        assert_eq!(
            massive_monitors.resolve_primary(Some("DOES_NOT_EXIST")),
            Some(PathBuf::from("/wallpapers/image_00000.png"))
        );
    }

    #[test]
    fn test_stress_extreme_strings_and_empty_keys() {
        let mut monitors = MonitorWallpapers::new();
        // Empty key string
        monitors.outputs.insert("".to_string(), PathBuf::from("/empty_key.png"));
        // 100,000 character key name
        let huge_key = "M".repeat(100_000);
        monitors.outputs.insert(huge_key.clone(), PathBuf::from("/huge_key.png"));

        assert!(monitors.has_divergent_wallpapers());
        let sorted = monitors.sorted_outputs();
        assert_eq!(sorted.len(), 2);
        assert_eq!(sorted[0].0, "");
        assert_eq!(sorted[1].0, huge_key);
    }

    #[test]
    fn test_stress_concurrent_multimonitor_resolution() {
        use std::sync::Arc;
        use std::thread;

        let mut map = MonitorWallpapers::new();
        for i in 0..100 {
            map.outputs.insert(
                format!("SCREEN-{}", i),
                PathBuf::from(format!("/images/wall_{}.png", i)),
            );
        }
        let shared_map = Arc::new(map);

        let mut handles = Vec::new();
        for thread_idx in 0..50 {
            let map_clone = Arc::clone(&shared_map);
            let h = thread::spawn(move || {
                for iter in 0..100 {
                    let primary_req = format!("SCREEN-{}", (thread_idx * 2 + iter) % 100);
                    let resolved = map_clone.resolve_primary(Some(&primary_req));
                    assert!(resolved.is_some());
                    assert!(map_clone.has_divergent_wallpapers());
                    let sorted = map_clone.sorted_outputs();
                    assert_eq!(sorted.len(), 100);
                }
            });
            handles.push(h);
        }

        for h in handles {
            h.join().expect("Thread in concurrent stress test panicked!");
        }
    }
}
