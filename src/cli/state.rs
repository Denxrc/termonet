//! State persistence to avoid redundant executions.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use crate::config::AppPaths;
use crate::error::Result;

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

    /// Checks if wallpaper has already been processed with the same path.
    #[must_use]
    pub fn is_up_to_date<P: AsRef<Path>>(&self, wallpaper_path: P) -> bool {
        let path_str = wallpaper_path.as_ref().to_string_lossy();
        if self.last_wallpaper_path.as_deref() == Some(path_str.as_ref()) {
            return true;
        }

        // Also compare canonicalized paths if accessible
        if let Some(ref last) = self.last_wallpaper_path
            && let (Ok(p1), Ok(p2)) = (
                fs::canonicalize(wallpaper_path.as_ref()),
                fs::canonicalize(PathBuf::from(last)),
            )
        {
            return p1 == p2;
        }

        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_state_cache_roundtrip() {
        let temp_dir = std::env::temp_dir().join("termonet_test_state");
        let _ = fs::create_dir_all(&temp_dir);
        let file = temp_dir.join("state.toml");

        let cache = AppStateCache {
            last_wallpaper_path: Some("/home/user/wall.png".into()),
            last_wallpaper_hash: Some("abc123hash".into()),
        };

        assert!(cache.save_to_path(&file).is_ok());

        let loaded = AppStateCache::load_from_path(&file).unwrap();
        assert_eq!(loaded, cache);
        assert!(loaded.is_up_to_date("/home/user/wall.png"));
        assert!(!loaded.is_up_to_date("/home/user/other.png"));

        let _ = fs::remove_file(file);
    }
}
