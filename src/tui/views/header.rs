//! Responsive top bar rendering with pixel/standard title and step navigation tabs.

use ratatui::layout::Rect;
use ratatui::Frame;
use crate::tui::app::AppState;

/// Renders the top header bar including logo and step indicators.
pub fn render_header(_frame: &mut Frame, _area: Rect, _state: &AppState) {
    // Stub: renders pixel/standard logo and tabs for steps 1..4
}
