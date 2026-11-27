//! TUI state machine and main interaction loop.

use crate::config::{AppConfig, DisplayServer, FontMode, Language, TuiTheme, WallpaperBackend};
use crate::core::GeneratedPalette;
use crate::error::Result;
use crate::i18n::get_translations;
use crate::template::{ProgramRule, TemplateEngine};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::path::PathBuf;

/// Active navigation step in the top panel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Step {
    #[default]
    Palette = 0,
    Programs = 1,
    Hooks = 2,
    Apply = 3,
}

impl Step {
    /// Advances to the next step.
    #[must_use]
    pub fn next(self) -> Self {
        match self {
            Self::Palette => Self::Programs,
            Self::Programs => Self::Hooks,
            Self::Hooks => Self::Apply,
            Self::Apply => Self::Apply,
        }
    }

    /// Moves back to the previous step.
    #[must_use]
    pub fn previous(self) -> Self {
        match self {
            Self::Palette => Self::Palette,
            Self::Programs => Self::Palette,
            Self::Hooks => Self::Programs,
            Self::Apply => Self::Hooks,
        }
    }

    /// Returns index of the step (0..3).
    #[must_use]
    pub const fn index(self) -> usize {
        self as usize
    }
}

/// First-run configuration wizard steps.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WizardStep {
    #[default]
    Language = 0,
    Appearance = 1,
    Environment = 2,
    Programs = 3,
    Finish = 4,
}

/// Contextual sub-phase within `WizardStep::Environment`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EnvSubStep {
    #[default]
    ServerAndBackend,
    Monitors,
}

impl WizardStep {
    /// Advances to the next wizard step.
    #[must_use]
    pub fn next(self) -> Self {
        match self {
            Self::Language => Self::Appearance,
            Self::Appearance => Self::Environment,
            Self::Environment => Self::Programs,
            Self::Programs => Self::Finish,
            Self::Finish => Self::Finish,
        }
    }

    /// Moves back to the previous wizard step.
    #[must_use]
    pub fn prev(self) -> Self {
        match self {
            Self::Language => Self::Language,
            Self::Appearance => Self::Language,
            Self::Environment => Self::Appearance,
            Self::Programs => Self::Environment,
            Self::Finish => Self::Programs,
        }
    }

    /// Returns index of the step (0..4).
    #[must_use]
    pub const fn index(self) -> usize {
        self as usize
    }
}

/// State tracking for the first-run configuration wizard.
#[derive(Debug, Clone)]
pub struct WizardState {
    pub step: WizardStep,
    pub selected_idx: usize,
    pub config: AppConfig,
    pub original_config: Option<AppConfig>,
    pub available_rules: Vec<ProgramRule>,
    pub opened_from_main: bool,
    pub env_substep: EnvSubStep,
    pub discovered_monitors: Option<Vec<(String, PathBuf)>>,
}

impl WizardState {
    /// Creates a new wizard state initialized with the given base config.
    #[must_use]
    pub fn new(config: AppConfig) -> Self {
        let available_rules = ProgramRule::load_all().unwrap_or_default();
        Self {
            step: WizardStep::Language,
            selected_idx: 0,
            config,
            original_config: None,
            available_rules,
            opened_from_main: false,
            env_substep: EnvSubStep::ServerAndBackend,
            discovered_monitors: None,
        }
    }

    /// Creates a wizard state invoked from within the main running TUI session.
    #[must_use]
    pub fn new_from_main(config: AppConfig) -> Self {
        let available_rules = ProgramRule::load_all().unwrap_or_default();
        Self {
            step: WizardStep::Language,
            selected_idx: 0,
            original_config: Some(config.clone()),
            config,
            available_rules,
            opened_from_main: true,
            env_substep: EnvSubStep::ServerAndBackend,
            discovered_monitors: None,
        }
    }

    /// Returns the number of selectable options for the active wizard step.
    #[must_use]
    pub fn max_options(&self) -> usize {
        match self.step {
            WizardStep::Language => 2,
            WizardStep::Appearance => 5,
            WizardStep::Environment => match self.env_substep {
                EnvSubStep::ServerAndBackend => {
                    2 + self.config.display_server.supported_backends().len()
                }
                EnvSubStep::Monitors => self
                    .discovered_monitors
                    .as_ref()
                    .map(|m| m.len())
                    .unwrap_or(0),
            },
            WizardStep::Programs => self.available_rules.len(),
            WizardStep::Finish => 3,
        }
    }
}

/// Interactive state for the manual wallpaper path input modal.
#[derive(Debug, Clone, Default)]
pub struct WallpaperInputState {
    pub input: String,
    pub cursor_pos: usize,
    pub error_msg: Option<String>,
}

impl WallpaperInputState {
    /// Creates a new wallpaper path input state initialized with current wallpaper path if available.
    #[must_use]
    pub fn new(initial_text: Option<&str>) -> Self {
        let text = initial_text.unwrap_or("").to_string();
        let len = text.len();
        Self {
            input: text,
            cursor_pos: len,
            error_msg: None,
        }
    }
}

