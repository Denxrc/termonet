//! State persistence to avoid redundant executions.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
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
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let serialized = toml::to_string_pretty(self)?;
        fs::write(path, serialized)?;
        Ok(())
    }

    /// Checks if wallpaper has already been processed with the same path.
    #[must_use]
    pub fn is_up_to_date<P: AsRef<Path>>(&self, wallpaper_path: P) -> bool {
        let path_str = wallpaper_path.as_ref().to_string_lossy();
        self.last_wallpaper_path.as_deref() == Some(path_str.as_ref())
    }
}
