//! Registry data models, validation helpers, and version checks.

use crate::error::{Result, TermonetError};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

fn default_true() -> bool {
    true
}

fn default_schema_version() -> u32 {
    1
}

/// Repository source definition stored in `~/.config/termonet/repos/*.toml`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepoConfig {
    pub name: String,
    pub url: String,
    #[serde(default)]
    pub priority: i32,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

/// Remote `registry.toml` catalog manifest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegistryManifest {
    pub schema_version: u32,
    #[serde(default)]
    pub programs: Vec<RegistryProgram>,
}

/// Single application entry inside `registry.toml`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegistryProgram {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub category: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub description_ru: String,
    pub rule: String,
    pub template: String,
    pub rule_version: u32,
    pub template_version: u32,
    pub min_termonet_version: String,
    pub rule_sha256: String,
    pub template_sha256: String,
}

/// Local manifest tracking registry-installed programs (`~/.local/state/termonet/installed.toml`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct InstalledManifest {
    #[serde(default = "default_schema_version")]
    pub schema_version: u32,
    #[serde(default)]
    pub installed: Vec<InstalledEntry>,
}

/// Tracked installation record for a single program.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstalledEntry {
    pub id: String,
    pub repo: String,
    pub rule_file: String,
    pub template_file: String,
    pub rule_version: u32,
    pub template_version: u32,
    pub rule_sha256: String,
    pub template_sha256: String,
}

/// Installation and update status of a program in the unified catalog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CatalogStatus {
    Installed {
        repo: String,
        update_available: Option<(u32, u32)>,
        locally_modified: bool,
    },
    Available {
        repo: String,
        compatible: bool,
    },
    LocalCustom,
}

/// Unified view item merging installed, repository-available, and local custom rules.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatalogEntry {
    pub id: String,
    pub name: String,
    pub category: String,
    pub description: String,
    pub description_ru: String,
    pub status: CatalogStatus,
    pub output_path: Option<String>,
    pub target_config: Option<String>,
    pub reload_cmd: Option<String>,
    pub min_termonet_version: Option<String>,
}

/// Pre-flight install preview shown to the user before downloading/writing files.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstallPreview {
    pub id: String,
    pub repo: String,
    pub name: String,
    pub output_path: String,
    pub target_config: Option<String>,
    pub reload_cmd: Option<String>,
    pub has_local_conflict: bool,
    pub rule_version: u32,
    pub template_version: u32,
}

/// Summary returned by `update_installed()`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct UpdateReport {
    pub updated: Vec<String>,
    pub skipped_modified: Vec<String>,
    pub skipped_incompatible: Vec<String>,
    pub up_to_date: Vec<String>,
    pub errors: Vec<String>,
}

/// Validates that a program `id` matches `^[a-z0-9][a-z0-9_-]*$` without path separators.
pub fn validate_program_id(id: &str) -> Result<()> {
    if id.is_empty() || id.len() > 64 {
        return Err(TermonetError::Registry(format!(
            "Invalid program id '{id}': must be 1..=64 characters"
        )));
    }
    let mut chars = id.chars();
    let first_ok = chars
        .next()
        .is_some_and(|c| c.is_ascii_lowercase() || c.is_ascii_digit());
    let rest_ok =
        chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_');
    if !first_ok || !rest_ok {
        return Err(TermonetError::Registry(format!(
            "Invalid program id '{id}': must match ^[a-z0-9][a-z0-9_-]*$"
        )));
    }
    Ok(())
}

/// Validates a single local filename (no `/`, `\`, `.` or `..` segments).
pub fn sanitize_single_filename(name: &str) -> Result<&str> {
    if name.is_empty()
        || name == "."
        || name == ".."
        || name.contains('/')
        || name.contains('\\')
        || name.contains('\0')
    {
        return Err(TermonetError::Registry(format!(
            "Unsafe local filename '{name}'"
        )));
    }
    Ok(name)
}

/// Validates a relative path inside a remote repository (rejects `..`, `\`, leading `/`).
pub fn sanitize_relative_repo_path(rel: &str) -> Result<&str> {
    if rel.is_empty() || rel.starts_with('/') || rel.contains('\\') {
        return Err(TermonetError::Registry(format!(
            "Unsafe relative repository path '{rel}'"
        )));
    }
    for segment in rel.split('/') {
        if segment.is_empty() || segment == "." || segment == ".." {
            return Err(TermonetError::Registry(format!(
                "Unsafe relative repository path segment in '{rel}'"
            )));
        }
    }
    Ok(rel)
}

fn parse_semver(ver: &str) -> Result<(u64, u64, u64)> {
    let trimmed = ver.trim().strip_prefix('v').unwrap_or(ver.trim());
    let mut parts = trimmed.split('.');
    let major = parts
        .next()
        .and_then(|s| s.parse::<u64>().ok())
        .ok_or_else(|| TermonetError::Registry(format!("Invalid semver version '{ver}'")))?;
    let minor = parts
        .next()
        .and_then(|s| s.parse::<u64>().ok())
        .ok_or_else(|| TermonetError::Registry(format!("Invalid semver version '{ver}'")))?;
    let patch = parts
        .next()
        .and_then(|s| s.parse::<u64>().ok())
        .ok_or_else(|| TermonetError::Registry(format!("Invalid semver version '{ver}'")))?;
    if parts.next().is_some() {
        return Err(TermonetError::Registry(format!(
            "Invalid semver version '{ver}': expected major.minor.patch"
        )));
    }
    Ok((major, minor, patch))
}

