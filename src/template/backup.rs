//! Safe backup management and atomic file writes.

use crate::error::Result;
use std::fs;
use std::path::Path;

/// Ensures that a master backup (`<file>.original`) exists for the given file if `create_backup` is true.
/// Master backup is created once on initial integration and is never overwritten.
pub fn ensure_master_backup<P: AsRef<Path>>(path: P, create_backup: bool) -> Result<()> {
    if !create_backup {
        return Ok(());
    }
    let original_path = path.as_ref();
    if !original_path.exists() {
        return Ok(());
    }

    let file_name = original_path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy();
    let backup_path = original_path.with_file_name(format!("{}.original", file_name));

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

    let file_name = original_path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy();
    let backup_path = original_path.with_file_name(format!("{}.bak", file_name));
    fs::copy(original_path, backup_path)?;
    Ok(())
}

/// Safely writes content to disk by creating parent directories and writing atomically.
/// If `create_backup` is true, creates a temporary safety backup (`<file>.bak`) before overwriting an existing file.
/// If `create_backup` is false, updates the file directly without cluttering directories with backup copies.
pub fn safe_write<P: AsRef<Path>>(path: P, content: &str, create_backup: bool) -> Result<()> {
    let dest = path.as_ref();

    // Skip rewrite if file already exists with identical content (avoids touching mtime and redundant disk I/O)
    if dest.exists()
        && let Ok(existing) = fs::read_to_string(dest)
        && existing == content
    {
        return Ok(());
    }

    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent)?;
    }

    if create_backup {
        create_safety_backup(dest)?;
    }

    // Atomic write: write to a temporary file in the same directory and rename
    let file_name = dest.file_name().unwrap_or_default().to_string_lossy();
    let tmp_path = dest.with_file_name(format!(".{}.tmp.{}", file_name, std::process::id()));
    fs::write(&tmp_path, content)?;
    if let Err(e) = fs::rename(&tmp_path, dest) {
        let _ = fs::remove_file(&tmp_path);
        return Err(e.into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_safe_write_respects_backup_flag() {
        let temp_dir =
            std::env::temp_dir().join(format!("termonet_test_backup_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        let test_file = temp_dir.join("sub").join("test_write.txt");

        // 1. Initial write
        assert!(safe_write(&test_file, "initial content", false).is_ok());
        assert!(test_file.exists());
        assert_eq!(fs::read_to_string(&test_file).unwrap(), "initial content");

        // 2. Second write with identical content should succeed without error
        assert!(safe_write(&test_file, "initial content", false).is_ok());
        assert_eq!(fs::read_to_string(&test_file).unwrap(), "initial content");

        // 3. Write with modified content and create_backup = false -> NO .bak created!
        assert!(safe_write(&test_file, "clean updated content", false).is_ok());
        assert_eq!(
            fs::read_to_string(&test_file).unwrap(),
            "clean updated content"
        );
        let bak_file = test_file.with_file_name(format!(
            "{}.bak",
            test_file.file_name().unwrap().to_string_lossy()
        ));
        assert!(
            !bak_file.exists(),
            ".bak should not exist when create_backup is false"
        );

        // 4. Write with modified content and create_backup = true -> creates .bak
        assert!(safe_write(&test_file, "backed-up content", true).is_ok());
        assert_eq!(fs::read_to_string(&test_file).unwrap(), "backed-up content");
        assert!(
            bak_file.exists(),
            ".bak must exist when create_backup is true"
        );
        assert_eq!(
            fs::read_to_string(&bak_file).unwrap(),
            "clean updated content"
        );

        // 5. Test master backup
        assert!(ensure_master_backup(&test_file, false).is_ok());
        let master_bak = test_file.with_extension("txt.original");
        assert!(
            !master_bak.exists(),
            ".original should not exist when create_backup is false"
        );

        assert!(ensure_master_backup(&test_file, true).is_ok());
        assert!(
            master_bak.exists(),
            ".original must exist when create_backup is true"
        );

        let _ = fs::remove_dir_all(&temp_dir);
    }
}
