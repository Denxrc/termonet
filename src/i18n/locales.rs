//! Localization string catalogs for English and Russian.

/// UI text translations for all screens and messages.
pub struct Translations {
    pub tab_palette: &'static str,
    pub tab_programs: &'static str,
    pub tab_hooks: &'static str,
    pub tab_apply: &'static str,
    pub prompt_press_q: &'static str,
}

pub const RU_TRANSLATIONS: Translations = Translations {
    tab_palette: "1. Палитра",
    tab_programs: "2. Программы",
    tab_hooks: "3. Хуки",
    tab_apply: "4. Применение",
    prompt_press_q: "Нажмите 'q' для выхода, Tab для смены шага",
};

pub const EN_TRANSLATIONS: Translations = Translations {
    tab_palette: "1. Palette",
    tab_programs: "2. Programs",
    tab_hooks: "3. Hooks",
    tab_apply: "4. Apply",
    prompt_press_q: "Press 'q' to quit, Tab to change step",
};