/// Checks whether `current` semver (`major.minor.patch`) satisfies `>= min_required`.
pub fn is_version_compatible(min_required: &str, current: &str) -> Result<bool> {
    let req = parse_semver(min_required)?;
    let cur = parse_semver(current)?;
    Ok(cur >= req)
}

/// Computes lowercase hex SHA-256 digest of a byte slice.
#[must_use]
pub fn sha256_hex(bytes: &[u8]) -> String {
    use std::fmt::Write;
    let digest = Sha256::digest(bytes);
    let mut hex = String::with_capacity(64);
    for byte in digest {
        let _ = write!(hex, "{byte:02x}");
    }
    hex
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_program_id_accepts_valid_and_rejects_traversal() {
        assert!(validate_program_id("kitty").is_ok());
        assert!(validate_program_id("gtk-4").is_ok());
        assert!(validate_program_id("my_app2").is_ok());

        for bad in [
            "", "-bad", "_bad", "Kitty", "../kitty", "a/b", r"a\b", "a..b",
        ] {
            assert!(
                validate_program_id(bad).is_err(),
                "Expected invalid id '{bad}' to be rejected"
            );
        }
    }

    #[test]
    fn test_sanitize_relative_repo_path_rejects_parent_and_absolute() {
        assert_eq!(
            sanitize_relative_repo_path("rules/kitty.toml").unwrap(),
            "rules/kitty.toml"
        );
        for bad in [
            "",
            "/etc/passwd",
            "../secret",
            "rules/../kitty.toml",
            r"rules\kitty.toml",
        ] {
            assert!(
                sanitize_relative_repo_path(bad).is_err(),
                "Expected unsafe path '{bad}' to be rejected"
            );
        }
    }

    #[test]
    fn test_semver_compatibility_gating() {
        assert!(is_version_compatible("1.6.0", "1.6.0").unwrap());
        assert!(is_version_compatible("1.5.2", "1.6.0").unwrap());
        assert!(is_version_compatible("1.6.0", "2.0.0").unwrap());
        assert!(!is_version_compatible("1.7.0", "1.6.0").unwrap());
        assert!(!is_version_compatible("1.6.1", "1.6.0").unwrap());
        assert!(is_version_compatible("bad-ver", "1.6.0").is_err());
        assert!(is_version_compatible("1.6", "1.6.0").is_err());
    }

    #[test]
    fn test_sha256_hex_known_vector() {
        assert_eq!(
            sha256_hex(b"hello world"),
            "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
        );
    }

    #[test]
    fn test_stress_path_traversal_and_semver_boundaries() {
        let long_64 = "a".repeat(64);
        let long_65 = "a".repeat(65);
        assert!(validate_program_id(&long_64).is_ok());
        assert!(validate_program_id(&long_65).is_err());

        for bad_id in [
            "",
            " ",
            "../kitty",
            "..\\kitty",
            "a/b",
            "a\\b",
            "a..b",
            "kitty\0",
            "kitty\n",
            "-kitty",
            "_kitty",
            "Kitty",
            "китти",
            "kitty.toml",
        ] {
            assert!(
                validate_program_id(bad_id).is_err(),
                "ID '{bad_id:?}' must be rejected"
            );
        }

        for bad_rel in [
            "",
            "/",
            "/etc/passwd",
            "..",
            ".",
            "../rules/kitty.toml",
            "rules/./kitty.toml",
            "rules/../kitty.toml",
            "rules//kitty.toml",
            "rules/kitty.toml/",
            "rules\\kitty.toml",
            "rules/..\\kitty.toml",
        ] {
            assert!(
                sanitize_relative_repo_path(bad_rel).is_err(),
                "Path '{bad_rel:?}' must be rejected"
            );
        }

        assert!(is_version_compatible("0.0.0", "0.0.0").unwrap());
        assert!(is_version_compatible("1.6.0", "v1.6.0").unwrap());
        assert!(is_version_compatible("v1.6.0", "1.6.1").unwrap());
        assert!(!is_version_compatible("1.6.1", "1.6.0").unwrap());
        assert!(!is_version_compatible("2.0.0", "1.999.999").unwrap());
        assert!(
            is_version_compatible(&format!("{}.0.0", u64::MAX), &format!("{}.0.0", u64::MAX))
                .unwrap()
        );

        for bad_ver in [
            "",
            "1.6",
            "1.6.0.0",
            "1.6.0-alpha",
            "1.6.0+build",
            "-1.0.0",
            "1.a.0",
            "999999999999999999999999.0.0",
        ] {
            assert!(
                is_version_compatible(bad_ver, "1.6.0").is_err(),
                "Invalid semver '{bad_ver}' must error"
            );
        }
    }
}