/// Represents an application target in the TUI list, including its reload hook.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProgramItem {
    pub name: String,
    pub output_path: String,
    pub enabled: bool,
    pub reload_cmd: Option<String>,
    pub reload_desc: Option<String>,
    pub hook_enabled: bool,
    pub rule: Option<ProgramRule>,
}

/// TUI application state.
pub struct AppState {
    pub current_step: Step,
    pub is_running: bool,
    pub config: AppConfig,
    pub resolved_is_dark: bool,
    pub palette: Option<GeneratedPalette>,
    pub wallpaper_path: Option<String>,
    pub custom_seed_color: Option<crate::core::RgbColor>,
    pub status_message: Option<String>,
    pub status_is_error: bool,

    // Step 2: Programs interactive state
    pub programs: Vec<ProgramItem>,
    pub selected_program_idx: usize,

    // Step 3: Hooks interactive state (pointer into active hooks)
    pub selected_hook_idx: usize,

    // First-run setup wizard state
    pub is_wizard: bool,
    pub wizard_state: Option<WizardState>,

    // Interactive wallpaper path input modal state
    pub wallpaper_input: Option<WallpaperInputState>,

    // Multi-monitor active displays and cycling
    pub active_monitors: Vec<(String, PathBuf)>,
    pub current_monitor_idx: usize,

    // Optional isolated config path override (used in tests)
    pub custom_config_path: Option<PathBuf>,
    // Optional isolated templates directory override (used in tests)
    pub custom_templates_dir: Option<PathBuf>,
}

impl AppState {
    /// Creates program items dynamically from discovered rules or embedded defaults.
    fn create_default_programs(enabled_list: &[String]) -> Vec<ProgramItem> {
        let rules = ProgramRule::load_all().unwrap_or_default();

        rules
            .into_iter()
            .map(|rule| {
                let name_lower = rule.name.to_lowercase();
                let is_enabled = rule.matches_enabled_list(enabled_list);

                let eff_cmd = rule.effective_reload_cmd().map(str::to_string);
                let reload_desc = if eff_cmd.is_some() {
                    match name_lower.as_str() {
                        "kitty" => Some("Live terminal color reload".to_string()),
                        "waybar" => Some("Instant CSS style reload".to_string()),
                        "hyprland" => Some("Window borders & active theme reload".to_string()),
                        "dunst" => Some("Notification daemon reload".to_string()),
                        "sway" => Some("Sway compositor config reload".to_string()),
                        _ => Some("Reload service".to_string()),
                    }
                } else {
                    None
                };

                let hook_default = eff_cmd.is_some();

                ProgramItem {
                    name: rule.name.clone(),
                    output_path: rule.output_path.to_string_lossy().to_string(),
                    enabled: is_enabled,
                    reload_cmd: eff_cmd,
                    reload_desc,
                    hook_enabled: hook_default,
                    rule: Some(rule),
                }
            })
            .collect()
    }

    /// Creates a new TUI application state with given configuration.
    #[must_use]
    pub fn new(config: AppConfig) -> Self {
        let programs = Self::create_default_programs(&config.enabled_programs);
        let resolved_is_dark = config.tui_theme.is_dark();

        Self {
            current_step: Step::Palette,
            is_running: true,
            config,
            resolved_is_dark,
            palette: None,
            wallpaper_path: None,
            custom_seed_color: None,
            status_message: None,
            status_is_error: false,
            programs,
            selected_program_idx: 0,
            selected_hook_idx: 0,
            is_wizard: false,
            wizard_state: None,
            wallpaper_input: None,
            active_monitors: Vec::new(),
            current_monitor_idx: 0,
            custom_config_path: None,
            custom_templates_dir: None,
        }
    }

    /// Creates a new TUI application state started directly in First-Run Wizard mode.
    #[must_use]
    pub fn new_wizard(config: AppConfig) -> Self {
        let programs = Self::create_default_programs(&config.enabled_programs);
        let resolved_is_dark = config.tui_theme.is_dark();
        let wizard_state = WizardState::new(config.clone());

        Self {
            current_step: Step::Palette,
            is_running: true,
            config,
            resolved_is_dark,
            palette: None,
            wallpaper_path: None,
            custom_seed_color: None,
            status_message: None,
            status_is_error: false,
            programs,
            selected_program_idx: 0,
            selected_hook_idx: 0,
            is_wizard: true,
            wizard_state: Some(wizard_state),
            wallpaper_input: None,
            active_monitors: Vec::new(),
            current_monitor_idx: 0,
            custom_config_path: None,
            custom_templates_dir: None,
        }
    }

    /// Re-evaluates cached dark/light state from the current `tui_theme` setting.
    pub fn refresh_theme_mode(&mut self) {
        self.resolved_is_dark = self.config.tui_theme.is_dark();
    }

    fn set_status(&mut self, msg: String, is_error: bool) {
        self.status_message = Some(msg);
        self.status_is_error = is_error;
    }

