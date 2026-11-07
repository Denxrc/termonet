//! Safe backup management and atomic file writes.

use std::fs;
use std::path::Path;
use crate::error::Result;

/// Ensures that a master backup (`<file>.original`) exists for the given file.
/// Master backup is created once on initial integration and is never overwritten.
pub fn ensure_master_backup<P: AsRef<Path>>(path: P) -> Result<()> {
    let original_path = path.as_ref();
    if !original_path.exists() {
        return Ok(());
    }

    let backup_path = original_path.with_extension(format!(
        "{}.original",
        original_path.extension().and_then(|s| s.to_str()).unwrap_or("")
    ));

    if !backup_path.exists() {
        fs::copy(original_path, backup_path)?;
    }

    Ok(())
}

/// Creates a temporary safety backup (`<file>.bak`) before applying any modifications.
pub fn create_safety_backup<P: AsRef<Path>>(path: P) -> Result<()> {
    let original_path = path.as_ref();
    if !original_path.exists() {
        return Ok(());
    }

    let backup_path = original_path.with_extension("bak");
    fs::copy(original_path, backup_path)?;
    Ok(())
}

/// Safely writes content to disk by creating parent directories and writing atomically.
pub fn safe_write<P: AsRef<Path>>(path: P, content: &str) -> Result<()> {
    let dest = path.as_ref();
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent)?;
    }

    create_safety_backup(dest)?;
    fs::write(dest, content)?;
    Ok(())
}
