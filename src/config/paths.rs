//! Standard filesystem and XDG path resolutions for Termonet.

use std::path::PathBuf;
use directories::ProjectDirs;
use crate::error::{Result, TermonetError};

/// Resolves standard XDG paths for configuration, state, and data.
pub struct AppPaths;

impl AppPaths {
    fn project_dirs() -> Result<ProjectDirs> {
        ProjectDirs::from("", "", "termonet")
            .ok_or_else(|| TermonetError::Config("Unable to determine user home or XDG directories".into()))
    }

    /// Returns the path to the configuration directory (`~/.config/termonet/`).
    pub fn config_dir() -> Result<PathBuf> {
        Ok(Self::project_dirs()?.config_dir().to_path_buf())
    }

    /// Returns the path to the main configuration file (`~/.config/termonet/config.toml`).
    pub fn config_file() -> Result<PathBuf> {
        Ok(Self::config_dir()?.join("config.toml"))
    }

    /// Returns the path to the user templates directory (`~/.config/termonet/templates/`).
    pub fn templates_dir() -> Result<PathBuf> {
        Ok(Self::config_dir()?.join("templates"))
    }

    /// Returns the path to the user rules directory (`~/.config/termonet/rules/`).
    pub fn rules_dir() -> Result<PathBuf> {
        Ok(Self::config_dir()?.join("rules"))
    }

    /// Returns the path to the state directory (`~/.local/state/termonet/`).
    pub fn state_dir() -> Result<PathBuf> {
        let dirs = Self::project_dirs()?;
        let path = dirs.state_dir().unwrap_or_else(|| dirs.data_dir());
        Ok(path.to_path_buf())
    }

    /// Returns the path to the state cache file storing the last applied wallpaper hash.
    pub fn state_file() -> Result<PathBuf> {
        Ok(Self::state_dir()?.join("state.toml"))
    }

    /// Expands `~` prefix to the current user's home directory.
    #[must_use]
    pub fn expand_tilde(path: &str) -> PathBuf {
        let trimmed = path.trim();
        if let Some(stripped) = trimmed.strip_prefix("~/") {
            if let Ok(home) = std::env::var("HOME") {
                return PathBuf::from(home).join(stripped);
            }
        } else if trimmed == "~"
            && let Ok(home) = std::env::var("HOME")
        {
            return PathBuf::from(home);
        }
        PathBuf::from(trimmed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_expand_tilde() {
        let regular = AppPaths::expand_tilde("/usr/share/backgrounds/default.png");
        assert_eq!(regular, PathBuf::from("/usr/share/backgrounds/default.png"));

        if let Ok(home) = std::env::var("HOME") {
            let tilde = AppPaths::expand_tilde("~/wallpapers/nature.jpg");
            assert_eq!(tilde, PathBuf::from(&home).join("wallpapers/nature.jpg"));

            let bare_tilde = AppPaths::expand_tilde("~");
            assert_eq!(bare_tilde, PathBuf::from(&home));
        }
    }
}