    /// Finalizes the wizard, saves `~/.config/termonet/config.toml`, and transitions to main TUI.
    pub fn finish_wizard(&mut self) -> Result<()> {
        if let Some(ref wizard) = self.wizard_state {
            let saved_config = wizard.config.clone();
            let t = get_translations(saved_config.language);
            let save_res = if let Some(ref custom_path) = self.custom_config_path {
                saved_config.save_to_path(custom_path)
            } else {
                saved_config.save().map(|_| ())
            };
            if let Err(e) = save_res {
                self.set_status(format!("{} {}", t.status_config_save_error, e), true);
                return Err(e);
            }
            self.config = saved_config;
            self.refresh_theme_mode();

            // Synchronize program items with the finalized configuration
            for prog in &mut self.programs {
                prog.enabled = self
                    .config
                    .enabled_programs
                    .iter()
                    .any(|p| p.eq_ignore_ascii_case(&prog.name));
            }
            self.ensure_hook_index_valid();

            self.is_wizard = false;
            self.wizard_state = None;

            // Automatically attempt wallpaper detection and palette generation with new settings
            if let Err(e) = self.reload_wallpaper() {
                self.set_status(format!("{} {})", t.status_config_saved_wall_err, e), false);
            } else {
                self.set_status(t.wizard_saved_success.to_string(), false);
            }
        }
        Ok(())
    }

    /// Opens the settings wizard on demand from the active main TUI screen.
    pub fn open_settings(&mut self) {
        let wizard = WizardState::new_from_main(self.config.clone());
        self.wizard_state = Some(wizard);
        self.is_wizard = true;
    }

    /// Opens the manual wallpaper path input dialog.
    pub fn open_wallpaper_input(&mut self) {
        let current_path = if self.is_wizard {
            self.wizard_state
                .as_ref()
                .and_then(|w| w.config.manual_wallpaper_path.as_deref())
                .or(self.wallpaper_path.as_deref())
        } else {
            self.wallpaper_path
                .as_deref()
                .or(self.config.manual_wallpaper_path.as_deref())
        };
        self.wallpaper_input = Some(WallpaperInputState::new(current_path));
    }

    /// Attempts to apply the entered wallpaper path or HEX color, generating a new palette.
    pub fn apply_manual_wallpaper(&mut self, raw_path: &str) -> bool {
        let t = get_translations(self.config.language);
        let trimmed = raw_path.trim();
        if trimmed.is_empty() {
            if let Some(ref mut input_state) = self.wallpaper_input {
                input_state.error_msg = Some(t.status_input_empty.to_string());
            }
            return false;
        }

        let explicit_hex =
            trimmed.starts_with('#') || trimmed.starts_with("0x") || trimmed.starts_with("0X");

        let resolved_candidate = crate::wallpaper::clean_wallpaper_path(trimmed)
            .unwrap_or_else(|| crate::config::paths::AppPaths::expand_tilde(trimmed));

        // 1. Check if input is a HEX color (unless an explicit existing file matches a bare hex name)
        if (explicit_hex || !resolved_candidate.is_file())
            && let Ok(color) = crate::core::RgbColor::from_hex(trimmed)
        {
            let palette = GeneratedPalette::from_seed_color(color.into());
            self.palette = Some(palette);
            self.custom_seed_color = Some(color);
            self.wallpaper_path = None;
            self.set_status(
                format!("{} {}", t.status_seed_applied, color.to_hex()),
                false,
            );
            self.wallpaper_input = None;
            return true;
        }

        // 2. Otherwise treat as wallpaper file path
        if !resolved_candidate.exists() {
            if let Some(ref mut input_state) = self.wallpaper_input {
                input_state.error_msg = Some(format!(
                    "{}: {}",
                    t.wallpaper_error_not_found,
                    resolved_candidate.display()
                ));
            }
            return false;
        }

        match GeneratedPalette::from_image_path(&resolved_candidate) {
            Ok(palette) => {
                let path_str = resolved_candidate.display().to_string();
                self.palette = Some(palette);
                self.wallpaper_path = Some(path_str.clone());
                self.custom_seed_color = None;

                if self.is_wizard {
                    if let Some(ref mut wizard) = self.wizard_state {
                        wizard.config.manual_wallpaper_path = Some(path_str);
                        wizard.config.wallpaper_backend = WallpaperBackend::Manual;
                    }
                } else {
                    self.config.manual_wallpaper_path = Some(path_str);
                    self.config.wallpaper_backend = WallpaperBackend::Manual;
                    let _ = self.config.save();
                }

                self.set_status(t.wallpaper_applied_success.to_string(), false);
                self.wallpaper_input = None;
                true
            }
            Err(e) => {
                if let Some(ref mut input_state) = self.wallpaper_input {
                    input_state.error_msg = Some(format!("{}: {}", t.wallpaper_error_decode, e));
                }
                false
            }
        }
    }

