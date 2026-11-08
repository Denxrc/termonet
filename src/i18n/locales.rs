//! Localization string catalogs for English and Russian.

/// Complete UI text translations for all screens, headers, widgets, and controls.
pub struct Translations {
    // Navigation tabs
    pub tab_palette: &'static str,
    pub tab_programs: &'static str,
    pub tab_hooks: &'static str,
    pub tab_apply: &'static str,

    // Step 1: Palette
    pub wallpaper_source: &'static str,
    pub no_wallpaper: &'static str,
    pub seed_color: &'static str,
    pub roles_title: &'static str,
    pub terminal_title: &'static str,
    pub normal_colors: &'static str,
    pub bright_colors: &'static str,
    pub role_primary: &'static str,
    pub role_on_primary: &'static str,
    pub role_primary_container: &'static str,
    pub role_secondary: &'static str,
    pub role_tertiary: &'static str,
    pub role_surface: &'static str,
    pub role_surface_variant: &'static str,
    pub role_background: &'static str,
    pub role_outline: &'static str,
    pub role_error: &'static str,

    // Step 2: Programs
    pub programs_title: &'static str,
    pub programs_hint: &'static str,
    pub programs_output: &'static str,

    // Step 3: Hooks
    pub hooks_title: &'static str,
    pub hooks_desc: &'static str,

    // Step 4: Apply
    pub apply_title: &'static str,
    pub apply_button: &'static str,
    pub apply_summary: &'static str,
    pub apply_ready: &'static str,
    pub apply_status: &'static str,

    // Footer hints
    pub hint_next: &'static str,
    pub hint_prev: &'static str,
    pub hint_jump: &'static str,
    pub hint_space: &'static str,
    pub hint_quit: &'static str,
}

pub const RU_TRANSLATIONS: Translations = Translations {
    tab_palette: "1. Палитра",
    tab_programs: "2. Программы",
    tab_hooks: "3. Хуки",
    tab_apply: "4. Применение",

    wallpaper_source: "Источник обоев:",
    no_wallpaper: "Обои не выбраны",
    seed_color: "Доминантный тон (Seed):",
    roles_title: " Цветовые роли Material You ",
    terminal_title: " 16 цветов терминала (ANSI) ",
    normal_colors: " [Базовые цвета 0-7]",
    bright_colors: " [Яркие цвета 8-15]",
    role_primary: "Основной (Primary)",
    role_on_primary: "Текст на основном",
    role_primary_container: "Контейнер основного",
    role_secondary: "Вторичный",
    role_tertiary: "Третичный",
    role_surface: "Поверхность",
    role_surface_variant: "Вариант поверхности",
    role_background: "Фон",
    role_outline: "Граница",
    role_error: "Ошибка",

    programs_title: " Поддерживаемые программы и пути конфигов ",
    programs_hint: "[Пробел] — вкл/выкл, [↑/↓] — выбор программы",
    programs_output: "Путь темы:",

    hooks_title: " Команды перезапуска приложений ",
    hooks_desc: "Все хуки выполняются в фоне без блокировки интерфейса",

    apply_title: " Подтверждение применения темы ",
    apply_button: "▶  [ ПРИМЕНИТЬ ТЕМУ ]  ◀",
    apply_summary: "Цветовые файлы будут сгенерированы для следующих программ:",
    apply_ready: "Готово к применению",
    apply_status: "Статус:",

    hint_next: "След. шаг",
    hint_prev: "Пред. шаг",
    hint_jump: "Перейти к шагу",
    hint_space: "Вкл/Выкл",
    hint_quit: "Выход",
};

pub const EN_TRANSLATIONS: Translations = Translations {
    tab_palette: "1. Palette",
    tab_programs: "2. Programs",
    tab_hooks: "3. Hooks",
    tab_apply: "4. Apply",

    wallpaper_source: "Wallpaper Source:",
    no_wallpaper: "No wallpaper selected",
    seed_color: "Dominant Seed Color:",
    roles_title: " Material You Color Roles ",
    terminal_title: " 16 Terminal ANSI Colors ",
    normal_colors: " [Normal Colors 0-7]",
    bright_colors: " [Bright Colors 8-15]",
    role_primary: "Primary",
    role_on_primary: "On Primary",
    role_primary_container: "Primary Container",
    role_secondary: "Secondary",
    role_tertiary: "Tertiary",
    role_surface: "Surface",
    role_surface_variant: "Surface Variant",
    role_background: "Background",
    role_outline: "Outline",
    role_error: "Error",

    programs_title: " Supported Applications & Config Paths ",
    programs_hint: "[Space] toggle, [↑/↓] navigate",
    programs_output: "Theme file:",

    hooks_title: " Application Reload Hooks ",
    hooks_desc: "All hooks are executed decoupled in background",

    apply_title: " Apply Theme Confirmation ",
    apply_button: "▶  [ APPLY THEME ]  ◀",
    apply_summary: "Theme configs will be generated for following programs:",
    apply_ready: "Ready to apply",
    apply_status: "Status:",

    hint_next: "Next Step",
    hint_prev: "Prev Step",
    hint_jump: "Jump Step",
    hint_space: "Toggle",
    hint_quit: "Quit",
};
