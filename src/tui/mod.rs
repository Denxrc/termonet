//! Terminal User Interface (TUI) components and event loop.

pub mod app;
pub mod font;
pub mod terminal;
pub mod views;

use std::time::Duration;
use crossterm::event::{self, Event};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
};

pub use app::{AppState, Step};
pub use font::PixelFont;
pub use terminal::{TerminalGuard, TuiTerminal};

use crate::error::Result;

/// Main TUI event loop driver.
pub fn run_tui(mut state: AppState) -> Result<()> {
    let mut guard = TerminalGuard::new()?;

    while state.is_running {
        guard.draw(|frame| {
            let total_area = frame.area();

            // Responsive header height calculation
            let header_height = if total_area.height >= 24 && total_area.width >= 48 {
                7
            } else {
                4
            };

            let root_layout = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(header_height), // Header with logo & tabs
                    Constraint::Min(8),                 // Active step view
                    Constraint::Length(1),             // Bottom footer with key hints
                ])
                .split(total_area);

            // 1. Render Header
            views::render_header(frame, root_layout[0], &state);

            // 2. Render Active View
            match state.current_step {
                Step::Palette => views::render_palette(frame, root_layout[1], &state),
                Step::Programs => views::render_programs(frame, root_layout[1], &state),
                Step::Hooks => views::render_hooks(frame, root_layout[1], &state),
                Step::Apply => views::render_apply(frame, root_layout[1], &state),
            }

            // 3. Render Footer Bar
            let primary_color = state
                .palette
                .as_ref()
                .map(|p| Color::from(p.dark.primary))
                .unwrap_or(Color::Rgb(173, 198, 255));

            let secondary_color = state
                .palette
                .as_ref()
                .map(|p| Color::from(p.dark.secondary))
                .unwrap_or(Color::Rgb(191, 198, 220));

            let footer_line = Line::from(vec![
                Span::styled(" [Tab] ", Style::default().fg(primary_color).add_modifier(Modifier::BOLD)),
                Span::styled("Next Step  ", Style::default().fg(Color::Gray)),
                Span::styled(" [Shift+Tab] ", Style::default().fg(primary_color).add_modifier(Modifier::BOLD)),
                Span::styled("Prev Step  ", Style::default().fg(Color::Gray)),
                Span::styled(" [1-4] ", Style::default().fg(primary_color).add_modifier(Modifier::BOLD)),
                Span::styled("Jump Step  ", Style::default().fg(Color::Gray)),
                Span::styled(" [d/l] ", Style::default().fg(primary_color).add_modifier(Modifier::BOLD)),
                Span::styled("Toggle Dark/Light  ", Style::default().fg(Color::Gray)),
                Span::styled(" [q/Esc] ", Style::default().fg(secondary_color).add_modifier(Modifier::BOLD)),
                Span::styled("Quit", Style::default().fg(Color::Gray)),
            ]);

            frame.render_widget(Paragraph::new(footer_line).alignment(Alignment::Center), root_layout[2]);
        })?;

        // 50ms polling (~20 FPS) for responsive interaction without high CPU usage
        if event::poll(Duration::from_millis(50))?
            && let Event::Key(key) = event::read()?
        {
            state.handle_key(key);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;
    use crate::config::AppConfig;
    use crate::core::GeneratedPalette;

    #[test]
    fn test_tui_render_all_steps() {
        let backend = TestBackend::new(100, 30);
        let mut terminal = Terminal::new(backend).expect("failed to create test terminal");

        let config = AppConfig::default();
        let mut state = AppState::new(config);
        let seed = material_colors::color::Rgb::new(0x42, 0x85, 0xf4);
        state.palette = Some(GeneratedPalette::from_seed_color(seed));

        for step in [Step::Palette, Step::Programs, Step::Hooks, Step::Apply] {
            state.current_step = step;
            terminal
                .draw(|f| {
                    let area = f.area();
                    let chunks = Layout::default()
                        .direction(Direction::Vertical)
                        .constraints([
                            Constraint::Length(7),
                            Constraint::Min(8),
                            Constraint::Length(1),
                        ])
                        .split(area);

                    views::render_header(f, chunks[0], &state);
                    match state.current_step {
                        Step::Palette => views::render_palette(f, chunks[1], &state),
                        Step::Programs => views::render_programs(f, chunks[1], &state),
                        Step::Hooks => views::render_hooks(f, chunks[1], &state),
                        Step::Apply => views::render_apply(f, chunks[1], &state),
                    }
                })
                .expect("draw failed");
        }
    }
}