    /// Automatically detects and reloads the active wallpaper based on current configuration.
    pub fn reload_wallpaper(&mut self) -> Result<()> {
        let t = get_translations(self.config.language);
        let manual_str = self.config.manual_wallpaper_path.as_deref();
        let path = if self.config.wallpaper_backend != WallpaperBackend::Manual {
            let monitors = crate::wallpaper::get_wallpaper_monitors(
                &self.config.wallpaper_backend,
                manual_str,
            )?;
            if monitors.has_divergent_wallpapers() {
                let sorted = monitors.sorted_outputs();
                self.active_monitors = sorted;
                self.current_monitor_idx = if let Some(ref primary) = self.config.primary_monitor {
                    self.active_monitors
                        .iter()
                        .position(|(name, _)| name == primary)
                        .unwrap_or(0)
                } else {
                    0
                };
                self.active_monitors[self.current_monitor_idx].1.clone()
            } else {
                self.active_monitors.clear();
                self.current_monitor_idx = 0;
                monitors
                    .resolve_primary(self.config.primary_monitor.as_deref())
                    .ok_or_else(|| {
                        crate::error::TermonetError::WallpaperNotFound(
                            "No active wallpaper detected".into(),
                        )
                    })?
            }
        } else {
            self.active_monitors.clear();
            self.current_monitor_idx = 0;
            crate::wallpaper::get_current_wallpaper(
                &self.config.wallpaper_backend,
                manual_str,
                self.config.primary_monitor.as_deref(),
            )?
        };

        let palette = GeneratedPalette::from_image_path(&path)?;
        let path_str = path.display().to_string();
        self.palette = Some(palette);
        self.wallpaper_path = Some(path_str);
        self.custom_seed_color = None;
        self.set_status(t.status_wallpaper_reloaded.to_string(), false);
        Ok(())
    }

    /// Cycles through active multi-monitor outputs on Step 1 (Palette), previewing the wallpaper in memory.
    pub fn cycle_monitor_wallpaper(&mut self) {
        if self.active_monitors.len() <= 1 {
            return;
        }
        let t = get_translations(self.config.language);

        self.current_monitor_idx = (self.current_monitor_idx + 1) % self.active_monitors.len();
        let (name, path) = self.active_monitors[self.current_monitor_idx].clone();
        self.config.primary_monitor = Some(name.clone());

        match GeneratedPalette::from_image_path(&path) {
            Ok(palette) => {
                self.wallpaper_path = Some(path.display().to_string());
                self.custom_seed_color = None;
                self.palette = Some(palette);
                self.set_status(format!("{} {}", t.status_monitor_switched, name), false);
            }
            Err(e) => {
                self.set_status(
                    format!("{} ({}): {}", t.wallpaper_error_decode, name, e),
                    true,
                );
            }
        }
    }

    /// Switches to automatic wallpaper detection mode, persists the setting to config,
    /// and generates a new palette from the actively detected wallpaper.
    pub fn switch_to_auto_and_reload(&mut self) -> Result<()> {
        self.config.wallpaper_backend = WallpaperBackend::Auto;
        let _ = self.config.save();
        self.reload_wallpaper()
    }

    /// Advances to the next step.
    pub fn next_step(&mut self) {
        self.current_step = self.current_step.next();
        self.ensure_hook_index_valid();
    }

    /// Returns to the previous step.
    pub fn prev_step(&mut self) {
        self.current_step = self.current_step.previous();
        self.ensure_hook_index_valid();
    }

    /// Sets the current active step directly.
    pub fn set_step(&mut self, step: Step) {
        self.current_step = step;
        self.ensure_hook_index_valid();
    }

    /// Quits the TUI event loop.
    pub fn quit(&mut self) {
        self.is_running = false;
    }

    /// Returns list of indices of programs that are enabled and have an associated reload command.
    pub fn active_hook_indices(&self) -> Vec<usize> {
        self.programs
            .iter()
            .enumerate()
            .filter(|(_, p)| p.enabled && p.reload_cmd.is_some())
            .map(|(idx, _)| idx)
            .collect()
    }

    /// Returns the number of programs that are enabled and have an associated reload command.
    #[must_use]
    pub fn active_hook_count(&self) -> usize {
        self.programs
            .iter()
            .filter(|p| p.enabled && p.reload_cmd.is_some())
            .count()
    }

    fn ensure_hook_index_valid(&mut self) {
        let active_count = self.active_hook_count();
        if active_count == 0 {
            self.selected_hook_idx = 0;
        } else if self.selected_hook_idx >= active_count {
            self.selected_hook_idx = active_count.saturating_sub(1);
        }
    }

    /// Toggles the currently selected program item in Step 2.
    pub fn toggle_current_program(&mut self) {
        if let Some(item) = self.programs.get_mut(self.selected_program_idx) {
            item.enabled = !item.enabled;
        }
        self.ensure_hook_index_valid();
    }

    /// Toggles the currently selected hook item in Step 3.
    pub fn toggle_current_hook(&mut self) {
        let active = self.active_hook_indices();
        if let Some(&prog_idx) = active.get(self.selected_hook_idx)
            && let Some(p) = self.programs.get_mut(prog_idx)
        {
            p.hook_enabled = !p.hook_enabled;
        }
    }

