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

pub use app::{AppState, ProgramItem, Step, WizardState, WizardStep};
pub use font::PixelFont;
pub use terminal::{TerminalGuard, TuiTerminal};

use crate::error::Result;
use crate::i18n::get_translations;

/// Main TUI event loop driver.
pub fn run_tui(mut state: AppState) -> Result<()> {
    let mut guard = TerminalGuard::new()?;

    while state.is_running {
        guard.draw(|frame| {
            let total_area = frame.area();

            // If First-Run Wizard is active, delegate full rendering to wizard view
            if state.is_wizard {
                views::render_wizard(frame, total_area, &state);
                return;
            }

            // Responsive header height: 5 lines for pixel logo + tabs, or 3 lines for compact
            let header_height = if total_area.height >= 22 && total_area.width >= 48 {
                5
            } else {
                3
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

            // 3. Render Footer Bar with localized hints
            let t = get_translations(state.config.language);
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

            let mut footer_spans = vec![
                Span::styled(" [← / →] ", Style::default().fg(primary_color).add_modifier(Modifier::BOLD)),
                Span::styled(format!("{}  ", t.hint_steps_nav), Style::default().fg(Color::Gray)),
                Span::styled(" [1-4] ", Style::default().fg(primary_color).add_modifier(Modifier::BOLD)),
                Span::styled(format!("{}  ", t.hint_jump), Style::default().fg(Color::Gray)),
            ];

            if state.current_step == Step::Palette {
                footer_spans.push(Span::styled(" [w] ", Style::default().fg(primary_color).add_modifier(Modifier::BOLD)));
                footer_spans.push(Span::styled(format!("{}  ", t.hint_wallpaper), Style::default().fg(Color::Gray)));
                footer_spans.push(Span::styled(" [r] ", Style::default().fg(primary_color).add_modifier(Modifier::BOLD)));
                footer_spans.push(Span::styled(format!("{}  ", t.hint_auto_detect), Style::default().fg(Color::Gray)));
            } else {
                footer_spans.push(Span::styled(" [Space] ", Style::default().fg(primary_color).add_modifier(Modifier::BOLD)));
                footer_spans.push(Span::styled(format!("{}  ", t.hint_space), Style::default().fg(Color::Gray)));
            }

            footer_spans.push(Span::styled(" [s] ", Style::default().fg(primary_color).add_modifier(Modifier::BOLD)));
            footer_spans.push(Span::styled(format!("{}  ", t.hint_settings), Style::default().fg(Color::Gray)));
            footer_spans.push(Span::styled(" [q/Esc] ", Style::default().fg(secondary_color).add_modifier(Modifier::BOLD)));
            footer_spans.push(Span::styled(t.hint_quit, Style::default().fg(Color::Gray)));

            frame.render_widget(Paragraph::new(Line::from(footer_spans)).alignment(Alignment::Center), root_layout[2]);
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
                            Constraint::Length(5),
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

    #[test]
    fn test_wizard_render_all_steps() {
        let backend = TestBackend::new(100, 30);
        let mut terminal = Terminal::new(backend).expect("failed to create test terminal");

        let config = AppConfig::default();
        let mut state = AppState::new_wizard(config);

        let wizard_steps = [
            WizardStep::Language,
            WizardStep::Appearance,
            WizardStep::Environment,
            WizardStep::Programs,
            WizardStep::Finish,
        ];

        for step in wizard_steps {
            if let Some(ref mut w) = state.wizard_state {
                w.step = step;
            }
            terminal
                .draw(|f| {
                    views::render_wizard(f, f.area(), &state);
                })
                .expect("draw wizard step succeeded");
        }
    }

    #[test]
    fn test_wizard_key_navigation_and_toggle() {
        let config = AppConfig::default();
        let mut state = AppState::new_wizard(config);

        assert!(state.is_wizard);
        assert_eq!(state.wizard_state.as_ref().unwrap().step, WizardStep::Language);

        // Right arrow to Step 2 (Appearance)
        state.handle_key(crossterm::event::KeyEvent::from(crossterm::event::KeyCode::Right));
        assert_eq!(state.wizard_state.as_ref().unwrap().step, WizardStep::Appearance);

        // Press '3' to jump to Step 3 (Environment)
        state.handle_key(crossterm::event::KeyEvent::from(crossterm::event::KeyCode::Char('3')));
        assert_eq!(state.wizard_state.as_ref().unwrap().step, WizardStep::Environment);

        // Left arrow back to Step 2
        state.handle_key(crossterm::event::KeyEvent::from(crossterm::event::KeyCode::Left));
        assert_eq!(state.wizard_state.as_ref().unwrap().step, WizardStep::Appearance);
    }

    #[test]
    fn test_open_settings_from_main_and_cancel_esc() {
        let config = AppConfig::default();
        let mut state = AppState::new(config);

        assert!(!state.is_wizard);

        // Press 's' to open settings wizard
        state.handle_key(crossterm::event::KeyEvent::from(crossterm::event::KeyCode::Char('s')));
        assert!(state.is_wizard);
        assert!(state.wizard_state.as_ref().unwrap().opened_from_main);

        // Press 'Esc' to cancel and return to main interface
        state.handle_key(crossterm::event::KeyEvent::from(crossterm::event::KeyCode::Esc));
        assert!(!state.is_wizard);
        assert!(state.is_running);
    }

    #[test]
    fn test_wizard_render_both_languages() {
        let backend = TestBackend::new(100, 30);
        let mut terminal = Terminal::new(backend).expect("failed to create test terminal");

        for lang in [crate::config::Language::Ru, crate::config::Language::En] {
            let mut config = AppConfig::default();
            config.language = lang;
            let state = AppState::new_wizard(config);

            terminal
                .draw(|f| {
                    views::render_wizard(f, f.area(), &state);
                })
                .expect("draw wizard step in language succeeded");
        }
    }

    #[test]
    fn test_wallpaper_input_flow() {
        let config = AppConfig::default();
        let mut state = AppState::new(config);
        state.current_step = Step::Palette;

        assert!(state.wallpaper_input.is_none());

        // Press 'w' to open wallpaper input
        state.handle_key(crossterm::event::KeyEvent::from(crossterm::event::KeyCode::Char('w')));
        assert!(state.wallpaper_input.is_some());

        // Clear pre-filled input and type new path
        if let Some(ref mut inp) = state.wallpaper_input {
            inp.input.clear();
            inp.cursor_pos = 0;
        }

        state.handle_key(crossterm::event::KeyEvent::from(crossterm::event::KeyCode::Char('a')));
        state.handle_key(crossterm::event::KeyEvent::from(crossterm::event::KeyCode::Char('b')));
        state.handle_key(crossterm::event::KeyEvent::from(crossterm::event::KeyCode::Char('c')));

        assert_eq!(state.wallpaper_input.as_ref().unwrap().input, "abc");

        // Backspace
        state.handle_key(crossterm::event::KeyEvent::from(crossterm::event::KeyCode::Backspace));
        assert_eq!(state.wallpaper_input.as_ref().unwrap().input, "ab");

        // Esc cancels modal
        state.handle_key(crossterm::event::KeyEvent::from(crossterm::event::KeyCode::Esc));
        assert!(state.wallpaper_input.is_none());
    }

    #[test]
    fn test_palette_enter_does_not_open_input_modal() {
        let config = AppConfig::default();
        let mut state = AppState::new(config);
        state.current_step = Step::Palette;

        // Press Enter in Palette step -> should do nothing
        state.handle_key(crossterm::event::KeyEvent::from(crossterm::event::KeyCode::Enter));
        assert!(state.wallpaper_input.is_none());

        // Press 'w' in Palette step -> opens modal
        state.handle_key(crossterm::event::KeyEvent::from(crossterm::event::KeyCode::Char('w')));
        assert!(state.wallpaper_input.is_some());
    }

    #[test]
    fn test_wizard_finish_step_space_saves() {
        let config = AppConfig::default();
        let mut state = AppState::new_wizard(config);

        // Jump to Step 5 (Finish)
        state.handle_key(crossterm::event::KeyEvent::from(crossterm::event::KeyCode::Char('5')));
        assert_eq!(state.wizard_state.as_ref().unwrap().step, WizardStep::Finish);

        // Space triggers finish_wizard
        state.handle_key(crossterm::event::KeyEvent::from(crossterm::event::KeyCode::Char(' ')));
        assert!(!state.is_wizard);
    }

    #[test]
    fn test_render_palette_with_input_modal() {
        let backend = TestBackend::new(100, 30);
        let mut terminal = Terminal::new(backend).expect("failed to create test terminal");

        let config = AppConfig::default();
        let mut state = AppState::new(config);
        let seed = material_colors::color::Rgb::new(0x42, 0x85, 0xf4);
        state.palette = Some(crate::core::GeneratedPalette::from_seed_color(seed));
        state.open_wallpaper_input();

        terminal
            .draw(|f| {
                views::render_palette(f, f.area(), &state);
            })
            .expect("draw palette with wallpaper modal succeeded");
    }

    #[test]
    fn test_apply_theme_execution() {
        let temp_dir = std::env::temp_dir().join(format!("termonet_test_apply_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&temp_dir);
        std::fs::create_dir_all(&temp_dir).unwrap();

        let kitty_conf = temp_dir.join("kitty.conf");
        std::fs::write(&kitty_conf, "font_size 11.0\n").unwrap();

        let custom_rule = crate::template::ProgramRule {
            version: 2,
            name: "TestKitty".to_string(),
            template: "kitty.tmpl".to_string(),
            output_path: temp_dir.join("colors-termonet.conf"),
            target_config: Some(kitty_conf.clone()),
            import_line: Some("include colors-termonet.conf".to_string()),
            import_position: crate::template::ImportPosition::Bottom,
            reload_cmd: None,
            comment_style: Some(crate::template::CommentStyle::Hash),
        };

        let config = AppConfig::default();
        let mut state = AppState::new(config);
        let seed = material_colors::color::Rgb::new(0x42, 0x85, 0xf4);
        state.palette = Some(crate::core::GeneratedPalette::from_seed_color(seed));

        state.programs = vec![
            ProgramItem {
                name: "TestKitty".to_string(),
                output_path: temp_dir.join("colors-termonet.conf").display().to_string(),
                enabled: true,
                reload_cmd: None,
                reload_desc: None,
                hook_enabled: false,
                rule: Some(custom_rule),
            }
        ];

        state.current_step = Step::Apply;
        state.handle_key(crossterm::event::KeyEvent::from(crossterm::event::KeyCode::Enter));

        assert!(state.status_message.as_ref().unwrap().contains("successfully"));
        assert!(temp_dir.join("colors-termonet.conf").exists());

        let target_content = std::fs::read_to_string(&kitty_conf).unwrap();
        assert!(target_content.contains("# >>> termonet colors >>>"));
        assert!(target_content.contains("include colors-termonet.conf"));
        assert!(target_content.contains("# <<< termonet colors <<<"));

        // Clean up
        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
