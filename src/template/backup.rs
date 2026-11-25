//! Safe backup management and atomic file writes.

use crate::error::{Result, TermonetError};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static TMP_FILE_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Resolves a destination path through symlinks so atomic renames update the target
/// without replacing the symlink itself. Fails cleanly on broken symlinks.
fn resolve_destination(dest: &Path) -> Result<PathBuf> {
    match fs::symlink_metadata(dest) {
        Ok(meta) if meta.file_type().is_symlink() => fs::canonicalize(dest).map_err(|e| {
            TermonetError::Io(std::io::Error::new(
                e.kind(),
                format!(
                    "Refusing to overwrite broken symlink {}: {}",
                    dest.display(),
                    e
                ),
            ))
        }),
        _ => Ok(dest.to_path_buf()),
    }
}

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
    let effective_dest = resolve_destination(dest)?;

    if effective_dest.is_dir() {
        return Err(TermonetError::Config(format!(
            "Cannot write file: destination {} is a directory",
            effective_dest.display()
        )));
    }

    // Skip rewrite if file already exists with identical content (avoids touching mtime and redundant disk I/O)
    if effective_dest.exists()
        && let Ok(existing) = fs::read_to_string(&effective_dest)
        && existing == content
    {
        return Ok(());
    }

    if let Some(parent) = effective_dest.parent() {
        fs::create_dir_all(parent)?;
    }

    if create_backup {
        create_safety_backup(dest)?;
    }

    let existing_perms = fs::metadata(&effective_dest).ok().map(|m| m.permissions());

    // Atomic write: write to a unique temporary file in the same directory and rename
    let file_name = effective_dest
        .file_name()
        .unwrap_or_default()
        .to_string_lossy();
    let seq = TMP_FILE_COUNTER.fetch_add(1, Ordering::Relaxed);
    let tmp_path =
        effective_dest.with_file_name(format!(".{}.tmp.{}.{}", file_name, std::process::id(), seq));
    fs::write(&tmp_path, content)?;
    if let Some(perms) = existing_perms {
        let _ = fs::set_permissions(&tmp_path, perms);
    }
    if let Err(e) = fs::rename(&tmp_path, &effective_dest) {
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

    #[test]
    fn test_safe_write_preserves_symlinks_and_permissions() {
        use std::os::unix::fs::{PermissionsExt, symlink};

        let temp_dir =
            std::env::temp_dir().join(format!("termonet_test_symlink_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        let real_target = temp_dir.join("dotfiles_kitty.conf");
        let link_path = temp_dir.join("kitty.conf");
        fs::write(&real_target, "old content").unwrap();
        fs::set_permissions(&real_target, fs::Permissions::from_mode(0o750)).unwrap();
        symlink(&real_target, &link_path).unwrap();

        safe_write(&link_path, "new content", false).unwrap();

        // Symlink must still be a symlink, and target must have updated content + preserved perms
        assert!(
            fs::symlink_metadata(&link_path)
                .unwrap()
                .file_type()
                .is_symlink()
        );
        assert_eq!(fs::read_to_string(&real_target).unwrap(), "new content");
        let mode = fs::metadata(&real_target).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o750);

        // Broken symlink must fail cleanly without destroying the symlink
        let broken_link = temp_dir.join("broken.conf");
        symlink(temp_dir.join("missing_target.conf"), &broken_link).unwrap();
        assert!(safe_write(&broken_link, "data", false).is_err());
        assert!(
            fs::symlink_metadata(&broken_link)
                .unwrap()
                .file_type()
                .is_symlink()
        );

        let _ = fs::remove_dir_all(&temp_dir);
    }
}
