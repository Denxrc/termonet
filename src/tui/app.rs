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

    /// Returns human-readable localized label index and title.
    #[must_use]
    pub const fn index(self) -> usize {
        self as usize
    }
}

/// TUI application state.
pub struct AppState {
    pub current_step: Step,
    pub is_running: bool,
    pub config: AppConfig,
    pub palette: Option<GeneratedPalette>,
    pub wallpaper_path: Option<String>,
    pub preview_dark: bool,
    pub status_message: Option<String>,
}

impl AppState {
    /// Creates a new TUI application state with given configuration.
    #[must_use]
    pub fn new(config: AppConfig) -> Self {
        Self {
            current_step: Step::Palette,
            is_running: true,
            config,
            palette: None,
            wallpaper_path: None,
            preview_dark: true,
            status_message: None,
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
            KeyCode::Char('d') | KeyCode::Char('l') => {
                self.preview_dark = !self.preview_dark;
            }
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => self.quit(),
            _ => {}
        }
    }
}
