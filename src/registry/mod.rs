//! External application template registry management.

pub mod client;
pub mod types;

pub use client::{HttpTransport, MAX_MANIFEST_BYTES, MAX_ASSET_BYTES, RegistryTransport};
pub use types::{
    CatalogEntry, CatalogStatus, InstallPreview, InstalledEntry, InstalledManifest,
    RegistryManifest, RegistryProgram, RepoConfig, UpdateReport, is_version_compatible,
    sanitize_relative_repo_path, sha256_hex, validate_program_id,
};
