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
        let env = Environment::new();
        // Custom filters (strip hex, rgb convert, etc.) will be attached here
        Self { env }
    }

    /// Renders a template string in-memory using the provided palette.
    ///
    /// # Errors
    /// Returns `TermonetError::Template` if rendering or template parsing fails.
    pub fn render_string(&self, template_source: &str, palette: &GeneratedPalette) -> Result<String> {
        let tmpl = self.env.template_from_str(template_source)
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