    /// Toggles or updates the selected setting in the current wizard step.
    pub fn toggle_wizard_option(&mut self) {
        let mut theme_changed = false;
        {
            let Some(wizard) = &mut self.wizard_state else {
                return;
            };

            match wizard.step {
                WizardStep::Language => {
                    match wizard.selected_idx {
                        0 => wizard.config.language = Language::Ru,
                        1 => wizard.config.language = Language::En,
                        _ => {}
                    }
                    self.config.language = wizard.config.language;
                }
                WizardStep::Appearance => {
                    match wizard.selected_idx {
                        0 => {
                            wizard.config.tui_theme = TuiTheme::Dark;
                            theme_changed = true;
                        }
                        1 => {
                            wizard.config.tui_theme = TuiTheme::Light;
                            theme_changed = true;
                        }
                        2 => {
                            wizard.config.tui_theme = TuiTheme::System;
                            theme_changed = true;
                        }
                        3 => wizard.config.font_mode = FontMode::Pixel,
                        4 => wizard.config.font_mode = FontMode::Standard,
                        _ => {}
                    }
                    self.config.tui_theme = wizard.config.tui_theme;
                    self.config.font_mode = wizard.config.font_mode;
                }
                WizardStep::Environment => match wizard.env_substep {
                    EnvSubStep::ServerAndBackend => {
                        let backends = wizard.config.display_server.supported_backends();
                        if wizard.selected_idx == 0 {
                            wizard.config.display_server = DisplayServer::Wayland;
                            if !DisplayServer::Wayland
                                .supported_backends()
                                .contains(&wizard.config.wallpaper_backend)
                            {
                                wizard.config.wallpaper_backend = WallpaperBackend::Auto;
                            }
                        } else if wizard.selected_idx == 1 {
                            wizard.config.display_server = DisplayServer::X11;
                            if !DisplayServer::X11
                                .supported_backends()
                                .contains(&wizard.config.wallpaper_backend)
                            {
                                wizard.config.wallpaper_backend = WallpaperBackend::Auto;
                            }
                        } else {
                            let backend_idx = wizard.selected_idx.saturating_sub(2);
                            if let Some(&backend) = backends.get(backend_idx) {
                                wizard.config.wallpaper_backend = backend;
                            }
                        }
                        wizard.selected_idx = wizard
                            .selected_idx
                            .min(wizard.max_options().saturating_sub(1));
                        wizard.discovered_monitors = None;
                    }
                    EnvSubStep::Monitors => {
                        if let Some(ref monitors) = wizard.discovered_monitors
                            && let Some((name, _)) = monitors.get(wizard.selected_idx)
                        {
                            wizard.config.primary_monitor = Some(name.clone());
                        }
                    }
                },
                WizardStep::Programs => {
                    if let Some(rule) = wizard.available_rules.get(wizard.selected_idx) {
                        if let Some(pos) = wizard
                            .config
                            .enabled_programs
                            .iter()
                            .position(|p| p.eq_ignore_ascii_case(&rule.name))
                        {
                            wizard.config.enabled_programs.remove(pos);
                        } else {
                            wizard
                                .config
                                .enabled_programs
                                .push(rule.name.to_lowercase());
                        }
                    }
                }
                WizardStep::Finish => {
                    match wizard.selected_idx {
                        0 => wizard.config.create_backups = false,
                        1 => wizard.config.create_backups = true,
                        _ => {}
                    }
                    self.config.create_backups = wizard.config.create_backups;
                }
            }
        }
        if theme_changed {
            self.refresh_theme_mode();
        }
    }

    /// Handles keyboard interaction when the First-Run Wizard is active.
    pub fn handle_wizard_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('q') => self.quit(),
            KeyCode::Esc => {
                if let Some(ref mut w) = self.wizard_state
                    && w.opened_from_main
                {
                    // Cancel and restore original in-memory config without modifying saved config
                    if let Some(orig) = w.original_config.take() {
                        self.config = orig;
                    }
                    self.refresh_theme_mode();
                    self.is_wizard = false;
                    self.wizard_state = None;
                } else {
                    self.quit();
                }
            }
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => self.quit(),

            // Wizard step navigation
            KeyCode::Right | KeyCode::Char('l') => self.handle_wizard_next(),
            KeyCode::Left | KeyCode::Char('h') => self.handle_wizard_prev(),
            KeyCode::Char('1') => {
                if let Some(ref mut wizard) = self.wizard_state {
                    wizard.step = WizardStep::Language;
                    wizard.selected_idx = 0;
                    wizard.env_substep = EnvSubStep::ServerAndBackend;
                }
            }
            KeyCode::Char('2') => {
                if let Some(ref mut wizard) = self.wizard_state {
                    wizard.step = WizardStep::Appearance;
                    wizard.selected_idx = 0;
                    wizard.env_substep = EnvSubStep::ServerAndBackend;
                }
            }
            KeyCode::Char('3') => {
                if let Some(ref mut wizard) = self.wizard_state {
                    wizard.step = WizardStep::Environment;
                    wizard.selected_idx = 0;
                    wizard.env_substep = EnvSubStep::ServerAndBackend;
                }
            }
            KeyCode::Char('4') => {
                if let Some(ref mut wizard) = self.wizard_state {
                    wizard.step = WizardStep::Programs;
                    wizard.selected_idx = 0;
                }
            }
            KeyCode::Char('5') => {
                if let Some(ref mut wizard) = self.wizard_state {
                    wizard.step = WizardStep::Finish;
                    wizard.selected_idx = 2;
                }
            }

