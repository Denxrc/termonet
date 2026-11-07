//! Embedded assets extraction and distribution.

use std::path::Path;
use crate::error::Result;

/// Unpacks default embedded rules and templates to user's config directory.
pub fn unpack_default_assets<P: AsRef<Path>>(_target_dir: P) -> Result<()> {
    // In v1.x: extracts embedded templates to ~/.config/termonet/
    Ok(())
}
