//! TUI state machine and main interaction loop.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use crate::config::AppConfig;
use crate::core::GeneratedPalette;

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

/// Represents an application target in the TUI list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProgramItem {
    pub name: String,
    pub output_path: String,
    pub enabled: bool,
}

/// Represents a hook command in the TUI list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HookItem {
    pub program: String,
    pub command: String,
    pub description: String,
    pub enabled: bool,
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

    // Step 3: Hooks interactive state
    pub hooks: Vec<HookItem>,
    pub selected_hook_idx: usize,
}

impl AppState {
    /// Creates a new TUI application state with given configuration.
    #[must_use]
    pub fn new(config: AppConfig) -> Self {
        let programs = vec![
            ProgramItem {
                name: "Kitty".into(),
                output_path: "~/.config/kitty/colors-termonet.conf".into(),
                enabled: true,
            },
            ProgramItem {
                name: "Waybar".into(),
                output_path: "~/.config/waybar/colors.css".into(),
                enabled: true,
            },
            ProgramItem {
                name: "Rofi".into(),
                output_path: "~/.config/rofi/colors.rasi".into(),
                enabled: true,
            },
            ProgramItem {
                name: "Hyprland".into(),
                output_path: "~/.config/hypr/colors.conf".into(),
                enabled: true,
            },
            ProgramItem {
                name: "Alacritty".into(),
                output_path: "~/.config/alacritty/colors.toml".into(),
                enabled: false,
            },
            ProgramItem {
                name: "Dunst".into(),
                output_path: "~/.config/dunst/dunstrc.d/colors.conf".into(),
                enabled: false,
            },
            ProgramItem {
                name: "Sway".into(),
                output_path: "~/.config/sway/colors.conf".into(),
                enabled: false,
            },
        ];

        let hooks = vec![
            HookItem {
                program: "Waybar".into(),
                command: "killall -SIGUSR2 waybar".into(),
                description: "Instant bar style reload".into(),
                enabled: true,
            },
            HookItem {
                program: "Kitty".into(),
                command: "killall -SIGUSR1 kitty".into(),
                description: "Live terminal color update".into(),
                enabled: true,
            },
            HookItem {
                program: "Hyprland".into(),
                command: "hyprctl reload".into(),
                description: "Window manager borders & theme reload".into(),
                enabled: true,
            },
            HookItem {
                program: "Dunst".into(),
                command: "killall -SIGUSR1 dunst".into(),
                description: "Notification daemon reload".into(),
                enabled: false,
            },
        ];

        Self {
            current_step: Step::Palette,
            is_running: true,
            config,
            palette: None,
            wallpaper_path: None,
            status_message: None,
            programs,
            selected_program_idx: 0,
            hooks,
            selected_hook_idx: 0,
        }
    }

    /// Advances to the next step.
    pub fn next_step(&mut self) {
        self.current_step = self.current_step.next();
    }

    /// Returns to the previous step.
    pub fn prev_step(&mut self) {
        self.current_step = self.current_step.previous();
    }

    /// Sets the current active step directly.
    pub fn set_step(&mut self, step: Step) {
        self.current_step = step;
    }

    /// Quits the TUI event loop.
    pub fn quit(&mut self) {
        self.is_running = false;
    }

    /// Toggles the currently selected program item in Step 2.
    pub fn toggle_current_program(&mut self) {
        if let Some(item) = self.programs.get_mut(self.selected_program_idx) {
            item.enabled = !item.enabled;
        }
    }

    /// Toggles the currently selected hook item in Step 3.
    pub fn toggle_current_hook(&mut self) {
        if let Some(item) = self.hooks.get_mut(self.selected_hook_idx) {
            item.enabled = !item.enabled;
        }
    }

    /// Handles keyboard events.
    pub fn handle_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => self.quit(),
            KeyCode::Tab => self.next_step(),
            KeyCode::BackTab => self.prev_step(),
            KeyCode::Char('1') => self.set_step(Step::Palette),
            KeyCode::Char('2') => self.set_step(Step::Programs),
            KeyCode::Char('3') => self.set_step(Step::Hooks),
            KeyCode::Char('4') => self.set_step(Step::Apply),

            // Step-specific navigation
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
                Step::Hooks if self.selected_hook_idx + 1 < self.hooks.len() => {
                    self.selected_hook_idx += 1;
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
                    self.status_message = Some("Theme applied successfully!".to_string());
                }
            }
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => self.quit(),
            _ => {}
        }
    }
}
