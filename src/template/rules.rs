//! Program rules definition, TOML deserialization, and safe configuration injection.

use std::fs;
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};

use crate::config::AppPaths;
use crate::core::GeneratedPalette;
use crate::error::{Result, TermonetError};
use crate::template::backup::{ensure_master_backup, safe_write};
use crate::template::embedded::{DEFAULT_RULES, DEFAULT_TEMPLATES};
use crate::template::engine::TemplateEngine;

/// Position where the import line should be injected into target config file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ImportPosition {
    #[default]
    Top,
    Bottom,
}

/// Comment syntax variant used for enclosing Termonet marker blocks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum CommentStyle {
    #[default]
    Hash,       // #
    BlockCss,   // /* ... */
    Slash,      // //
    DoubleDash, // -- (Lua, SQL, Haskell)
}

impl CommentStyle {
    /// Determines the appropriate comment style from a file path or extension.
    #[must_use]
    pub fn from_path<P: AsRef<Path>>(path: P) -> Self {
        let p = path.as_ref();
        match p.extension().and_then(|ext| ext.to_str()).unwrap_or("") {
            "css" | "rasi" => Self::BlockCss,
            "js" | "ts" | "jsonc" => Self::Slash,
            "lua" => Self::DoubleDash,
            _ => Self::Hash,
        }
    }

    /// Formats the opening block marker.
    #[must_use]
    pub fn begin_marker(self) -> &'static str {
        match self {
            Self::Hash => "# >>> termonet colors >>>",
            Self::BlockCss => "/* >>> termonet colors >>> */",
            Self::Slash => "// >>> termonet colors >>>",
            Self::DoubleDash => "-- >>> termonet colors >>>",
        }
    }

    /// Formats the closing block marker.
    #[must_use]
    pub fn end_marker(self) -> &'static str {
        match self {
            Self::Hash => "# <<< termonet colors <<<",
            Self::BlockCss => "/* <<< termonet colors <<< */",
            Self::Slash => "// <<< termonet colors <<<",
            Self::DoubleDash => "-- <<< termonet colors <<<",
        }
    }
}

fn default_rule_version() -> u32 {
    1
}

/// Rule definition for an individual target application.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProgramRule {
    /// Schema and content version for auto-migration and updates.
    #[serde(default = "default_rule_version")]
    pub version: u32,

    /// Identifier or display name of the program (e.g. "Kitty", "Waybar").
    pub name: String,

    /// Filename of the minijinja template (e.g. "kitty.tmpl").
    pub template: String,

    /// Destination path where the rendered color palette should be saved.
    pub output_path: PathBuf,

    /// Optional target main configuration file to inject `import_line` into.
    pub target_config: Option<PathBuf>,

    /// Optional import directive (e.g. "include colors-termonet.conf" or "@import 'colors.css';").
    pub import_line: Option<String>,

    /// Where to place `import_line` if not already present.
    #[serde(default)]
    pub import_position: ImportPosition,

    /// Shell command to reload the application without restarting it.
    pub reload_cmd: Option<String>,

    /// Explicit comment style override. If `None`, deduced from `target_config` extension.
    pub comment_style: Option<CommentStyle>,
}

impl ProgramRule {
    /// Parses a single rule from TOML string.
    pub fn from_toml(content: &str) -> Result<Self> {
        let rule: Self = toml::from_str(content)?;
        Ok(rule)
    }

    /// Resolves output path with user home tilde expansion.
    #[must_use]
    pub fn resolve_output_path(&self) -> PathBuf {
        AppPaths::expand_tilde(&self.output_path.to_string_lossy())
    }

    /// Resolves target configuration file path with user home tilde expansion.
    #[must_use]
    pub fn resolve_target_config(&self) -> Option<PathBuf> {
        self.target_config
            .as_ref()
            .map(|p| AppPaths::expand_tilde(&p.to_string_lossy()))
    }

