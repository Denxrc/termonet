//! TUI state machine and main interaction loop.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use crate::config::{AppConfig, DisplayServer, FontMode, Language, TuiTheme, WallpaperBackend};
use crate::core::GeneratedPalette;
use crate::error::Result;
use crate::template::{ProgramRule, TemplateEngine};

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
    pub opened_from_main: bool,
}

impl WizardState {
    /// Creates a new wizard state initialized with the given base config.
    #[must_use]
    pub fn new(config: AppConfig) -> Self {
        Self {
            step: WizardStep::Language,
            selected_idx: 0,
            config,
            opened_from_main: false,
        }
    }

    /// Creates a wizard state invoked from within the main running TUI session.
    #[must_use]
    pub fn new_from_main(config: AppConfig) -> Self {
        Self {
            step: WizardStep::Language,
            selected_idx: 0,
            config,
            opened_from_main: true,
        }
    }

    /// Returns the number of selectable options for the active wizard step.
    #[must_use]
    pub fn max_options(&self) -> usize {
        match self.step {
            WizardStep::Language => 2,
            WizardStep::Appearance => 5,
            WizardStep::Environment => 13,
            WizardStep::Programs => crate::template::ProgramRule::load_all().map(|r| r.len()).unwrap_or(4),
            WizardStep::Finish => 1,
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
    pub palette: Option<GeneratedPalette>,
    pub wallpaper_path: Option<String>,
    pub status_message: Option<String>,

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
}

impl AppState {
    /// Creates program items dynamically from discovered rules or embedded defaults.
    fn create_default_programs(enabled_list: &[String]) -> Vec<ProgramItem> {
        let rules = ProgramRule::load_all().unwrap_or_default();

        rules
            .into_iter()
            .map(|rule| {
                let name_lower = rule.name.to_lowercase();
                let is_enabled = if enabled_list.is_empty() {
                    matches!(name_lower.as_str(), "kitty" | "waybar" | "rofi" | "hyprland")
                } else {
                    enabled_list.iter().any(|p| p.to_lowercase() == name_lower)
                };

                let reload_desc = match name_lower.as_str() {
                    "kitty" => Some("Live terminal color reload".to_string()),
                    "waybar" => Some("Instant CSS style reload".to_string()),
                    "hyprland" => Some("Window borders & active theme reload".to_string()),
                    "dunst" => Some("Notification daemon reload".to_string()),
                    "sway" => Some("Sway compositor config reload".to_string()),
                    _ => rule.reload_cmd.as_ref().map(|_| "Reload service".to_string()),
                };

                let hook_default = rule.reload_cmd.is_some();

                ProgramItem {
                    name: rule.name.clone(),
                    output_path: rule.output_path.to_string_lossy().to_string(),
                    enabled: is_enabled,
                    reload_cmd: rule.reload_cmd.clone(),
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

        Self {
            current_step: Step::Palette,
            is_running: true,
            config,
            palette: None,
            wallpaper_path: None,
            status_message: None,
            programs,
            selected_program_idx: 0,
            selected_hook_idx: 0,
            is_wizard: false,
            wizard_state: None,
            wallpaper_input: None,
        }
    }

    /// Creates a new TUI application state started directly in First-Run Wizard mode.
    #[must_use]
    pub fn new_wizard(config: AppConfig) -> Self {
        let programs = Self::create_default_programs(&config.enabled_programs);
        let wizard_state = WizardState::new(config.clone());

        Self {
            current_step: Step::Palette,
            is_running: true,
            config,
            palette: None,
            wallpaper_path: None,
            status_message: None,
            programs,
            selected_program_idx: 0,
            selected_hook_idx: 0,
            is_wizard: true,
            wizard_state: Some(wizard_state),
            wallpaper_input: None,
        }
    }

    /// Finalizes the wizard, saves `~/.config/termonet/config.toml`, and transitions to main TUI.
    pub fn finish_wizard(&mut self) -> Result<()> {
        if let Some(ref wizard) = self.wizard_state {
            let saved_config = wizard.config.clone();
            saved_config.save()?;
            self.config = saved_config;

            // Synchronize program items with the finalized configuration
            for prog in &mut self.programs {
                let name_lower = prog.name.to_lowercase();
                prog.enabled = self.config.enabled_programs.iter().any(|p| p.to_lowercase() == name_lower);
            }

            self.is_wizard = false;
            self.wizard_state = None;
            self.status_message = Some("Configuration saved successfully!".to_string());
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
        let current_path = self.wallpaper_path.as_deref().or(self.config.manual_wallpaper_path.as_deref());
        self.wallpaper_input = Some(WallpaperInputState::new(current_path));
    }

    /// Attempts to apply the entered wallpaper path, generating a new palette and saving config.
    pub fn apply_manual_wallpaper(&mut self, raw_path: &str) -> bool {
        let trimmed = raw_path.trim();
        if trimmed.is_empty() {
            if let Some(ref mut input_state) = self.wallpaper_input {
                input_state.error_msg = Some("Path cannot be empty".to_string());
            }
            return false;
        }

        let expanded = crate::config::paths::AppPaths::expand_tilde(trimmed);
        if !expanded.exists() {
            if let Some(ref mut input_state) = self.wallpaper_input {
                input_state.error_msg = Some(format!("File not found: {}", expanded.display()));
            }
            return false;
        }

        match GeneratedPalette::from_image_path(&expanded) {
            Ok(palette) => {
                let path_str = expanded.display().to_string();
                self.palette = Some(palette);
                self.wallpaper_path = Some(path_str.clone());
                self.config.manual_wallpaper_path = Some(path_str.clone());
                self.config.wallpaper_backend = WallpaperBackend::Manual;

                // If currently in wizard, also update wizard config
                if let Some(ref mut wizard) = self.wizard_state {
                    wizard.config.manual_wallpaper_path = Some(path_str);
                    wizard.config.wallpaper_backend = WallpaperBackend::Manual;
                }

                // Persist update to config.toml
                let _ = self.config.save();

                self.status_message = Some("Wallpaper applied and palette generated successfully!".into());
                self.wallpaper_input = None;
                true
            }
            Err(e) => {
                if let Some(ref mut input_state) = self.wallpaper_input {
                    input_state.error_msg = Some(format!("Failed to load image: {}", e));
                }
                false
            }
        }
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

    fn ensure_hook_index_valid(&mut self) {
        let active = self.active_hook_indices();
        if active.is_empty() {
            self.selected_hook_idx = 0;
        } else if self.selected_hook_idx >= active.len() {
            self.selected_hook_idx = active.len().saturating_sub(1);
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
    fn toggle_wizard_option(&mut self) {
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
                    0 => wizard.config.tui_theme = TuiTheme::Dark,
                    1 => wizard.config.tui_theme = TuiTheme::Light,
                    2 => wizard.config.tui_theme = TuiTheme::System,
                    3 => wizard.config.font_mode = FontMode::Pixel,
                    4 => wizard.config.font_mode = FontMode::Standard,
                    _ => {}
                }
                self.config.tui_theme = wizard.config.tui_theme;
                self.config.font_mode = wizard.config.font_mode;
            }
            WizardStep::Environment => {
                match wizard.selected_idx {
                    0 => wizard.config.display_server = DisplayServer::Wayland,
                    1 => wizard.config.display_server = DisplayServer::X11,
                    2 => wizard.config.wallpaper_backend = WallpaperBackend::Auto,
                    3 => wizard.config.wallpaper_backend = WallpaperBackend::Swww,
                    4 => wizard.config.wallpaper_backend = WallpaperBackend::Hyprpaper,
                    5 => wizard.config.wallpaper_backend = WallpaperBackend::Awww,
                    6 => wizard.config.wallpaper_backend = WallpaperBackend::Wpaperd,
                    7 => wizard.config.wallpaper_backend = WallpaperBackend::Swaybg,
                    8 => wizard.config.wallpaper_backend = WallpaperBackend::Gnome,
                    9 => wizard.config.wallpaper_backend = WallpaperBackend::Kde,
                    10 => wizard.config.wallpaper_backend = WallpaperBackend::Feh,
                    11 => wizard.config.wallpaper_backend = WallpaperBackend::Nitrogen,
                    12 => wizard.config.wallpaper_backend = WallpaperBackend::Manual,
                    _ => {}
                }
            }
            WizardStep::Programs => {
                let rules = crate::template::ProgramRule::load_all().unwrap_or_default();
                if let Some(rule) = rules.get(wizard.selected_idx) {
                    let name_lower = rule.name.to_lowercase();
                    if let Some(pos) = wizard.config.enabled_programs.iter().position(|p| p.to_lowercase() == name_lower) {
                        wizard.config.enabled_programs.remove(pos);
                    } else {
                        wizard.config.enabled_programs.push(name_lower);
                    }
                }
            }
            WizardStep::Finish => {}
        }
    }

    /// Handles keyboard interaction when the First-Run Wizard is active.
    pub fn handle_wizard_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('q') => self.quit(),
            KeyCode::Esc => {
                if let Some(ref w) = self.wizard_state
                    && w.opened_from_main
                {
                    // Cancel and return cleanly to main TUI without modifying saved config
                    self.is_wizard = false;
                    self.wizard_state = None;
                } else {
                    self.quit();
                }
            }
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => self.quit(),

            // Wizard step navigation
            KeyCode::Tab | KeyCode::Right | KeyCode::Char('l') => {
                if let Some(ref mut wizard) = self.wizard_state {
                    wizard.step = wizard.step.next();
                    wizard.selected_idx = 0;
                }
            }
            KeyCode::BackTab | KeyCode::Left | KeyCode::Char('h') => {
                if let Some(ref mut wizard) = self.wizard_state {
                    wizard.step = wizard.step.prev();
                    wizard.selected_idx = 0;
                }
            }
            KeyCode::Char('1') => {
                if let Some(ref mut wizard) = self.wizard_state { wizard.step = WizardStep::Language; wizard.selected_idx = 0; }
            }
            KeyCode::Char('2') => {
                if let Some(ref mut wizard) = self.wizard_state { wizard.step = WizardStep::Appearance; wizard.selected_idx = 0; }
            }
            KeyCode::Char('3') => {
                if let Some(ref mut wizard) = self.wizard_state { wizard.step = WizardStep::Environment; wizard.selected_idx = 0; }
            }
            KeyCode::Char('4') => {
                if let Some(ref mut wizard) = self.wizard_state { wizard.step = WizardStep::Programs; wizard.selected_idx = 0; }
            }
            KeyCode::Char('5') => {
                if let Some(ref mut wizard) = self.wizard_state { wizard.step = WizardStep::Finish; wizard.selected_idx = 0; }
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
                self.toggle_wizard_option();
            }

            KeyCode::Char('w') => {
                self.open_wallpaper_input();
            }

            KeyCode::Enter => {
                let current_step = self.wizard_state.as_ref().map(|w| w.step);
                let selected_idx = self.wizard_state.as_ref().map(|w| w.selected_idx).unwrap_or(0);

                if current_step == Some(WizardStep::Finish) {
                    if let Err(e) = self.finish_wizard() {
                        self.status_message = Some(format!("Error: {}", e));
                    }
                } else if current_step == Some(WizardStep::Environment) && selected_idx == 12 {
                    // Option 12 in Environment is Manual path -> trigger input dialog
                    self.toggle_wizard_option();
                    self.open_wallpaper_input();
                } else {
                    self.toggle_wizard_option();
                    if let Some(ref mut wizard) = self.wizard_state {
                        wizard.step = wizard.step.next();
                        wizard.selected_idx = 0;
                    }
                }
            }
            _ => {}
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
                if input_state.cursor_pos > 0 && !input_state.input.is_empty() {
                    input_state.input.remove(input_state.cursor_pos - 1);
                    input_state.cursor_pos -= 1;
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
                if input_state.cursor_pos > 0 {
                    input_state.cursor_pos -= 1;
                }
            }
            KeyCode::Right => {
                if input_state.cursor_pos < input_state.input.len() {
                    input_state.cursor_pos += 1;
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
                input_state.cursor_pos += 1;
                input_state.error_msg = None;
            }
            _ => {}
        }
    }

    /// Renders templates for all enabled programs, safely writes them, and executes reload hooks.
    pub fn apply_theme(&mut self) -> Result<()> {
        let Some(ref palette) = self.palette else {
            return Err(crate::error::TermonetError::Config(
                "No palette generated to apply".into(),
            ));
        };

        let engine = TemplateEngine::new();

        // 1. Render and apply each enabled program
        for prog in &self.programs {
            if prog.enabled {
                if let Some(ref rule) = prog.rule {
                    rule.apply(&engine, palette)?;
                }
            }
        }

        // 2. Execute active reload hooks safely with detached stdio
        for prog in &self.programs {
            if prog.enabled && prog.hook_enabled {
                if let Some(ref cmd) = prog.reload_cmd {
                    let _ = std::process::Command::new("sh")
                        .arg("-c")
                        .arg(cmd)
                        .stdin(std::process::Stdio::null())
                        .stdout(std::process::Stdio::null())
                        .stderr(std::process::Stdio::null())
                        .status();
                }
            }
        }

        // 3. Update state cache
        if let Some(ref wall_path) = self.wallpaper_path {
            let mut cache = crate::cli::AppStateCache::load().unwrap_or_default();
            cache.last_wallpaper_path = Some(wall_path.clone());
            let _ = cache.save();
        }

        self.status_message = Some("Theme applied and hooks triggered successfully!".to_string());
        Ok(())
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
            KeyCode::Enter if self.current_step == Step::Palette => {
                self.open_wallpaper_input();
            }
            KeyCode::Tab => self.next_step(),
            KeyCode::BackTab => self.prev_step(),
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
                    let active_len = self.active_hook_indices().len();
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
                if self.current_step == Step::Apply {
                    if let Err(e) = self.apply_theme() {
                        self.status_message = Some(format!("Error: {}", e));
                    }
                }
            }
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => self.quit(),
            _ => {}
        }
    }
}
