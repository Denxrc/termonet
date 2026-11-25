//! Embedded assets extraction, versioning, and distribution.

use crate::error::Result;
use crate::template::rules::{ProgramRule, extract_template_version};
use std::fs;
use std::path::Path;

/// Default rule assets embedded directly into the binary.
pub const DEFAULT_RULES: &[(&str, &str)] = &[
    ("kitty.toml", include_str!("../../assets/rules/kitty.toml")),
    (
        "waybar.toml",
        include_str!("../../assets/rules/waybar.toml"),
    ),
    ("rofi.toml", include_str!("../../assets/rules/rofi.toml")),
    (
        "hyprland.toml",
        include_str!("../../assets/rules/hyprland.toml"),
    ),
];

/// Default template assets embedded directly into the binary.
pub const DEFAULT_TEMPLATES: &[(&str, &str)] = &[
    (
        "kitty.tmpl",
        include_str!("../../assets/templates/kitty.tmpl"),
    ),
    (
        "waybar.tmpl",
        include_str!("../../assets/templates/waybar.tmpl"),
    ),
    (
        "rofi.tmpl",
        include_str!("../../assets/templates/rofi.tmpl"),
    ),
    (
        "hyprland.tmpl",
        include_str!("../../assets/templates/hyprland.tmpl"),
    ),
];

/// Synchronizes embedded rules and templates with the user's config directory.
///
/// - If a file does not exist, it is written directly.
/// - If a file exists, its version is inspected:
///   - If outdated (lower version than embedded default), a `<file>.v{old}.bak` backup is made
///     and the file is safely upgraded to the modern default!
///   - If current or newer, user's customizations are preserved untouched.
///
/// Returns a list of filenames that were newly installed or upgraded.
pub fn sync_default_assets<P: AsRef<Path>>(
    config_dir: P,
    force: bool,
    create_backups: bool,
) -> Result<Vec<String>> {
    let rules_dir = config_dir.as_ref().join("rules");
    let templates_dir = config_dir.as_ref().join("templates");

    // Gracefully handle read-only environments
    let ensure_dir = |dir: &Path| -> Result<bool> {
        if let Err(e) = fs::create_dir_all(dir) {
            if e.kind() == std::io::ErrorKind::PermissionDenied || e.raw_os_error() == Some(30) {
                return Ok(false);
            }
            return Err(e.into());
        }
        Ok(true)
    };

    if !ensure_dir(&rules_dir)? || !ensure_dir(&templates_dir)? {
        return Ok(Vec::new());
    }

    let is_readonly_err = |e: &std::io::Error| {
        e.kind() == std::io::ErrorKind::PermissionDenied || e.raw_os_error() == Some(30)
    };

    let write_or_ro = |dest: &Path, content: &str| -> Result<bool> {
        match fs::write(dest, content) {
            Ok(()) => Ok(true),
            Err(e) if is_readonly_err(&e) => Ok(false),
            Err(e) => Err(e.into()),
        }
    };

    let mut updated = Vec::new();

    // 1. Sync Rules
    for &(name, embedded_content) in DEFAULT_RULES {
        let dest = rules_dir.join(name);
        let embedded_rule = match ProgramRule::from_toml(embedded_content) {
            Ok(r) => r,
            Err(_) => continue,
        };

        if !dest.exists() {
            if !write_or_ro(&dest, embedded_content)? {
                return Ok(updated);
            }
            updated.push(name.to_string());
        } else {
            let disk_content = fs::read_to_string(&dest)?;
            let disk_rule = ProgramRule::from_toml(&disk_content);
            let is_corrupted = disk_rule.is_err();
            let disk_ver = disk_rule.map(|r| r.version).unwrap_or(0);

            if force || is_corrupted || disk_ver < embedded_rule.version {
                if create_backups {
                    let backup_path = rules_dir.join(format!("{}.v{}.bak", name, disk_ver));
                    if !write_or_ro(&backup_path, &disk_content)? {
                        return Ok(updated);
                    }
                }
                if !write_or_ro(&dest, embedded_content)? {
                    return Ok(updated);
                }
                updated.push(format!(
                    "{} (v{} -> v{})",
                    name, disk_ver, embedded_rule.version
                ));
            }
        }
    }

    // 2. Sync Templates
    for &(name, embedded_content) in DEFAULT_TEMPLATES {
        let dest = templates_dir.join(name);
        let embedded_ver = extract_template_version(embedded_content);

        if !dest.exists() {
            if !write_or_ro(&dest, embedded_content)? {
                return Ok(updated);
            }
            updated.push(name.to_string());
        } else {
            let disk_content = fs::read_to_string(&dest)?;
            let disk_ver = extract_template_version(&disk_content);

            if force || disk_ver < embedded_ver {
                if create_backups {
                    let backup_path = templates_dir.join(format!("{}.v{}.bak", name, disk_ver));
                    if !write_or_ro(&backup_path, &disk_content)? {
                        return Ok(updated);
                    }
                }
                if !write_or_ro(&dest, embedded_content)? {
                    return Ok(updated);
                }
                updated.push(format!("{} (v{} -> v{})", name, disk_ver, embedded_ver));
            }
        }
    }

    Ok(updated)
}

