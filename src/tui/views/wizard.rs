//! First-run interactive setup wizard.

use ratatui::layout::Rect;
use ratatui::Frame;
use crate::tui::app::AppState;

/// Renders the first-run configuration wizard.
pub fn render_wizard(_frame: &mut Frame, _area: Rect, _state: &AppState) {
    // Stub: renders wizard steps (language, font, display server, etc.)
}