            // Move cursor between options inside active step
            KeyCode::Up | KeyCode::Char('k') => {
                if let Some(ref mut wizard) = self.wizard_state
                    && wizard.selected_idx > 0
                {
                    wizard.selected_idx -= 1;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if let Some(ref mut wizard) = self.wizard_state {
                    let max = wizard.max_options();
                    if max > 0 && wizard.selected_idx + 1 < max {
                        wizard.selected_idx += 1;
                    }
                }
            }

            // Select or toggle current option
            KeyCode::Char(' ') => {
                let current_step = self.wizard_state.as_ref().map(|w| w.step);
                let selected_idx = self
                    .wizard_state
                    .as_ref()
                    .map(|w| w.selected_idx)
                    .unwrap_or(0);
                if current_step == Some(WizardStep::Finish) && selected_idx == 2 {
                    if let Err(e) = self.finish_wizard() {
                        let t = get_translations(self.config.language);
                        self.set_status(format!("{} {}", t.status_error_prefix, e), true);
                    }
                } else {
                    self.toggle_wizard_option();
                }
            }

            KeyCode::Char('w') => {
                let in_env_backend = self.wizard_state.as_ref().is_some_and(|w| {
                    w.step == WizardStep::Environment
                        && w.env_substep == EnvSubStep::ServerAndBackend
                });
                if in_env_backend {
                    self.open_wallpaper_input();
                }
            }

            KeyCode::Enter => self.handle_wizard_enter(),
            _ => {}
        }
    }

    fn handle_wizard_next(&mut self) {
        if let Some(ref mut wizard) = self.wizard_state {
            if wizard.step == WizardStep::Environment {
                match wizard.env_substep {
                    EnvSubStep::ServerAndBackend => {
                        let manual_str = wizard.config.manual_wallpaper_path.as_deref();
                        match crate::wallpaper::get_wallpaper_monitors(
                            &wizard.config.wallpaper_backend,
                            manual_str,
                        ) {
                            Ok(monitors) if monitors.has_divergent_wallpapers() => {
                                let sorted = monitors.sorted_outputs();
                                let cur_idx =
                                    if let Some(ref primary) = wizard.config.primary_monitor {
                                        sorted
                                            .iter()
                                            .position(|(name, _)| name == primary)
                                            .unwrap_or(0)
                                    } else {
                                        if let Some(first) = sorted.first() {
                                            wizard.config.primary_monitor = Some(first.0.clone());
                                        }
                                        0
                                    };
                                wizard.discovered_monitors = Some(sorted);
                                wizard.env_substep = EnvSubStep::Monitors;
                                wizard.selected_idx = cur_idx;
                            }
                            _ => {
                                wizard.discovered_monitors = None;
                                wizard.env_substep = EnvSubStep::ServerAndBackend;
                                wizard.step = WizardStep::Programs;
                                wizard.selected_idx = 0;
                            }
                        }
                    }
                    EnvSubStep::Monitors => {
                        wizard.step = WizardStep::Programs;
                        wizard.selected_idx = 0;
                    }
                }
            } else {
                wizard.step = wizard.step.next();
                wizard.selected_idx = if wizard.step == WizardStep::Finish {
                    2
                } else {
                    0
                };
            }
        }
    }

    fn handle_wizard_prev(&mut self) {
        if let Some(ref mut wizard) = self.wizard_state {
            if wizard.step == WizardStep::Programs {
                if wizard.discovered_monitors.is_some() {
                    wizard.step = WizardStep::Environment;
                    wizard.env_substep = EnvSubStep::Monitors;
                    wizard.selected_idx = wizard
                        .config
                        .primary_monitor
                        .as_ref()
                        .and_then(|p| {
                            wizard
                                .discovered_monitors
                                .as_ref()?
                                .iter()
                                .position(|(n, _)| n == p)
                        })
                        .unwrap_or(0);
                } else {
                    wizard.step = WizardStep::Environment;
                    wizard.env_substep = EnvSubStep::ServerAndBackend;
                    wizard.selected_idx = 0;
                }
            } else if wizard.step == WizardStep::Environment {
                match wizard.env_substep {
                    EnvSubStep::Monitors => {
                        wizard.env_substep = EnvSubStep::ServerAndBackend;
                        wizard.selected_idx = 0;
                    }
                    EnvSubStep::ServerAndBackend => {
                        wizard.step = WizardStep::Appearance;
                        wizard.selected_idx = 0;
                    }
                }
            } else {
                wizard.step = wizard.step.prev();
                wizard.selected_idx = if wizard.step == WizardStep::Finish {
                    2
                } else {
                    0
                };
            }
        }
    }

    fn handle_wizard_enter(&mut self) {
        let current_step = self.wizard_state.as_ref().map(|w| w.step);
        let selected_idx = self
            .wizard_state
            .as_ref()
            .map(|w| w.selected_idx)
            .unwrap_or(0);

        if current_step == Some(WizardStep::Finish) {
            if selected_idx == 2 {
                if let Err(e) = self.finish_wizard() {
                    let t = get_translations(self.config.language);
                    self.set_status(format!("{} {}", t.status_error_prefix, e), true);
                }
            } else {
                self.toggle_wizard_option();
                if let Some(ref mut wizard) = self.wizard_state {
                    wizard.selected_idx = 2;
                }
            }
        } else if current_step == Some(WizardStep::Environment)
            && self.wizard_state.as_ref().map(|w| w.env_substep)
                == Some(EnvSubStep::ServerAndBackend)
        {
            let is_manual = if let Some(ref w) = self.wizard_state {
                if w.selected_idx >= 2 {
                    let backends = w.config.display_server.supported_backends();
                    backends.get(w.selected_idx - 2) == Some(&WallpaperBackend::Manual)
                } else {
                    false
                }
            } else {
                false
            };

            self.toggle_wizard_option();
            if is_manual {
                self.open_wallpaper_input();
            }
        } else {
            // Toggle current option without jumping screens
            self.toggle_wizard_option();
        }
    }

    /// Handles keyboard interaction when typing a manual wallpaper path.
    pub fn handle_wallpaper_input_key(&mut self, key: KeyEvent) {
        let Some(ref mut input_state) = self.wallpaper_input else {
            return;
        };

        match key.code {
            KeyCode::Esc => {
                self.wallpaper_input = None;
            }
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.wallpaper_input = None;
            }
            KeyCode::Enter => {
                let input = input_state.input.clone();
                self.apply_manual_wallpaper(&input);
            }
            KeyCode::Backspace => {
                if input_state.cursor_pos > 0
                    && let Some((prev_idx, _)) = input_state.input[..input_state.cursor_pos]
                        .char_indices()
                        .last()
                {
                    input_state.input.remove(prev_idx);
                    input_state.cursor_pos = prev_idx;
                    input_state.error_msg = None;
                }
            }
            KeyCode::Delete => {
                if input_state.cursor_pos < input_state.input.len() {
                    input_state.input.remove(input_state.cursor_pos);
                    input_state.error_msg = None;
                }
            }
            KeyCode::Left => {
                if input_state.cursor_pos > 0
                    && let Some((prev_idx, _)) = input_state.input[..input_state.cursor_pos]
                        .char_indices()
                        .last()
                {
                    input_state.cursor_pos = prev_idx;
                }
            }
            KeyCode::Right => {
                if input_state.cursor_pos < input_state.input.len() {
                    if let Some((next_offset, _)) = input_state.input[input_state.cursor_pos..]
                        .char_indices()
                        .nth(1)
                    {
                        input_state.cursor_pos += next_offset;
                    } else {
                        input_state.cursor_pos = input_state.input.len();
                    }
                }
            }
            KeyCode::Home => {
                input_state.cursor_pos = 0;
            }
            KeyCode::End => {
                input_state.cursor_pos = input_state.input.len();
            }
            KeyCode::Char(c) => {
                input_state.input.insert(input_state.cursor_pos, c);
                input_state.cursor_pos += c.len_utf8();
                input_state.error_msg = None;
            }
            _ => {}
        }
    }

    /// Renders templates for all enabled programs, safely writes them, and executes reload hooks in background.
    pub fn apply_theme(&mut self) -> Result<()> {
        let t = get_translations(self.config.language);
        let Some(ref palette) = self.palette else {
            return Err(crate::error::TermonetError::Config(
                t.status_no_palette.into(),
            ));
        };

        let engine = TemplateEngine::new();
        let is_dark = self.is_dark_mode();

        // 1. Render and apply each enabled program, spawning reload hooks without blocking UI
        for prog in &self.programs {
            if prog.enabled
                && let Some(ref rule) = prog.rule
            {
                if let Some(ref tpl_dir) = self.custom_templates_dir {
                    rule.apply_with_hook_detached_from(
                        &engine,
                        palette,
                        is_dark,
                        self.config.create_backups,
                        prog.hook_enabled,
                        Some(tpl_dir.as_path()),
                    )?;
                } else {
                    rule.apply_with_hook_detached(
                        &engine,
                        palette,
                        is_dark,
                        self.config.create_backups,
                        prog.hook_enabled,
                    )?;
                }
            }
        }

        // 2. Synchronize enabled programs into configuration before saving
        self.config.enabled_programs = self
            .programs
            .iter()
            .filter(|p| p.enabled)
            .map(|p| p.name.to_lowercase())
            .collect();

        // 3. Save configuration (persisting any monitor switch made during palette preview)
        let _ = self.config.save();

        // 4. Update state cache
        let config_fp = Some(crate::config::AppStateCache::compute_config_fingerprint(
            &self.config.enabled_programs,
            crate::config::AppPaths::rules_dir().ok().as_deref(),
            crate::config::AppPaths::templates_dir().ok().as_deref(),
        ));
        let mut cache = crate::config::AppStateCache::load().unwrap_or_default();
        if let Some(color) = self.custom_seed_color {
            cache.update_source_with_fingerprint(
                &crate::config::CacheSource::Color(color),
                is_dark,
                &color,
                config_fp,
            );
            let _ = cache.save();
        } else if let Some(ref wall_path) = self.wallpaper_path {
            cache.update_source_with_fingerprint(
                &crate::config::CacheSource::Wallpaper(std::path::Path::new(wall_path)),
                is_dark,
                &palette.seed_color,
                config_fp,
            );
            let _ = cache.save();
        }

        self.set_status(t.status_theme_applied.to_string(), false);
        Ok(())
    }

    /// Returns true if the active theme mode resolves to Dark (cached in `AppState`).
    #[must_use]
    pub const fn is_dark_mode(&self) -> bool {
        self.resolved_is_dark
    }

    /// Returns the active MaterialScheme based on configuration and system settings.
    #[must_use]
    pub fn active_scheme(&self) -> Option<&crate::core::MaterialScheme> {
        self.palette
            .as_ref()
            .map(|p| p.scheme_for(self.is_dark_mode()))
    }

    /// Returns the active TerminalPalette based on configuration and system settings.
    #[must_use]
    pub fn active_terminal(&self) -> Option<&crate::core::TerminalPalette> {
        self.palette
            .as_ref()
            .map(|p| p.terminal_for(self.is_dark_mode()))
    }

    /// Primary and secondary UI accent colors for TUI rendering.
    #[must_use]
    pub fn ui_colors(&self) -> (ratatui::style::Color, ratatui::style::Color) {
        if let Some(scheme) = self.active_scheme() {
            (
                ratatui::style::Color::from(scheme.primary),
                ratatui::style::Color::from(scheme.secondary),
            )
        } else {
            (
                ratatui::style::Color::Rgb(173, 198, 255),
                ratatui::style::Color::Rgb(191, 198, 220),
            )
        }
    }

    /// Primary text color (high contrast against active theme background).
    #[must_use]
    pub fn text_color(&self) -> ratatui::style::Color {
        if self.is_dark_mode() {
            ratatui::style::Color::White
        } else if let Some(scheme) = self.active_scheme() {
            ratatui::style::Color::from(scheme.on_surface)
        } else {
            ratatui::style::Color::Rgb(25, 28, 32)
        }
    }

    /// Secondary / muted text color (medium contrast).
    #[must_use]
    pub fn text_muted(&self) -> ratatui::style::Color {
        if self.is_dark_mode() {
            ratatui::style::Color::Gray
        } else {
            ratatui::style::Color::Rgb(70, 75, 80)
        }
    }

    /// Dim / disabled text color (low contrast).
    #[must_use]
    pub fn text_dim(&self) -> ratatui::style::Color {
        if self.is_dark_mode() {
            ratatui::style::Color::DarkGray
        } else {
            ratatui::style::Color::Rgb(115, 120, 125)
        }
    }

    /// Handles keyboard events.
    pub fn handle_key(&mut self, key: KeyEvent) {
        if self.wallpaper_input.is_some() {
            self.handle_wallpaper_input_key(key);
            return;
        }

        if self.is_wizard {
            self.handle_wizard_key(key);
            return;
        }

        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => self.quit(),
            KeyCode::Char('s') | KeyCode::F(2) => self.open_settings(),
            KeyCode::Char('w') | KeyCode::Char('o') if self.current_step == Step::Palette => {
                self.open_wallpaper_input();
            }
            KeyCode::Char('r') if self.current_step == Step::Palette => {
                if let Err(e) = self.switch_to_auto_and_reload() {
                    let t = get_translations(self.config.language);
                    self.set_status(format!("{} {}", t.status_auto_detect_error, e), true);
                }
            }
            KeyCode::Tab | KeyCode::Char('m') if self.current_step == Step::Palette => {
                self.cycle_monitor_wallpaper();
            }
            KeyCode::Right | KeyCode::Char('l') => self.next_step(),
            KeyCode::Left | KeyCode::Char('h') => self.prev_step(),
            KeyCode::Char('1') => self.set_step(Step::Palette),
            KeyCode::Char('2') => self.set_step(Step::Programs),
            KeyCode::Char('3') => self.set_step(Step::Hooks),
            KeyCode::Char('4') => self.set_step(Step::Apply),

            // Navigation within lists
            KeyCode::Up | KeyCode::Char('k') => match self.current_step {
                Step::Programs if self.selected_program_idx > 0 => {
                    self.selected_program_idx -= 1;
                }
                Step::Hooks if self.selected_hook_idx > 0 => {
                    self.selected_hook_idx -= 1;
                }
                _ => {}
            },
            KeyCode::Down | KeyCode::Char('j') => match self.current_step {
                Step::Programs if self.selected_program_idx + 1 < self.programs.len() => {
                    self.selected_program_idx += 1;
                }
                Step::Hooks => {
                    let active_len = self.active_hook_count();
                    if active_len > 0 && self.selected_hook_idx + 1 < active_len {
                        self.selected_hook_idx += 1;
                    }
                }
                _ => {}
            },
            KeyCode::Char(' ') => match self.current_step {
                Step::Programs => self.toggle_current_program(),
                Step::Hooks => self.toggle_current_hook(),
                _ => {}
            },
            KeyCode::Enter => {
                if self.current_step == Step::Apply
                    && let Err(e) = self.apply_theme()
                {
                    let t = get_translations(self.config.language);
                    self.set_status(format!("{} {}", t.status_error_prefix, e), true);
                }
            }
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => self.quit(),
            _ => {}
        }
    }
}
