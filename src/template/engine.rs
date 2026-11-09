//! MiniJinja template rendering environment and custom color filters.

use minijinja::{Environment, Value};
use serde::Serialize;
use crate::core::{GeneratedPalette, MaterialScheme, RgbColor, TerminalPalette};
use crate::error::{Result, TermonetError};

#[derive(Serialize)]
struct TemplateContext<'a> {
    seed_color: &'a RgbColor,
    light: &'a MaterialScheme,
    dark: &'a MaterialScheme,
    active: &'a MaterialScheme,
    theme: &'a MaterialScheme,
    terminal: &'a TerminalPalette,
    terminal_dark: &'a TerminalPalette,
    terminal_light: &'a TerminalPalette,
}

fn extract_stripped(val: &Value) -> String {
    if let Ok(strip_str) = val.get_attr("strip")
        && !strip_str.is_undefined()
    {
        strip_str.to_string()
    } else {
        val.to_string().trim_start_matches('#').to_string()
    }
}

fn strip_filter(val: &Value) -> std::result::Result<String, minijinja::Error> {
    Ok(extract_stripped(val))
}

/// Wrapper around `minijinja::Environment` configured with Termonet helpers.
pub struct TemplateEngine<'a> {
    env: Environment<'a>,
}

impl<'a> TemplateEngine<'a> {
    /// Initializes a new template engine with custom formatting filters.
    #[must_use]
    pub fn new() -> Self {
        let mut env = Environment::new();

        // Filter: hex (ensures `#rrggbb`)
        env.add_filter("hex", |val: &Value| -> std::result::Result<String, minijinja::Error> {
            if let Ok(hex_str) = val.get_attr("hex")
                && !hex_str.is_undefined()
            {
                return Ok(hex_str.to_string());
            }
            let s = val.to_string();
            if s.starts_with('#') {
                Ok(s)
            } else {
                Ok(format!("#{}", s))
            }
        });

        // Filter: strip and stripped (removes leading `#`)
        env.add_filter("strip", strip_filter);
        env.add_filter("stripped", strip_filter);

        // Filter: rgb (returns "r, g, b")
        env.add_filter("rgb", |val: &Value| -> std::result::Result<String, minijinja::Error> {
            if let Ok(rgb_str) = val.get_attr("rgb")
                && !rgb_str.is_undefined()
            {
                return Ok(rgb_str.to_string());
            }
            if let (Ok(r), Ok(g), Ok(b)) = (val.get_attr("r"), val.get_attr("b"), val.get_attr("b"))
                && !r.is_undefined() && !g.is_undefined() && !b.is_undefined()
            {
                return Ok(format!("{}, {}, {}", r, g, b));
            }
            Ok(val.to_string())
        });

        // Filter: hypr_rgb (returns "rgb(stripped)")
        env.add_filter("hypr_rgb", |val: &Value| -> std::result::Result<String, minijinja::Error> {
            let stripped = extract_stripped(val);
            Ok(format!("rgb({})", stripped))
        });

        // Filter: hypr_rgba (returns "rgba(stripped<alpha>)")
        env.add_filter(
            "hypr_rgba",
            |val: &Value, alpha: Option<&str>| -> std::result::Result<String, minijinja::Error> {
                let stripped = extract_stripped(val);
                let a = alpha.unwrap_or("ff");
                Ok(format!("rgba({}{})", stripped, a))
            },
        );

        Self { env }
    }

    /// Renders a template string in-memory using the provided palette and active theme mode.
    ///
    /// # Errors
    /// Returns `TermonetError::Template` if rendering or template parsing fails.
    pub fn render_string(
        &self,
        template_source: &str,
        palette: &GeneratedPalette,
        is_dark: bool,
    ) -> Result<String> {
        let tmpl = self
            .env
            .template_from_str(template_source)
            .map_err(|e| TermonetError::Template(e.to_string()))?;

        let active_scheme = palette.scheme_for(is_dark);
        let active_terminal = palette.terminal_for(is_dark);

        let ctx = Value::from(minijinja::value::Serde(TemplateContext {
            seed_color: &palette.seed_color,
            light: &palette.light,
            dark: &palette.dark,
            active: active_scheme,
            theme: active_scheme,
            terminal: active_terminal,
            terminal_dark: &palette.terminal,
            terminal_light: &palette.terminal_light,
        }));

        tmpl.render(ctx)
            .map_err(|e| TermonetError::Template(e.to_string()))
    }
}

impl<'a> Default for TemplateEngine<'a> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use material_colors::color::Rgb as MaterialRgb;

    #[test]
    fn test_template_rendering_direct_fields_and_filters() {
        let seed = MaterialRgb::new(0x21, 0x96, 0xf3);
        let palette = GeneratedPalette::from_seed_color(seed);
        let engine = TemplateEngine::new();

        let template = r#"
hex_field: {{ dark.primary.hex }}
strip_field: {{ dark.primary.strip }}
rgb_field: {{ dark.primary.rgb }}
hex_filter: {{ dark.primary | hex }}
strip_filter: {{ dark.primary | strip }}
hypr_rgb: {{ dark.primary | hypr_rgb }}
hypr_rgba: {{ dark.primary | hypr_rgba("cc") }}
terminal_bg: {{ terminal.background.hex }}
active_primary: {{ active.primary.hex }}
"#;

        let rendered_dark = engine.render_string(template, &palette, true).unwrap();
        assert!(rendered_dark.contains("hex_field: #"));
        assert!(rendered_dark.contains("strip_field: "));
        assert!(!rendered_dark.contains("strip_field: #"));
        assert!(rendered_dark.contains("hypr_rgb: rgb("));
        assert!(rendered_dark.contains("hypr_rgba: rgba("));
        assert!(rendered_dark.contains("cc)"));
        assert!(rendered_dark.contains("terminal_bg: #"));
        assert!(rendered_dark.contains(&format!("active_primary: {}", palette.dark.primary.to_hex())));

        let rendered_light = engine.render_string(template, &palette, false).unwrap();
        assert!(rendered_light.contains(&format!("active_primary: {}", palette.light.primary.to_hex())));
        assert!(rendered_light.contains(&format!("terminal_bg: {}", palette.terminal_light.background.to_hex())));
    }
}
