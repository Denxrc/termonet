//! State persistence to avoid redundant executions.

use crate::config::AppPaths;
use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

/// State cache tracking the last applied wallpaper.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct AppStateCache {
    pub last_wallpaper_path: Option<String>,
    pub last_wallpaper_hash: Option<String>,
}

impl AppStateCache {
    /// Loads the cached state from standard state location.
    pub fn load() -> Result<Self> {
        let path = AppPaths::state_file()?;
        Self::load_from_path(path)
    }

    /// Loads the cached state from a specific file path.
    pub fn load_from_path<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        if !path.exists() {
            return Ok(Self::default());
        }

        let content = fs::read_to_string(path)?;
        let cache = toml::from_str(&content)?;
        Ok(cache)
    }

    /// Saves the current state cache to disk.
    pub fn save(&self) -> Result<()> {
        let path = AppPaths::state_file()?;
        self.save_to_path(path)
    }

    /// Saves the current state cache to a specific file path.
    pub fn save_to_path<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }

        let serialized = toml::to_string_pretty(self)?;
        fs::write(path, serialized)?;
        Ok(())
    }

    /// Computes a lightweight file fingerprint based on file size and modification time.
    #[must_use]
    pub fn compute_hash<P: AsRef<Path>>(wallpaper_path: P) -> Option<String> {
        let p = wallpaper_path.as_ref();
        let meta = fs::metadata(p).ok()?;
        let len = meta.len();
        let mtime = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0);
        Some(format!("{len}:{mtime}"))
    }

    /// Updates cache with the given wallpaper path and its current content fingerprint.
    pub fn update<P: AsRef<Path>>(&mut self, wallpaper_path: P) {
        let p = wallpaper_path.as_ref();
        self.last_wallpaper_path = Some(p.display().to_string());
        self.last_wallpaper_hash = Self::compute_hash(p);
    }

    /// Checks if wallpaper has already been processed with the same path and contents.
    #[must_use]
    pub fn is_up_to_date<P: AsRef<Path>>(&self, wallpaper_path: P) -> bool {
        let p = wallpaper_path.as_ref();
        let path_str = p.to_string_lossy();
        let matches_path = self.last_wallpaper_path.as_deref() == Some(path_str.as_ref())
            || (self.last_wallpaper_path.as_ref().is_some_and(|last| {
                if let (Ok(p1), Ok(p2)) = (fs::canonicalize(p), fs::canonicalize(last)) {
                    p1 == p2
                } else {
                    false
                }
            }));

        if !matches_path {
            return false;
        }

        // If file exists on disk and we have a fingerprint check, ensure contents haven't changed
        if let Some(current_hash) = Self::compute_hash(p)
            && let Some(ref saved_hash) = self.last_wallpaper_hash
        {
            return saved_hash == &current_hash;
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_state_cache_roundtrip() -> Result<()> {
        let temp_dir =
            std::env::temp_dir().join(format!("termonet_test_state_{}", std::process::id()));
        let _ = fs::create_dir_all(&temp_dir);
        let file = temp_dir.join("state.toml");

        let cache = AppStateCache {
            last_wallpaper_path: Some("/home/user/wall.png".into()),
            last_wallpaper_hash: Some("abc123hash".into()),
        };

        cache.save_to_path(&file)?;

        let loaded = AppStateCache::load_from_path(&file)?;
        assert_eq!(loaded, cache);
        assert!(loaded.is_up_to_date("/home/user/wall.png"));
        assert!(!loaded.is_up_to_date("/home/user/other.png"));

        let _ = fs::remove_file(&file);
        let _ = fs::remove_dir(&temp_dir);
        Ok(())
    }

    #[test]
    fn test_state_cache_mtime_invalidation() -> Result<()> {
        let temp_dir =
            std::env::temp_dir().join(format!("termonet_test_state_mtime_{}", std::process::id()));
        let _ = fs::create_dir_all(&temp_dir);
        let img_file = temp_dir.join("current_wall.png");
        fs::write(&img_file, b"initial wallpaper content")?;

        let mut cache = AppStateCache::default();
        cache.update(&img_file);
        assert!(cache.is_up_to_date(&img_file));

        // Overwrite file with different size
        fs::write(
            &img_file,
            b"modified new wallpaper content with different size and data",
        )?;
        assert!(!cache.is_up_to_date(&img_file));

        // Update cache -> now up to date again
        cache.update(&img_file);
        assert!(cache.is_up_to_date(&img_file));

        let _ = fs::remove_file(&img_file);
        let _ = fs::remove_dir(&temp_dir);
        Ok(())
    }
}