    /// Returns the effective comment style for this rule.
    #[must_use]
    pub fn effective_comment_style(&self) -> CommentStyle {
        if let Some(style) = self.comment_style {
            return style;
        }
        if let Some(ref target) = self.target_config {
            return CommentStyle::from_path(target);
        }
        CommentStyle::Hash
    }

    /// Loads all available program rules: merges embedded defaults and user rules.
    /// If an existing user rule is an outdated default (lower version), the newer embedded default is preferred.
    pub fn load_all() -> Result<Vec<Self>> {
        let mut rules_map: std::collections::HashMap<String, Self> = std::collections::HashMap::new();

        // 1. Seed with embedded defaults
        for (_, content) in DEFAULT_RULES {
            if let Ok(rule) = Self::from_toml(content) {
                rules_map.insert(rule.name.to_lowercase(), rule);
            }
        }

        // 2. Scan user's rules directory (~/.config/termonet/rules/)
        if let Ok(rules_dir) = AppPaths::rules_dir()
            && rules_dir.is_dir()
            && let Ok(entries) = fs::read_dir(&rules_dir)
        {
            let mut paths: Vec<_> = entries
                .filter_map(std::result::Result::ok)
                .map(|e| e.path())
                .filter(|p| p.extension().is_some_and(|ext| ext == "toml"))
                .collect();
            paths.sort();

            for path in paths {
                if let Ok(content) = fs::read_to_string(&path)
                    && let Ok(user_rule) = Self::from_toml(&content)
                {
                    let key = user_rule.name.to_lowercase();
                    if let Some(embedded_rule) = rules_map.get(&key) {
                        // Only override if the user rule has equal or newer version
                        if user_rule.version >= embedded_rule.version {
                            rules_map.insert(key, user_rule);
                        }
                    } else {
                        // User's own custom rule
                        rules_map.insert(key, user_rule);
                    }
                }
            }
        }

        let mut rules: Vec<_> = rules_map.into_values().collect();
        rules.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(rules)
    }

    /// Retrieves the raw template content for this rule.
    /// Automatically detects if a template on disk is an outdated default and provides the modern version.
    pub fn load_template_content(&self) -> Result<String> {
        let embedded_content = DEFAULT_TEMPLATES
            .iter()
            .find(|(name, _)| *name == self.template)
            .map(|(_, content)| *content);

        if let Ok(templates_dir) = AppPaths::templates_dir() {
            let file_path = templates_dir.join(&self.template);
            if file_path.is_file()
                && let Ok(disk_content) = fs::read_to_string(&file_path)
            {
                if let Some(embedded) = embedded_content {
                    let disk_version = extract_template_version(&disk_content);
                    let embedded_version = extract_template_version(embedded);
                    if disk_version < embedded_version {
                        return Ok(embedded.to_string());
                    }
                }
                return Ok(disk_content);
            }
        }

        if let Some(content) = embedded_content {
            return Ok(content.to_string());
        }

        Err(TermonetError::Template(format!(
            "Template file not found: {}",
            self.template
        )))
    }

    /// Renders the template in memory, writes the output color palette,
    /// and safely injects the import directive into target configuration file.
    pub fn apply(&self, engine: &TemplateEngine, palette: &GeneratedPalette) -> Result<()> {
        let template_content = self.load_template_content()?;
        let rendered = engine.render_string(&template_content, palette)?;

        // 1. Write the rendered palette file
        let output_dest = self.resolve_output_path();
        safe_write(&output_dest, &rendered)?;

        // 2. Inject import line into target config if configured
        if let (Some(target_path), Some(import_line)) =
            (self.resolve_target_config(), &self.import_line)
        {
            let comment_style = self.effective_comment_style();

            let original_content = if target_path.exists() {
                ensure_master_backup(&target_path)?;
                fs::read_to_string(&target_path)?
            } else {
                String::new()
            };

            let updated_content = inject_import(
                &original_content,
                import_line,
                self.import_position,
                comment_style,
            );

            safe_write(&target_path, &updated_content)?;
        }

        Ok(())
    }
}

