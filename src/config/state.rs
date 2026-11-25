//! State persistence to avoid redundant executions.

use crate::config::AppPaths;
use crate::core::color::RgbColor;
use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

/// Source of color palette generation for state caching.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CacheSource<'a> {
    Wallpaper(&'a Path),
    Color(RgbColor),
}

/// State cache tracking the last applied wallpaper, theme mode, and config fingerprint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct AppStateCache {
    pub last_wallpaper_path: Option<String>,
    pub last_wallpaper_hash: Option<String>,
    #[serde(default)]
    pub last_seed_color: Option<String>,
    #[serde(default)]
    pub last_is_dark: Option<bool>,
    #[serde(default)]
    pub last_config_fingerprint: Option<String>,
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

    /// Saves the current state cache to a specific file path atomically.
    pub fn save_to_path<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let path = path.as_ref();
        let serialized = toml::to_string_pretty(self)?;
        crate::template::safe_write(path, &serialized, false)
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
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        Some(format!("{len}:{mtime}"))
    }

    /// Updates cache with the given wallpaper path and its current content fingerprint.
    pub fn update<P: AsRef<Path>>(&mut self, wallpaper_path: P) {
        let p = wallpaper_path.as_ref();
        self.last_wallpaper_path = Some(p.display().to_string());
        self.last_wallpaper_hash = Self::compute_hash(p);
    }

    /// Updates cache with a specific source, theme mode, and seed color.
    pub fn update_source(&mut self, source: &CacheSource, is_dark: bool, seed_color: &RgbColor) {
        self.update_source_with_fingerprint(source, is_dark, seed_color, None);
    }

    /// Updates cache with a specific source, theme mode, seed color, and config/rules fingerprint.
    pub fn update_source_with_fingerprint(
        &mut self,
        source: &CacheSource,
        is_dark: bool,
        seed_color: &RgbColor,
        config_fingerprint: Option<String>,
    ) {
        self.last_is_dark = Some(is_dark);
        self.last_seed_color = Some(seed_color.to_hex());
        self.last_config_fingerprint = config_fingerprint;
        match source {
            CacheSource::Wallpaper(p) => {
                self.last_wallpaper_path = Some(p.display().to_string());
                self.last_wallpaper_hash = Self::compute_hash(p);
            }
            CacheSource::Color(_) => {
                self.last_wallpaper_path = None;
                self.last_wallpaper_hash = None;
            }
        }
    }

    /// Computes a deterministic FNV-1a fingerprint of enabled programs and on-disk rules/templates.
    #[must_use]
    pub fn compute_config_fingerprint(
        enabled_programs: &[String],
        rules_dir: Option<&Path>,
        templates_dir: Option<&Path>,
    ) -> String {
        let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
        let mix = |h: &mut u64, bytes: &[u8]| {
            for &b in bytes {
                *h ^= u64::from(b);
                *h = h.wrapping_mul(0x0100_0000_01b3);
            }
            *h ^= 0xff;
            *h = h.wrapping_mul(0x0100_0000_01b3);
        };

        let mut progs: Vec<String> = enabled_programs.iter().map(|p| p.to_lowercase()).collect();
        progs.sort();
        for p in &progs {
            mix(&mut hash, p.as_bytes());
        }

        let hash_dir = |dir: &Path, h: &mut u64| {
            if let Ok(entries) = fs::read_dir(dir) {
                let mut paths: Vec<_> = entries
                    .filter_map(std::result::Result::ok)
                    .map(|e| e.path())
                    .filter(|p| p.is_file())
                    .collect();
                paths.sort();
                for p in paths {
                    if let Some(name) = p.file_name().and_then(|n| n.to_str()) {
                        mix(h, name.as_bytes());
                    }
                    if let Some(file_hash) = Self::compute_hash(&p) {
                        mix(h, file_hash.as_bytes());
                    }
                }
            }
        };

        if let Some(r_dir) = rules_dir {
            mix(&mut hash, b"rules");
            hash_dir(r_dir, &mut hash);
        }
        if let Some(t_dir) = templates_dir {
            mix(&mut hash, b"templates");
            hash_dir(t_dir, &mut hash);
        }

        format!("{hash:016x}")
    }

    /// Checks if source, theme mode, and optional config fingerprint match the cached state.
    #[must_use]
    pub fn is_up_to_date_with_config(
        &self,
        source: &CacheSource,
        current_is_dark: bool,
        config_fingerprint: Option<&str>,
    ) -> bool {
        if let Some(fp) = config_fingerprint
            && self.last_config_fingerprint.as_deref() != Some(fp)
        {
            return false;
        }
        self.is_up_to_date(source, current_is_dark)
    }

    /// Checks if source and theme mode match the cached state.
    #[must_use]
    pub fn is_up_to_date(&self, source: &CacheSource, current_is_dark: bool) -> bool {
        if self.last_is_dark != Some(current_is_dark) {
            return false;
        }

        match source {
            CacheSource::Wallpaper(p) => {
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

                // Require a saved fingerprint and an existing readable file with matching size+mtime.
                match (self.last_wallpaper_hash.as_ref(), Self::compute_hash(p)) {
                    (Some(saved), Some(current)) => saved == &current,
                    _ => false,
                }
            }
            CacheSource::Color(c) => {
                if self.last_wallpaper_path.is_some() {
                    return false;
                }
                self.last_seed_color
                    .as_deref()
                    .is_some_and(|s| s.eq_ignore_ascii_case(&c.to_hex()))
            }
        }
    }

    /// Backward-compatible check for wallpaper paths alone.
    #[must_use]
    pub fn is_up_to_date_path<P: AsRef<Path>>(&self, wallpaper_path: P) -> bool {
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

        match (self.last_wallpaper_hash.as_ref(), Self::compute_hash(p)) {
            (Some(saved), Some(current)) => saved == &current,
            (None, Some(_)) => true,
            _ => false,
        }
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
        let wall_file = temp_dir.join("wall.png");
        fs::write(&wall_file, b"test_wallpaper_content")?;

        let hash = AppStateCache::compute_hash(&wall_file);
        let cache = AppStateCache {
            last_wallpaper_path: Some(wall_file.display().to_string()),
            last_wallpaper_hash: hash,
            last_seed_color: Some("#385e72".into()),
            last_is_dark: Some(true),
            last_config_fingerprint: None,
        };

        cache.save_to_path(&file)?;

        let loaded = AppStateCache::load_from_path(&file)?;
        assert_eq!(loaded, cache);
        assert!(loaded.is_up_to_date(&CacheSource::Wallpaper(&wall_file), true));
        assert!(!loaded.is_up_to_date(
            &CacheSource::Wallpaper(Path::new("/home/user/other.png")),
            true
        ));

        let _ = fs::remove_file(&file);
        let _ = fs::remove_file(&wall_file);
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
        let source = CacheSource::Wallpaper(&img_file);
        cache.update_source(&source, true, &RgbColor::new(0, 0, 0));
        assert!(cache.is_up_to_date(&source, true));

        // Overwrite file with different size
        fs::write(
            &img_file,
            b"modified new wallpaper content with different size and data",
        )?;
        assert!(!cache.is_up_to_date(&source, true));

        // Update cache -> now up to date again
        cache.update_source(&source, true, &RgbColor::new(0, 0, 0));
        assert!(cache.is_up_to_date(&source, true));

        let _ = fs::remove_file(&img_file);
        let _ = fs::remove_dir(&temp_dir);
        Ok(())
    }

    #[test]
    fn test_cache_invalidation_on_theme_mode_switch() {
        let mut cache = AppStateCache::default();
        let wall =
            std::env::temp_dir().join(format!("termonet_test_wall_{}.png", std::process::id()));
        fs::write(&wall, b"fake_png_data").unwrap();

        let source = CacheSource::Wallpaper(&wall);
        cache.update_source(&source, true, &crate::core::RgbColor::new(0x38, 0x5e, 0x72));

        assert!(cache.is_up_to_date(&source, true));
        // Toggling to light mode (is_dark = false) must invalidate cache!
        assert!(!cache.is_up_to_date(&source, false));

        // Deleting the file on disk must also invalidate cache
        let _ = fs::remove_file(&wall);
        assert!(!cache.is_up_to_date(&source, true));
    }

    #[test]
    fn test_cache_source_switching_from_wallpaper_to_color() {
        let mut cache = AppStateCache::default();
        let wall = std::env::temp_dir().join(format!("termonet_wall2_{}.png", std::process::id()));
        fs::write(&wall, b"data").unwrap();

        let wall_source = CacheSource::Wallpaper(&wall);
        let color = crate::core::RgbColor::new(0x11, 0x22, 0x33);
        cache.update_source(&wall_source, true, &color);

        // Switching to explicit standalone color must invalidate wallpaper cache
        let color_source = CacheSource::Color(color);
        assert!(!cache.is_up_to_date(&color_source, true));

        // Update with color source
        cache.update_source(&color_source, true, &color);
        assert!(cache.is_up_to_date(&color_source, true));
        assert_eq!(cache.last_wallpaper_path, None);
        assert_eq!(cache.last_wallpaper_hash, None);
        assert_eq!(cache.last_seed_color.as_deref(), Some("#112233"));

        let _ = fs::remove_file(&wall);
    }

    #[test]
    fn test_cache_invalidates_when_hash_missing_or_config_changes() {
        let temp_dir =
            std::env::temp_dir().join(format!("termonet_test_state_fp_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        let rules_dir = temp_dir.join("rules");
        let tpl_dir = temp_dir.join("templates");
        fs::create_dir_all(&rules_dir).unwrap();
        fs::create_dir_all(&tpl_dir).unwrap();

        let wall = temp_dir.join("wall.png");
        fs::write(&wall, b"img").unwrap();
        let source = CacheSource::Wallpaper(&wall);
        let color = RgbColor::new(0x11, 0x22, 0x33);

        // Missing last_wallpaper_hash must NOT count as up-to-date
        let partial_cache = AppStateCache {
            last_wallpaper_path: Some(wall.display().to_string()),
            last_wallpaper_hash: None,
            last_seed_color: Some("#112233".into()),
            last_is_dark: Some(true),
            last_config_fingerprint: None,
        };
        assert!(!partial_cache.is_up_to_date(&source, true));

        let fp1 = AppStateCache::compute_config_fingerprint(
            &["kitty".into()],
            Some(&rules_dir),
            Some(&tpl_dir),
        );
        let mut cache = AppStateCache::default();
        cache.update_source_with_fingerprint(&source, true, &color, Some(fp1.clone()));
        assert!(cache.is_up_to_date_with_config(&source, true, Some(&fp1)));

        // Adding a program invalidates fingerprint
        let fp2 = AppStateCache::compute_config_fingerprint(
            &["kitty".into(), "waybar".into()],
            Some(&rules_dir),
            Some(&tpl_dir),
        );
        assert_ne!(fp1, fp2);
        assert!(!cache.is_up_to_date_with_config(&source, true, Some(&fp2)));

        // Modifying a template file invalidates fingerprint
        fs::write(tpl_dir.join("kitty.tmpl"), "background #000000").unwrap();
        let fp3 = AppStateCache::compute_config_fingerprint(
            &["kitty".into()],
            Some(&rules_dir),
            Some(&tpl_dir),
        );
        assert_ne!(fp1, fp3);
        assert!(!cache.is_up_to_date_with_config(&source, true, Some(&fp3)));

        let _ = fs::remove_dir_all(&temp_dir);
    }
}
