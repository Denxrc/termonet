//! Internationalization and multi-language support.

pub mod locales;

pub use locales::{Translations, EN_TRANSLATIONS, RU_TRANSLATIONS};
use crate::config::Language;

/// Returns localized string resources for the selected language.
#[must_use]
pub fn get_translations(lang: Language) -> &'static Translations {
    match lang {
        Language::Ru => &RU_TRANSLATIONS,
        Language::En => &EN_TRANSLATIONS,
    }
}
