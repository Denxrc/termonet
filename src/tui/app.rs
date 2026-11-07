//! TUI state machine and main interaction loop.

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
}

/// TUI application state.
pub struct AppState {
    pub current_step: Step,
    pub is_running: bool,
    pub config: AppConfig,
    pub palette: Option<GeneratedPalette>,
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
        }
    }

    /// Quits the TUI event loop.
    pub fn quit(&mut self) {
        self.is_running = false;
    }
}
