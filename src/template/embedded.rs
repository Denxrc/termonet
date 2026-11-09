//! Embedded assets extraction, versioning, and distribution.

use std::fs;
use std::path::Path;
use crate::error::Result;
use crate::template::rules::{extract_template_version, ProgramRule};

/// Default rule assets embedded directly into the binary.
pub const DEFAULT_RULES: &[(&str, &str)] = &[
    ("kitty.toml", include_str!("../../assets/rules/kitty.toml")),
    ("waybar.toml", include_str!("../../assets/rules/waybar.toml")),
    ("rofi.toml", include_str!("../../assets/rules/rofi.toml")),
    ("hyprland.toml", include_str!("../../assets/rules/hyprland.toml")),
];

/// Default template assets embedded directly into the binary.
pub const DEFAULT_TEMPLATES: &[(&str, &str)] = &[
    ("kitty.tmpl", include_str!("../../assets/templates/kitty.tmpl")),
    ("waybar.tmpl", include_str!("../../assets/templates/waybar.tmpl")),
    ("rofi.tmpl", include_str!("../../assets/templates/rofi.tmpl")),
    ("hyprland.tmpl", include_str!("../../assets/templates/hyprland.tmpl")),
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
pub fn sync_default_assets<P: AsRef<Path>>(config_dir: P, force: bool, create_backups: bool) -> Result<Vec<String>> {
    let rules_dir = config_dir.as_ref().join("rules");
    let templates_dir = config_dir.as_ref().join("templates");

    // Gracefully handle read-only environments
    if let Err(e) = fs::create_dir_all(&rules_dir) {
        if e.kind() == std::io::ErrorKind::PermissionDenied || e.raw_os_error() == Some(30) {
            return Ok(Vec::new());
        }
        return Err(e.into());
    }
    let _ = fs::create_dir_all(&templates_dir);

    let mut updated = Vec::new();

    // 1. Sync Rules
    for &(name, embedded_content) in DEFAULT_RULES {
        let dest = rules_dir.join(name);
        let embedded_rule = match ProgramRule::from_toml(embedded_content) {
            Ok(r) => r,
            Err(_) => continue,
        };

        if !dest.exists() {
            let _ = fs::write(&dest, embedded_content);
            updated.push(name.to_string());
        } else {
            let disk_content = fs::read_to_string(&dest).unwrap_or_default();
            let disk_rule = ProgramRule::from_toml(&disk_content);
            let disk_ver = disk_rule.map(|r| r.version).unwrap_or(1);

            if force || disk_ver < embedded_rule.version {
                if create_backups {
                    let backup_path = rules_dir.join(format!("{}.v{}.bak", name, disk_ver));
                    let _ = fs::write(&backup_path, &disk_content);
                }
                let _ = fs::write(&dest, embedded_content);
                updated.push(format!("{} (v{} -> v{})", name, disk_ver, embedded_rule.version));
            }
        }
    }

    // 2. Sync Templates
    for &(name, embedded_content) in DEFAULT_TEMPLATES {
        let dest = templates_dir.join(name);
        let embedded_ver = extract_template_version(embedded_content);

        if !dest.exists() {
            let _ = fs::write(&dest, embedded_content);
            updated.push(name.to_string());
        } else {
            let disk_content = fs::read_to_string(&dest).unwrap_or_default();
            let disk_ver = extract_template_version(&disk_content);

            if force || disk_ver < embedded_ver {
                if create_backups {
                    let backup_path = templates_dir.join(format!("{}.v{}.bak", name, disk_ver));
                    let _ = fs::write(&backup_path, &disk_content);
                }
                let _ = fs::write(&dest, embedded_content);
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
            assert!(content.contains("termonet-version: 3"));
        }
    }

    #[test]
    fn test_sync_outdated_assets_migration() {
        let temp_dir = std::env::temp_dir().join(format!("termonet_test_sync_{}", std::process::id()));
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
        assert!(updated.iter().any(|s| s.contains("hyprland.toml (v1 -> v2)")));
        assert!(updated.iter().any(|s| s.contains("hyprland.tmpl (v1 -> v3)")));

        // Check that backup was created
        assert!(rules_dir.join("hyprland.toml.v1.bak").exists());
        assert!(templates_dir.join("hyprland.tmpl.v1.bak").exists());

        // Check that content was upgraded to Lua
        let upgraded_rule = fs::read_to_string(rules_dir.join("hyprland.toml")).unwrap();
        assert!(upgraded_rule.contains("hyprland.lua"));
        assert!(upgraded_rule.contains("version = 2"));

        let upgraded_template = fs::read_to_string(templates_dir.join("hyprland.tmpl")).unwrap();
        assert!(upgraded_template.contains("return {"));
        assert!(upgraded_template.contains("termonet-version: 3"));

        // Second sync: should NOT update again since version is already current
        let updated_second = sync_default_assets(&temp_dir, false, true).unwrap();
        assert!(updated_second.is_empty());

        // Test sync with create_backups = false on a new outdated file
        fs::write(rules_dir.join("hyprland.toml"), old_rule).unwrap();
        let _ = fs::remove_file(rules_dir.join("hyprland.toml.v1.bak"));
        let updated_clean = sync_default_assets(&temp_dir, false, false).unwrap();
        assert!(updated_clean.iter().any(|s| s.contains("hyprland.toml (v1 -> v2)")));
        assert!(!rules_dir.join("hyprland.toml.v1.bak").exists(), "No .bak file when create_backups is false");

        let _ = fs::remove_dir_all(&temp_dir);
    }
}
