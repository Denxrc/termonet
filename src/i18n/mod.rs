//! Internationalization and multi-language support.

pub mod locales;

use crate::config::Language;
pub use locales::{EN_TRANSLATIONS, RU_TRANSLATIONS, Translations};

/// Returns localized string resources for the selected language.
#[must_use]
pub fn get_translations(lang: Language) -> &'static Translations {
    match lang {
        Language::Ru => &RU_TRANSLATIONS,
        Language::En => &EN_TRANSLATIONS,
    }
}