/// Unpacks default embedded rules and templates to user's config directory if not already present.
pub fn unpack_default_assets<P: AsRef<Path>>(config_dir: P) -> Result<()> {
    sync_default_assets(config_dir, false, false)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_embedded_assets_present() {
        assert_eq!(DEFAULT_RULES.len(), 4);
        assert_eq!(DEFAULT_TEMPLATES.len(), 4);

        for (name, content) in DEFAULT_RULES {
            assert!(!name.is_empty());
            assert!(!content.is_empty());
            assert!(content.contains("name ="));
            assert!(content.contains("version = 2"));
        }

        for (name, content) in DEFAULT_TEMPLATES {
            assert!(!name.is_empty());
            assert!(!content.is_empty());
            assert!(content.contains("termonet-version: 4"));
        }
    }

    #[test]
    fn test_sync_outdated_assets_migration() {
        let temp_dir =
            std::env::temp_dir().join(format!("termonet_test_sync_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        let rules_dir = temp_dir.join("rules");
        let templates_dir = temp_dir.join("templates");
        fs::create_dir_all(&rules_dir).unwrap();
        fs::create_dir_all(&templates_dir).unwrap();

        // Write old version 1 rule and template (representing old hyprland.conf)
        let old_rule = r#"
name = "Hyprland"
template = "hyprland.tmpl"
output_path = "~/.config/hypr/colors.conf"
target_config = "~/.config/hypr/hyprland.conf"
import_line = "source = ~/.config/hypr/colors.conf"
"#;
        let old_template = "$primary = rgb(123456)\n";

        fs::write(rules_dir.join("hyprland.toml"), old_rule).unwrap();
        fs::write(templates_dir.join("hyprland.tmpl"), old_template).unwrap();

        // Run sync with create_backups = true
        let updated = sync_default_assets(&temp_dir, false, true).unwrap();
        assert!(
            updated
                .iter()
                .any(|s| s.contains("hyprland.toml (v1 -> v2)"))
        );
        assert!(
            updated
                .iter()
                .any(|s| s.contains("hyprland.tmpl (v1 -> v4)"))
        );

        // Check that backup was created
        assert!(rules_dir.join("hyprland.toml.v1.bak").exists());
        assert!(templates_dir.join("hyprland.tmpl.v1.bak").exists());

        // Check that content was upgraded to Lua
        let upgraded_rule = fs::read_to_string(rules_dir.join("hyprland.toml")).unwrap();
        assert!(upgraded_rule.contains("hyprland.lua"));
        assert!(upgraded_rule.contains("version = 2"));

        let upgraded_template = fs::read_to_string(templates_dir.join("hyprland.tmpl")).unwrap();
        assert!(upgraded_template.contains("return {"));
        assert!(upgraded_template.contains("termonet-version: 4"));

        // Second sync: should NOT update again since version is already current
        let updated_second = sync_default_assets(&temp_dir, false, true).unwrap();
        assert!(updated_second.is_empty());

        // Test sync with create_backups = false on a new outdated file
        fs::write(rules_dir.join("hyprland.toml"), old_rule).unwrap();
        let _ = fs::remove_file(rules_dir.join("hyprland.toml.v1.bak"));
        let updated_clean = sync_default_assets(&temp_dir, false, false).unwrap();
        assert!(
            updated_clean
                .iter()
                .any(|s| s.contains("hyprland.toml (v1 -> v2)"))
        );
        assert!(
            !rules_dir.join("hyprland.toml.v1.bak").exists(),
            "No .bak file when create_backups is false"
        );

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_sync_repairs_corrupted_rule_and_propagates_non_ro_errors() {
        let temp_dir =
            std::env::temp_dir().join(format!("termonet_test_sync_corrupt_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        let rules_dir = temp_dir.join("rules");
        let templates_dir = temp_dir.join("templates");
        fs::create_dir_all(&rules_dir).unwrap();
        fs::create_dir_all(&templates_dir).unwrap();

        // 1. Write invalid TOML for kitty.toml -> must be repaired with v0 -> v2 backup
        fs::write(rules_dir.join("kitty.toml"), "invalid toml [[[").unwrap();
        let updated = sync_default_assets(&temp_dir, false, true).unwrap();
        assert!(updated.iter().any(|s| s.contains("kitty.toml (v0 -> v2)")));
        assert!(rules_dir.join("kitty.toml.v0.bak").exists());
        let repaired = fs::read_to_string(rules_dir.join("kitty.toml")).unwrap();
        assert!(ProgramRule::from_toml(&repaired).is_ok());

        // 2. Non-RO error: replace rofi.toml with a directory -> must return Err
        let _ = fs::remove_file(rules_dir.join("rofi.toml"));
        fs::create_dir_all(rules_dir.join("rofi.toml")).unwrap();
        assert!(sync_default_assets(&temp_dir, false, false).is_err());

        let _ = fs::remove_dir_all(&temp_dir);
    }
}
