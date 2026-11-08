//! MiniJinja template rendering environment and custom color filters.

use minijinja::{Environment, Value};
use crate::core::GeneratedPalette;
use crate::error::{Result, TermonetError};

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

        // Filter: strip (removes leading `#`)
        env.add_filter("strip", |val: &Value| -> std::result::Result<String, minijinja::Error> {
            if let Ok(strip_str) = val.get_attr("strip")
                && !strip_str.is_undefined()
            {
                return Ok(strip_str.to_string());
            }
            let s = val.to_string();
            Ok(s.trim_start_matches('#').to_string())
        });

        // Alias for strip
        env.add_filter("stripped", |val: &Value| -> std::result::Result<String, minijinja::Error> {
            if let Ok(strip_str) = val.get_attr("strip")
                && !strip_str.is_undefined()
            {
                return Ok(strip_str.to_string());
            }
            let s = val.to_string();
            Ok(s.trim_start_matches('#').to_string())
        });

        // Filter: rgb (returns "r, g, b")
        env.add_filter("rgb", |val: &Value| -> std::result::Result<String, minijinja::Error> {
            if let Ok(rgb_str) = val.get_attr("rgb")
                && !rgb_str.is_undefined()
            {
                return Ok(rgb_str.to_string());
            }
            if let (Ok(r), Ok(g), Ok(b)) = (val.get_attr("r"), val.get_attr("g"), val.get_attr("b"))
                && !r.is_undefined() && !g.is_undefined() && !b.is_undefined()
            {
                return Ok(format!("{}, {}, {}", r, g, b));
            }
            Ok(val.to_string())
        });

        // Filter: hypr_rgb (returns "rgb(stripped)")
        env.add_filter("hypr_rgb", |val: &Value| -> std::result::Result<String, minijinja::Error> {
            let stripped = if let Ok(strip_str) = val.get_attr("strip") {
                if !strip_str.is_undefined() {
                    strip_str.to_string()
                } else {
                    val.to_string().trim_start_matches('#').to_string()
                }
            } else {
                val.to_string().trim_start_matches('#').to_string()
            };
            Ok(format!("rgb({})", stripped))
        });

        // Filter: hypr_rgba (returns "rgba(stripped<alpha>)")
        env.add_filter(
            "hypr_rgba",
            |val: &Value, alpha: Option<&str>| -> std::result::Result<String, minijinja::Error> {
                let stripped = if let Ok(strip_str) = val.get_attr("strip") {
                    if !strip_str.is_undefined() {
                        strip_str.to_string()
                    } else {
                        val.to_string().trim_start_matches('#').to_string()
                    }
                } else {
                    val.to_string().trim_start_matches('#').to_string()
                };
                let a = alpha.unwrap_or("ff");
                Ok(format!("rgba({}{})", stripped, a))
            },
        );

        Self { env }
    }

    /// Renders a template string in-memory using the provided palette.
    ///
    /// # Errors
    /// Returns `TermonetError::Template` if rendering or template parsing fails.
    pub fn render_string(&self, template_source: &str, palette: &GeneratedPalette) -> Result<String> {
        let tmpl = self
            .env
            .template_from_str(template_source)
            .map_err(|e| TermonetError::Template(e.to_string()))?;

        let ctx = Value::from(minijinja::value::Serde(palette));
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
"#;

        let rendered = engine.render_string(template, &palette).unwrap();
        assert!(rendered.contains("hex_field: #"));
        assert!(rendered.contains("strip_field: "));
        assert!(!rendered.contains("strip_field: #"));
        assert!(rendered.contains("hypr_rgb: rgb("));
        assert!(rendered.contains("hypr_rgba: rgba("));
        assert!(rendered.contains("cc)"));
        assert!(rendered.contains("terminal_bg: #"));
    }
}