/// Extracts template version integer from comment headers (e.g. `termonet-version: 2`).
/// Defaults to 1 if not found.
#[must_use]
pub fn extract_template_version(content: &str) -> u32 {
    for line in content.lines().take(10) {
        if let Some(idx) = line.find("termonet-version:") {
            let rest = line[idx + "termonet-version:".len()..].trim();
            if let Ok(ver) = rest.split_whitespace().next().unwrap_or("").parse::<u32>() {
                return ver;
            }
        }
    }
    1
}

/// Injects or updates an import line within the content string, enclosed in Termonet marker comments.
///
/// If a Termonet block already exists, it is updated in-place.
/// If an existing legacy/pywal color import line exists, it is replaced with the Termonet block.
/// Otherwise, the block is prepended or appended according to `position`.
#[must_use]
pub fn inject_import(
    content: &str,
    import_line: &str,
    position: ImportPosition,
    style: CommentStyle,
) -> String {
    let begin_marker = style.begin_marker();
    let end_marker = style.end_marker();
    let block = format!("{}\n{}\n{}", begin_marker, import_line, end_marker);

    // Case 1: Termonet block already exists -> replace in-place
    if let Some(begin_idx) = content.find(begin_marker)
        && let Some(end_rel_idx) = content[begin_idx..].find(end_marker)
    {
        let end_idx = begin_idx + end_rel_idx + end_marker.len();
        let mut result = String::with_capacity(content.len() + 32);
        result.push_str(&content[..begin_idx]);
        result.push_str(&block);
        result.push_str(&content[end_idx..]);
        return result;
    }

    // Case 2: Check for foreign / legacy color imports (e.g. pywal, old color includes)
    let lines: Vec<&str> = content.lines().collect();
    let mut replaced = false;
    let mut new_lines: Vec<String> = Vec::with_capacity(lines.len() + 4);

    for line in &lines {
        let trimmed = line.trim();
        let is_legacy_color_import = (trimmed.contains("colors")
            || trimmed.contains("wal")
            || trimmed.contains("termonet"))
            && (trimmed.starts_with("include")
                || trimmed.starts_with("@import")
                || trimmed.starts_with("@theme")
                || trimmed.starts_with("source")
                || trimmed.starts_with("local")
                || trimmed.contains("require"));

        if !replaced && is_legacy_color_import {
            new_lines.push(block.clone());
            replaced = true;
        } else {
            new_lines.push((*line).to_string());
        }
    }

    if replaced {
        let mut res = new_lines.join("\n");
        if content.ends_with('\n') {
            res.push('\n');
        }
        return res;
    }

    // Case 3: Fresh insertion at Top or Bottom
    match position {
        ImportPosition::Top => {
            if content.starts_with("#!") {
                // Keep shebang at line 0 if present
                if let Some(first_nl) = content.find('\n') {
                    let shebang = &content[..=first_nl];
                    let rest = &content[first_nl + 1..];
                    return format!("{}{}\n{}", shebang, block, rest);
                }
            }
            if content.is_empty() {
                format!("{}\n", block)
            } else {
                format!("{}\n{}", block, content)
            }
        }
        ImportPosition::Bottom => {
            if content.is_empty() {
                format!("{}\n", block)
            } else if content.ends_with('\n') {
                format!("{}{}\n", content, block)
            } else {
                format!("{}\n{}\n", content, block)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inject_fresh_bottom() {
        let original = "font_size 12.0\nenable_audio_bell no\n";
        let injected = inject_import(
            original,
            "include colors-termonet.conf",
            ImportPosition::Bottom,
            CommentStyle::Hash,
        );

        assert!(injected.starts_with("font_size 12.0\n"));
        assert!(injected.contains("# >>> termonet colors >>>"));
        assert!(injected.contains("include colors-termonet.conf"));
        assert!(injected.contains("# <<< termonet colors <<<"));
    }

    #[test]
    fn test_inject_fresh_top() {
        let original = "window {\n    border: 2px;\n}\n";
        let injected = inject_import(
            original,
            "@import 'colors.css';",
            ImportPosition::Top,
            CommentStyle::BlockCss,
        );

        assert!(injected.starts_with("/* >>> termonet colors >>> */\n@import 'colors.css';\n/* <<< termonet colors <<< */"));
        assert!(injected.contains("window {"));
    }

    #[test]
    fn test_update_existing_termonet_block() {
        let original = r#"# Main config
# >>> termonet colors >>>
include old-colors.conf
# <<< termonet colors <<<
other_setting yes
"#;
        let updated = inject_import(
            original,
            "include colors-termonet.conf",
            ImportPosition::Bottom,
            CommentStyle::Hash,
        );

        assert!(!updated.contains("include old-colors.conf"));
        assert!(updated.contains("include colors-termonet.conf"));
        // Ensure no duplicate markers were added
        assert_eq!(updated.matches("# >>> termonet colors >>>").count(), 1);
        assert_eq!(updated.matches("# <<< termonet colors <<<").count(), 1);
    }

    #[test]
    fn test_replace_legacy_pywal_import() {
        let original = r#"# Kitty configuration
font_family monospace
include ~/.cache/wal/colors-kitty.conf
cursor_shape beam
"#;
        let replaced = inject_import(
            original,
            "include colors-termonet.conf",
            ImportPosition::Bottom,
            CommentStyle::Hash,
        );

        assert!(!replaced.contains("~/.cache/wal/colors-kitty.conf"));
        assert!(replaced.contains("# >>> termonet colors >>>"));
        assert!(replaced.contains("include colors-termonet.conf"));
        assert!(replaced.contains("cursor_shape beam"));
    }

    #[test]
    fn test_load_embedded_default_rules() {
        let rules = ProgramRule::load_all().unwrap();
        assert!(rules.len() >= 4);

        let names: Vec<_> = rules.iter().map(|r| r.name.as_str()).collect();
        assert!(names.contains(&"Kitty"));
        assert!(names.contains(&"Waybar"));
        assert!(names.contains(&"Rofi"));
        assert!(names.contains(&"Hyprland"));
    }

    #[test]
    fn test_render_all_default_templates() {
        use material_colors::color::Rgb as MaterialRgb;

        let seed = MaterialRgb::new(0x67, 0x50, 0xa4);
        let palette = GeneratedPalette::from_seed_color(seed);
        let engine = TemplateEngine::new();

        for &(name, content) in DEFAULT_TEMPLATES {
            let rendered = engine.render_string(content, &palette).unwrap();

            assert!(!rendered.is_empty(), "Rendered template for {} was empty", name);
            match name {
                "kitty.tmpl" => {
                    assert!(rendered.contains("background #"));
                    assert!(rendered.contains("color0 #"));
                    assert!(rendered.contains("color15 #"));
                }
                "waybar.tmpl" => {
                    assert!(rendered.contains("@define-color primary #"));
                    assert!(rendered.contains("@define-color background #"));
                }
                "rofi.tmpl" => {
                    assert!(rendered.contains("primary: #"));
                    assert!(rendered.contains("background: #"));
                }
                "hyprland.tmpl" => {
                    assert!(rendered.contains("return {"));
                    assert!(rendered.contains("primary = \"#"));
                    assert!(rendered.contains("active_border_1 = \"rgba("));
                }
                _ => {}
            }
        }
    }

    #[test]
    fn test_inject_hyprland_lua() {
        let original = r#"-- Hyprland Lua main configuration
hl.config({
    general = {
        border_size = 2,
    }
})
"#;
        let injected = inject_import(
            original,
            "local colors = require(\"colors\")",
            ImportPosition::Top,
            CommentStyle::DoubleDash,
        );

        assert!(injected.starts_with("-- >>> termonet colors >>>\nlocal colors = require(\"colors\")\n-- <<< termonet colors <<<"));
        assert!(injected.contains("hl.config({"));
    }
}
