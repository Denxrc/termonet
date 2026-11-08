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

/// Represents an application target in the TUI list, including its reload hook.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProgramItem {
    pub name: String,
    pub output_path: String,
    pub enabled: bool,
    pub reload_cmd: Option<String>,
    pub reload_desc: Option<String>,
    pub hook_enabled: bool,
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
                reload_cmd: Some("killall -SIGUSR1 kitty".into()),
                reload_desc: Some("Live terminal color reload".into()),
                hook_enabled: true,
            },
            ProgramItem {
                name: "Waybar".into(),
                output_path: "~/.config/waybar/colors.css".into(),
                enabled: true,
                reload_cmd: Some("killall -SIGUSR2 waybar".into()),
                reload_desc: Some("Instant CSS style reload".into()),
                hook_enabled: true,
            },
            ProgramItem {
                name: "Rofi".into(),
                output_path: "~/.config/rofi/colors.rasi".into(),
                enabled: true,
                reload_cmd: None, // Rofi loads colors on each run, no reload needed
                reload_desc: None,
                hook_enabled: false,
            },
            ProgramItem {
                name: "Hyprland".into(),
                output_path: "~/.config/hypr/colors.conf".into(),
                enabled: true,
                reload_cmd: Some("hyprctl reload".into()),
                reload_desc: Some("Window borders & active theme reload".into()),
                hook_enabled: true,
            },
            ProgramItem {
                name: "Alacritty".into(),
                output_path: "~/.config/alacritty/colors.toml".into(),
                enabled: false,
                reload_cmd: None, // Alacritty auto-reloads on file change
                reload_desc: None,
                hook_enabled: false,
            },
            ProgramItem {
                name: "Dunst".into(),
                output_path: "~/.config/dunst/dunstrc.d/colors.conf".into(),
                enabled: false,
                reload_cmd: Some("killall -SIGUSR1 dunst".into()),
                reload_desc: Some("Notification daemon reload".into()),
                hook_enabled: true,
            },
            ProgramItem {
                name: "Sway".into(),
                output_path: "~/.config/sway/colors.conf".into(),
                enabled: false,
                reload_cmd: Some("swaymsg reload".into()),
                reload_desc: Some("Sway compositor config reload".into()),
                hook_enabled: true,
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
            selected_hook_idx: 0,
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
                    self.status_message = Some("Theme applied successfully!".to_string());
                }
            }
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => self.quit(),
            _ => {}
        }
    }
}
