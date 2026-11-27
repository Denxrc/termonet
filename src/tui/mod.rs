//! Terminal User Interface (TUI) components and event loop.

pub mod app;
pub mod font;
pub mod terminal;
pub mod views;

use crossterm::event::{self, Event};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
};
use std::time::Duration;

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

            // Responsive header height: 5 lines for pixel logo + tabs, or 3 lines for compact / standard font
            let header_height = if total_area.height >= 22
                && total_area.width >= 48
                && state.config.font_mode == crate::config::FontMode::Pixel
            {
                5
            } else {
                3
            };

            let root_layout = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(header_height), // Header with logo & tabs
                    Constraint::Min(8),                // Active step view
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
            render_footer(frame, root_layout[2], &state);
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

/// Renders the bottom footer bar with navigation hints based on the active step.
fn render_footer(frame: &mut Frame, area: Rect, state: &AppState) {
    let t = get_translations(state.config.language);
    let (primary_color, secondary_color) = state.ui_colors();
    let text_muted = state.text_muted();

    let mut footer_spans = vec![
        Span::styled(
            " [← / →] ",
            Style::default()
                .fg(primary_color)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("{}  ", t.hint_steps_nav),
            Style::default().fg(text_muted),
        ),
        Span::styled(
            " [1-4] ",
            Style::default()
                .fg(primary_color)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("{}  ", t.hint_jump),
            Style::default().fg(text_muted),
        ),
    ];

    match state.current_step {
        Step::Palette => {
            footer_spans.push(Span::styled(
                " [w] ",
                Style::default()
                    .fg(primary_color)
                    .add_modifier(Modifier::BOLD),
            ));
            footer_spans.push(Span::styled(
                format!("{}  ", t.hint_wallpaper),
                Style::default().fg(text_muted),
            ));
            footer_spans.push(Span::styled(
                " [r] ",
                Style::default()
                    .fg(primary_color)
                    .add_modifier(Modifier::BOLD),
            ));
            footer_spans.push(Span::styled(
                format!("{}  ", t.hint_auto_detect),
                Style::default().fg(text_muted),
            ));
            if state.active_monitors.len() > 1 {
                footer_spans.push(Span::styled(
                    " [Tab/m] ",
                    Style::default()
                        .fg(primary_color)
                        .add_modifier(Modifier::BOLD),
                ));
                footer_spans.push(Span::styled(
                    format!("{}  ", t.hint_switch_monitor),
                    Style::default().fg(text_muted),
                ));
            }
        }
        Step::Programs | Step::Hooks => {
            footer_spans.push(Span::styled(
                " [↑/↓] ",
                Style::default()
                    .fg(primary_color)
                    .add_modifier(Modifier::BOLD),
            ));
            footer_spans.push(Span::styled(
                format!("{}  ", t.hint_select),
                Style::default().fg(text_muted),
            ));
            footer_spans.push(Span::styled(
                " [Space] ",
                Style::default()
                    .fg(primary_color)
                    .add_modifier(Modifier::BOLD),
            ));
            footer_spans.push(Span::styled(
                format!("{}  ", t.hint_space),
                Style::default().fg(text_muted),
            ));
        }
        Step::Apply => {
            footer_spans.push(Span::styled(
                " [Enter] ",
                Style::default()
                    .fg(primary_color)
                    .add_modifier(Modifier::BOLD),
            ));
            footer_spans.push(Span::styled(
                format!("{}  ", t.hint_choose),
                Style::default().fg(text_muted),
            ));
        }
    }

    footer_spans.push(Span::styled(
        " [s] ",
        Style::default()
            .fg(primary_color)
            .add_modifier(Modifier::BOLD),
    ));
    footer_spans.push(Span::styled(
        format!("{}  ", t.hint_settings),
        Style::default().fg(text_muted),
    ));
    footer_spans.push(Span::styled(
        " [q/Esc] ",
        Style::default()
            .fg(secondary_color)
            .add_modifier(Modifier::BOLD),
    ));
    footer_spans.push(Span::styled(t.hint_quit, Style::default().fg(text_muted)));

    frame.render_widget(
        Paragraph::new(Line::from(footer_spans)).alignment(Alignment::Center),
        area,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{AppConfig, DisplayServer, WallpaperBackend};
    use crate::core::{GeneratedPalette, RgbColor};
    use crate::tui::app::{AppState, EnvSubStep, WizardStep};
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;
    use ratatui::style::Color;

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
        assert_eq!(
            state.wizard_state.as_ref().unwrap().step,
            WizardStep::Language
        );

        // Right arrow to Step 2 (Appearance)
        state.handle_key(crossterm::event::KeyEvent::from(
            crossterm::event::KeyCode::Right,
        ));
        assert_eq!(
            state.wizard_state.as_ref().unwrap().step,
            WizardStep::Appearance
        );

        // Press '3' to jump to Step 3 (Environment)
        state.handle_key(crossterm::event::KeyEvent::from(
            crossterm::event::KeyCode::Char('3'),
        ));
        assert_eq!(
            state.wizard_state.as_ref().unwrap().step,
            WizardStep::Environment
        );

        // Left arrow back to Step 2
        state.handle_key(crossterm::event::KeyEvent::from(
            crossterm::event::KeyCode::Left,
        ));
        assert_eq!(
            state.wizard_state.as_ref().unwrap().step,
            WizardStep::Appearance
        );
    }

    #[test]
    fn test_open_settings_from_main_and_cancel_esc() {
        let config = AppConfig::default();
        let mut state = AppState::new(config);

        assert!(!state.is_wizard);

        // Press 's' to open settings wizard
        state.handle_key(crossterm::event::KeyEvent::from(
            crossterm::event::KeyCode::Char('s'),
        ));
        assert!(state.is_wizard);
        assert!(state.wizard_state.as_ref().unwrap().opened_from_main);

        // Press 'Esc' to cancel and return to main interface
        state.handle_key(crossterm::event::KeyEvent::from(
            crossterm::event::KeyCode::Esc,
        ));
        assert!(!state.is_wizard);
        assert!(state.is_running);
    }

    #[test]
    fn test_wizard_render_both_languages() {
        let backend = TestBackend::new(100, 30);
        let mut terminal = Terminal::new(backend).expect("failed to create test terminal");

        for lang in [crate::config::Language::Ru, crate::config::Language::En] {
            let config = AppConfig {
                language: lang,
                ..Default::default()
            };
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
        state.handle_key(crossterm::event::KeyEvent::from(
            crossterm::event::KeyCode::Char('w'),
        ));
        assert!(state.wallpaper_input.is_some());

        // Clear pre-filled input and type new path
        if let Some(ref mut inp) = state.wallpaper_input {
            inp.input.clear();
            inp.cursor_pos = 0;
        }

        state.handle_key(crossterm::event::KeyEvent::from(
            crossterm::event::KeyCode::Char('a'),
        ));
        state.handle_key(crossterm::event::KeyEvent::from(
            crossterm::event::KeyCode::Char('b'),
        ));
        state.handle_key(crossterm::event::KeyEvent::from(
            crossterm::event::KeyCode::Char('c'),
        ));

        assert_eq!(state.wallpaper_input.as_ref().unwrap().input, "abc");

        // Backspace
        state.handle_key(crossterm::event::KeyEvent::from(
            crossterm::event::KeyCode::Backspace,
        ));
        assert_eq!(state.wallpaper_input.as_ref().unwrap().input, "ab");

        // Esc cancels modal
        state.handle_key(crossterm::event::KeyEvent::from(
            crossterm::event::KeyCode::Esc,
        ));
        assert!(state.wallpaper_input.is_none());
    }

    #[test]
    fn test_palette_enter_does_not_open_input_modal() {
        let config = AppConfig::default();
        let mut state = AppState::new(config);
        state.current_step = Step::Palette;

        // Press Enter in Palette step -> should do nothing
        state.handle_key(crossterm::event::KeyEvent::from(
            crossterm::event::KeyCode::Enter,
        ));
        assert!(state.wallpaper_input.is_none());

        // Press 'w' in Palette step -> opens modal
        state.handle_key(crossterm::event::KeyEvent::from(
            crossterm::event::KeyCode::Char('w'),
        ));
        assert!(state.wallpaper_input.is_some());
    }

    #[test]
    fn test_wizard_finish_step_space_saves() {
        let temp_dir =
            std::env::temp_dir().join(format!("termonet_test_wizard_space_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&temp_dir);
        let cfg_path = temp_dir.join("config.toml");

        let config = AppConfig::default();
        let mut state = AppState::new_wizard(config);
        state.custom_config_path = Some(cfg_path.clone());

        // Jump to Step 5 (Finish)
        state.handle_key(crossterm::event::KeyEvent::from(
            crossterm::event::KeyCode::Char('5'),
        ));
        assert_eq!(
            state.wizard_state.as_ref().unwrap().step,
            WizardStep::Finish
        );

        // Space triggers finish_wizard
        state.handle_key(crossterm::event::KeyEvent::from(
            crossterm::event::KeyCode::Char(' '),
        ));
        assert!(!state.is_wizard);
        assert!(cfg_path.exists());

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_wizard_finish_step_backup_selection() {
        let temp_dir = std::env::temp_dir().join(format!(
            "termonet_test_wizard_backup_{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&temp_dir);
        let cfg_path = temp_dir.join("config.toml");

        let config = AppConfig::default();
        let mut state = AppState::new_wizard(config);
        state.custom_config_path = Some(cfg_path.clone());

        // Jump to Step 5 (Finish)
        state.handle_key(crossterm::event::KeyEvent::from(
            crossterm::event::KeyCode::Char('5'),
        ));
        assert_eq!(
            state.wizard_state.as_ref().unwrap().step,
            WizardStep::Finish
        );
        assert_eq!(state.wizard_state.as_ref().unwrap().selected_idx, 2); // Default focused on Save button
        assert!(!state.wizard_state.as_ref().unwrap().config.create_backups);

        // Move Up to Safe Mode (index 1)
        state.handle_key(crossterm::event::KeyEvent::from(
            crossterm::event::KeyCode::Up,
        ));
        assert_eq!(state.wizard_state.as_ref().unwrap().selected_idx, 1);

        // Press Space to toggle
        state.handle_key(crossterm::event::KeyEvent::from(
            crossterm::event::KeyCode::Char(' '),
        ));
        assert!(state.wizard_state.as_ref().unwrap().config.create_backups);

        // Move Up to Clean Mode (index 0)
        state.handle_key(crossterm::event::KeyEvent::from(
            crossterm::event::KeyCode::Up,
        ));
        assert_eq!(state.wizard_state.as_ref().unwrap().selected_idx, 0);

        // Press Enter on Clean Mode: selects Clean mode and auto-focuses Save button (index 2)
        state.handle_key(crossterm::event::KeyEvent::from(
            crossterm::event::KeyCode::Enter,
        ));
        assert!(!state.wizard_state.as_ref().unwrap().config.create_backups);
        assert_eq!(state.wizard_state.as_ref().unwrap().selected_idx, 2);

        // Press Enter on Save button: finishes wizard
        state.handle_key(crossterm::event::KeyEvent::from(
            crossterm::event::KeyCode::Enter,
        ));
        assert!(!state.is_wizard);
        assert!(!state.config.create_backups);
        assert!(cfg_path.exists());

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_wizard_multi_monitor_flow() {
        use crate::tui::app::EnvSubStep;
        use std::path::PathBuf;

        let config = AppConfig::default();
        let mut state = AppState::new_wizard(config);

        // Step 1: Jump to Step 3 (Environment)
        state.handle_key(crossterm::event::KeyEvent::from(
            crossterm::event::KeyCode::Char('3'),
        ));
        let wizard = state.wizard_state.as_ref().unwrap();
        assert_eq!(wizard.step, WizardStep::Environment);
        assert_eq!(wizard.env_substep, EnvSubStep::ServerAndBackend);

        // Inject discovered divergent monitors into wizard state
        if let Some(ref mut w) = state.wizard_state {
            w.discovered_monitors = Some(vec![
                ("DP-1".into(), PathBuf::from("/wallpaper_dp.png")),
                ("HDMI-A-1".into(), PathBuf::from("/wallpaper_hdmi.png")),
            ]);
            w.env_substep = EnvSubStep::Monitors;
            w.selected_idx = 0;
            w.config.primary_monitor = Some("DP-1".into());
        }

        // Test cursor move down
        state.handle_key(crossterm::event::KeyEvent::from(
            crossterm::event::KeyCode::Down,
        ));
        assert_eq!(state.wizard_state.as_ref().unwrap().selected_idx, 1);

        // Test selection with Space
        state.handle_key(crossterm::event::KeyEvent::from(
            crossterm::event::KeyCode::Char(' '),
        ));
        assert_eq!(
            state
                .wizard_state
                .as_ref()
                .unwrap()
                .config
                .primary_monitor
                .as_deref(),
            Some("HDMI-A-1")
        );

        // Advance Right -> should move to Programs
        state.handle_key(crossterm::event::KeyEvent::from(
            crossterm::event::KeyCode::Right,
        ));
        assert_eq!(
            state.wizard_state.as_ref().unwrap().step,
            WizardStep::Programs
        );

        // Move Left -> should return to Environment in EnvSubStep::Monitors
        state.handle_key(crossterm::event::KeyEvent::from(
            crossterm::event::KeyCode::Left,
        ));
        assert_eq!(
            state.wizard_state.as_ref().unwrap().step,
            WizardStep::Environment
        );
        assert_eq!(
            state.wizard_state.as_ref().unwrap().env_substep,
            EnvSubStep::Monitors
        );
        assert_eq!(state.wizard_state.as_ref().unwrap().selected_idx, 1);

        // Move Left again -> should return to ServerAndBackend
        state.handle_key(crossterm::event::KeyEvent::from(
            crossterm::event::KeyCode::Left,
        ));
        assert_eq!(
            state.wizard_state.as_ref().unwrap().step,
            WizardStep::Environment
        );
        assert_eq!(
            state.wizard_state.as_ref().unwrap().env_substep,
            EnvSubStep::ServerAndBackend
        );

        // Move Left again -> should return to Appearance
        state.handle_key(crossterm::event::KeyEvent::from(
            crossterm::event::KeyCode::Left,
        ));
        assert_eq!(
            state.wizard_state.as_ref().unwrap().step,
            WizardStep::Appearance
        );
    }

    #[test]
    fn test_wizard_dynamic_backend_options() {
        let config = AppConfig::default(); // default display server is Wayland
        let mut wizard = WizardState::new(config);
        wizard.step = WizardStep::Environment;
        wizard.env_substep = EnvSubStep::ServerAndBackend;

        // Wayland: 2 servers + 9 backends = 11 options
        assert_eq!(wizard.max_options(), 11);

        // Switch to X11
        wizard.config.display_server = DisplayServer::X11;
        // X11: 2 servers + 6 backends = 8 options
        assert_eq!(wizard.max_options(), 8);
    }

    #[test]
    fn test_wizard_server_switch_clamps_index_and_resets_backend() {
        let mut state = AppState::new_wizard(AppConfig::default());
        if let Some(ref mut wizard) = state.wizard_state {
            wizard.step = WizardStep::Environment;
            wizard.env_substep = EnvSubStep::ServerAndBackend;
            wizard.config.display_server = DisplayServer::Wayland;
            wizard.config.wallpaper_backend = WallpaperBackend::Swww; // Wayland-only
            wizard.selected_idx = 10; // near bottom of Wayland list
        }

        // Switch to X11 (selected_idx 1)
        if let Some(ref mut wizard) = state.wizard_state {
            wizard.selected_idx = 1;
        }
        state.toggle_wizard_option();

        let wizard = state.wizard_state.as_ref().unwrap();
        assert_eq!(wizard.config.display_server, DisplayServer::X11);
        assert_eq!(wizard.config.wallpaper_backend, WallpaperBackend::Auto);
        assert!(wizard.selected_idx < wizard.max_options());
    }

    #[test]
    fn test_tui_manual_hex_input_generates_palette() {
        let mut state = AppState::new(AppConfig::default());
        assert!(state.custom_seed_color.is_none());

        // User types "#e63946" in manual input modal
        let success = state.apply_manual_wallpaper("#e63946");
        assert!(success);
        assert_eq!(
            state.custom_seed_color,
            Some(RgbColor::new(0xe6, 0x39, 0x46))
        );
        assert_eq!(state.wallpaper_path, None);
        assert!(state.palette.is_some());
        assert_eq!(
            state.palette.as_ref().unwrap().seed_color,
            RgbColor::new(0xe6, 0x39, 0x46)
        );
    }

    #[test]
    fn test_render_wizard_monitors_substep() {
        let backend = TestBackend::new(100, 30);
        let mut terminal = Terminal::new(backend).expect("failed to create test terminal");

        let config = AppConfig::default();
        let mut state = AppState::new_wizard(config);
        if let Some(ref mut w) = state.wizard_state {
            w.step = WizardStep::Environment;
            w.env_substep = crate::tui::app::EnvSubStep::Monitors;
            w.discovered_monitors = Some(vec![
                ("DP-1".into(), std::path::PathBuf::from("/path/to/dp.png")),
                (
                    "HDMI-A-1".into(),
                    std::path::PathBuf::from("/path/to/hdmi.png"),
                ),
            ]);
            w.config.primary_monitor = Some("DP-1".into());
        }

        terminal
            .draw(|f| {
                views::render_wizard(f, f.area(), &state);
            })
            .expect("draw wizard monitors substep succeeded");
    }

    #[test]
    fn test_palette_tab_cycles_monitors() {
        let temp_dir = std::env::temp_dir();
        let file_dp = temp_dir.join("termonet_test_tab_dp.png");
        let file_hdmi = temp_dir.join("termonet_test_tab_hdmi.png");

        let png_bytes = [
            0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48,
            0x44, 0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x02, 0x00, 0x00,
            0x00, 0x90, 0x77, 0x53, 0xde, 0x00, 0x00, 0x00, 0x0c, 0x49, 0x44, 0x41, 0x54, 0x08,
            0xd7, 0x63, 0xf8, 0xcf, 0xc0, 0x00, 0x00, 0x03, 0x01, 0x01, 0x00, 0x18, 0xdd, 0x8d,
            0xb0, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4e, 0x44, 0xae, 0x42, 0x60, 0x82,
        ];
        std::fs::write(&file_dp, png_bytes).unwrap();
        std::fs::write(&file_hdmi, png_bytes).unwrap();

        let config = AppConfig::default();
        let mut state = AppState::new(config);
        state.current_step = Step::Palette;
        state.active_monitors = vec![
            ("DP-1".into(), file_dp.clone()),
            ("HDMI-A-1".into(), file_hdmi.clone()),
        ];
        state.current_monitor_idx = 0;
        state.wallpaper_path = Some(file_dp.display().to_string());

        // Press Tab to cycle to HDMI-A-1
        state.handle_key(crossterm::event::KeyEvent::from(
            crossterm::event::KeyCode::Tab,
        ));
        assert_eq!(state.current_monitor_idx, 1);
        assert_eq!(state.config.primary_monitor.as_deref(), Some("HDMI-A-1"));
        assert_eq!(
            state.wallpaper_path.as_deref(),
            Some(file_hdmi.to_str().unwrap())
        );

        // Press 'm' to cycle back to DP-1
        state.handle_key(crossterm::event::KeyEvent::from(
            crossterm::event::KeyCode::Char('m'),
        ));
        assert_eq!(state.current_monitor_idx, 0);
        assert_eq!(state.config.primary_monitor.as_deref(), Some("DP-1"));
        assert_eq!(
            state.wallpaper_path.as_deref(),
            Some(file_dp.to_str().unwrap())
        );

        let _ = std::fs::remove_file(file_dp);
        let _ = std::fs::remove_file(file_hdmi);
    }

    #[test]
    fn test_render_palette_with_multi_monitors() {
        let backend = TestBackend::new(140, 30);
        let mut terminal = Terminal::new(backend).expect("failed to create test terminal");

        let config = AppConfig::default();
        let mut state = AppState::new(config);
        let seed = material_colors::color::Rgb::new(0x42, 0x85, 0xf4);
        state.palette = Some(crate::core::GeneratedPalette::from_seed_color(seed));
        state.active_monitors = vec![
            ("DP-1".into(), std::path::PathBuf::from("/path/to/dp.png")),
            (
                "HDMI-A-1".into(),
                std::path::PathBuf::from("/path/to/hdmi.png"),
            ),
        ];
        state.current_monitor_idx = 0;

        terminal
            .draw(|f| {
                render_footer(f, f.area(), &state);
            })
            .expect("draw footer with multi-monitors succeeded");

        let buffer = terminal.backend().buffer();
        let footer_row: String = (0..140).map(|x| buffer[(x, 0)].symbol()).collect();
        assert!(
            footer_row.contains("[Tab/m]"),
            "Palette footer should contain [Tab/m]"
        );
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
        let temp_dir =
            std::env::temp_dir().join(format!("termonet_test_apply_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&temp_dir);
        std::fs::create_dir_all(&temp_dir).unwrap();

        let kitty_conf = temp_dir.join("kitty.conf");
        std::fs::write(&kitty_conf, "font_size 11.0\n").unwrap();

        let custom_rule = crate::template::ProgramRule {
            id: String::new(),
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

        let config = AppConfig {
            language: crate::config::Language::En,
            ..Default::default()
        };
        let mut state = AppState::new(config);
        let seed = material_colors::color::Rgb::new(0x42, 0x85, 0xf4);
        state.palette = Some(crate::core::GeneratedPalette::from_seed_color(seed));

        state.programs = vec![ProgramItem {
            name: "TestKitty".to_string(),
            output_path: temp_dir.join("colors-termonet.conf").display().to_string(),
            enabled: true,
            reload_cmd: None,
            reload_desc: None,
            hook_enabled: false,
            rule: Some(custom_rule),
        }];

        state.current_step = Step::Apply;
        state.handle_key(crossterm::event::KeyEvent::from(
            crossterm::event::KeyCode::Enter,
        ));

        assert!(
            state
                .status_message
                .as_ref()
                .unwrap()
                .contains("successfully")
        );
        assert!(!state.status_is_error);
        assert!(temp_dir.join("colors-termonet.conf").exists());

        // Verify Status line is actually visible inside render_apply (not clipped by border)
        let backend = TestBackend::new(100, 24);
        let mut terminal = Terminal::new(backend).expect("failed to create test terminal");
        terminal
            .draw(|f| {
                views::render_apply(f, f.area(), &state);
            })
            .unwrap();
        let buffer = terminal.backend().buffer();
        let mut rendered_text = String::new();
        for y in 0..6 {
            for x in 0..100 {
                rendered_text.push_str(buffer[(x, y)].symbol());
            }
            rendered_text.push('\n');
        }
        assert!(
            rendered_text.contains("Theme applied"),
            "Status line must be visible inside Apply action box, got:\n{rendered_text}"
        );

        let target_content = std::fs::read_to_string(&kitty_conf).unwrap();
        assert!(target_content.contains("# >>> termonet colors >>>"));
        assert!(target_content.contains("include colors-termonet.conf"));
        assert!(target_content.contains("# <<< termonet colors <<<"));

        // Clean up
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_footer_hints_per_step() {
        let backend = TestBackend::new(120, 30);
        let mut terminal = Terminal::new(backend).expect("failed to create test terminal");

        let config = AppConfig::default();
        let mut state = AppState::new(config);

        for step in [Step::Palette, Step::Programs, Step::Hooks, Step::Apply] {
            state.current_step = step;
            terminal
                .draw(|f| {
                    render_footer(f, f.area(), &state);
                })
                .expect("draw footer succeeded");

            let buffer = terminal.backend().buffer();
            let footer_row: String = (0..120).map(|x| buffer[(x, 0)].symbol()).collect();

            match step {
                Step::Palette => {
                    assert!(
                        footer_row.contains("[w]"),
                        "Palette footer should contain [w]"
                    );
                    assert!(
                        footer_row.contains("[r]"),
                        "Palette footer should contain [r]"
                    );
                }
                Step::Programs | Step::Hooks => {
                    assert!(
                        footer_row.contains("[↑/↓]"),
                        "Programs/Hooks footer should contain [↑/↓]"
                    );
                    assert!(
                        footer_row.contains("[Space]"),
                        "Programs/Hooks footer should contain [Space]"
                    );
                }
                Step::Apply => {
                    assert!(
                        footer_row.contains("[Enter]"),
                        "Apply footer should contain [Enter]"
                    );
                }
            }
        }
    }

    #[test]
    fn test_ui_colors_respect_tui_theme() {
        let seed = material_colors::color::Rgb::new(0x21, 0x96, 0xf3);
        let palette = GeneratedPalette::from_seed_color(seed);

        let dark_config = AppConfig {
            tui_theme: crate::config::TuiTheme::Dark,
            ..Default::default()
        };
        let mut dark_state = AppState::new(dark_config);
        dark_state.palette = Some(palette.clone());

        assert!(dark_state.is_dark_mode());
        let (dark_prim, dark_sec) = dark_state.ui_colors();
        assert_eq!(dark_prim, Color::from(palette.dark.primary));
        assert_eq!(dark_sec, Color::from(palette.dark.secondary));

        let light_config = AppConfig {
            tui_theme: crate::config::TuiTheme::Light,
            ..Default::default()
        };
        let mut light_state = AppState::new(light_config);
        light_state.palette = Some(palette.clone());

        assert!(!light_state.is_dark_mode());
        let (light_prim, light_sec) = light_state.ui_colors();
        assert_eq!(light_prim, Color::from(palette.light.primary));
        assert_eq!(light_sec, Color::from(palette.light.secondary));
        assert_ne!(dark_prim, light_prim);
    }

    #[test]
    fn test_stress_wizard_state_index_overflow_protection() {
        use crate::tui::app::{EnvSubStep, WizardState, WizardStep};

        let mut wizard = WizardState::new(AppConfig::default());
        wizard.step = WizardStep::Environment;
        wizard.env_substep = EnvSubStep::Monitors;
        wizard.discovered_monitors = None;
        assert_eq!(wizard.max_options(), 0);

        wizard.discovered_monitors = Some(Vec::new());
        assert_eq!(wizard.max_options(), 0);

        wizard.selected_idx = usize::MAX;
        let mut state = AppState::new_wizard(AppConfig::default());
        state.wizard_state = Some(wizard);

        state.handle_key(crossterm::event::KeyEvent::from(
            crossterm::event::KeyCode::Char(' '),
        ));
        state.handle_key(crossterm::event::KeyEvent::from(
            crossterm::event::KeyCode::Enter,
        ));
        assert!(state.wizard_state.is_some());
    }

    #[test]
    fn test_stress_app_state_cycling_with_empty_or_single_monitors() {
        let mut state = AppState::new(AppConfig::default());
        state.current_step = Step::Palette;

        // 1. Zero monitors: cycle must be a safe no-op
        state.active_monitors.clear();
        state.current_monitor_idx = 999;
        state.cycle_monitor_wallpaper();
        assert_eq!(state.current_monitor_idx, 999);

        // 2. Single monitor: cycle must also be a safe no-op
        state.active_monitors = vec![("DP-1".into(), std::path::PathBuf::from("/fake.png"))];
        state.current_monitor_idx = 0;
        state.cycle_monitor_wallpaper();
        assert_eq!(state.current_monitor_idx, 0);

        // 3. Overflow index wrap-around: current_monitor_idx starting beyond active_monitors.len()
        state.active_monitors = vec![
            ("DP-1".into(), std::path::PathBuf::from("/a.png")),
            ("HDMI-1".into(), std::path::PathBuf::from("/b.png")),
        ];
        state.current_monitor_idx = 100;
        state.cycle_monitor_wallpaper(); // (100 + 1) % 2 == 1
        assert_eq!(state.current_monitor_idx, 1);
    }

    #[test]
    fn test_stress_wallpaper_input_modal_typing_and_backspacing() {
        let mut state = AppState::new(AppConfig::default());
        state.open_wallpaper_input();
        assert!(state.wallpaper_input.is_some());

        // Type 10,000 characters
        for i in 0..10_000 {
            let ch = (b'a' + (i % 26) as u8) as char;
            state.handle_key(crossterm::event::KeyEvent::from(
                crossterm::event::KeyCode::Char(ch),
            ));
        }
        assert_eq!(state.wallpaper_input.as_ref().unwrap().input.len(), 10_000);

        // Navigate Left 5,000 times
        for _ in 0..5_000 {
            state.handle_key(crossterm::event::KeyEvent::from(
                crossterm::event::KeyCode::Left,
            ));
        }
        assert_eq!(state.wallpaper_input.as_ref().unwrap().cursor_pos, 5_000);

        // Hit Home
        state.handle_key(crossterm::event::KeyEvent::from(
            crossterm::event::KeyCode::Home,
        ));
        assert_eq!(state.wallpaper_input.as_ref().unwrap().cursor_pos, 0);

        // Delete at index 0 1,000 times
        for _ in 0..1_000 {
            state.handle_key(crossterm::event::KeyEvent::from(
                crossterm::event::KeyCode::Delete,
            ));
        }
        assert_eq!(state.wallpaper_input.as_ref().unwrap().input.len(), 9_000);

        // Hit End
        state.handle_key(crossterm::event::KeyEvent::from(
            crossterm::event::KeyCode::End,
        ));
        assert_eq!(state.wallpaper_input.as_ref().unwrap().cursor_pos, 9_000);

        // Backspace 15,000 times (more than length) -> cursor must stay at 0, no panic
        for _ in 0..15_000 {
            state.handle_key(crossterm::event::KeyEvent::from(
                crossterm::event::KeyCode::Backspace,
            ));
        }
        assert_eq!(state.wallpaper_input.as_ref().unwrap().cursor_pos, 0);
        assert!(state.wallpaper_input.as_ref().unwrap().input.is_empty());
    }

    #[test]
    fn test_cyrillic_and_emoji_modal_input_utf8_boundaries() {
        let mut state = AppState::new(AppConfig::default());
        state.open_wallpaper_input();

        // Type Russian word "обои"
        for c in "обои".chars() {
            state.handle_key(crossterm::event::KeyEvent::from(
                crossterm::event::KeyCode::Char(c),
            ));
        }
        let input_state = state.wallpaper_input.as_ref().unwrap();
        assert_eq!(input_state.input, "обои");
        assert_eq!(input_state.cursor_pos, "обои".len());

        // Navigate Left twice
        state.handle_key(crossterm::event::KeyEvent::from(
            crossterm::event::KeyCode::Left,
        ));
        state.handle_key(crossterm::event::KeyEvent::from(
            crossterm::event::KeyCode::Left,
        ));
        let cur = state.wallpaper_input.as_ref().unwrap().cursor_pos;
        assert!(
            state
                .wallpaper_input
                .as_ref()
                .unwrap()
                .input
                .is_char_boundary(cur)
        );

        // Backspace removes one char safely
        state.handle_key(crossterm::event::KeyEvent::from(
            crossterm::event::KeyCode::Backspace,
        ));
        let input_state2 = state.wallpaper_input.as_ref().unwrap();
        assert_eq!(input_state2.input, "оои");
        assert!(input_state2.input.is_char_boundary(input_state2.cursor_pos));
    }

    #[test]
    fn test_stress_fuzzing_tui_key_router() {
        let mut state = AppState::new(AppConfig::default());

        let strange_keys = [
            crossterm::event::KeyCode::F(1),
            crossterm::event::KeyCode::F(12),
            crossterm::event::KeyCode::Null,
            crossterm::event::KeyCode::PageUp,
            crossterm::event::KeyCode::PageDown,
            crossterm::event::KeyCode::Insert,
            crossterm::event::KeyCode::BackTab,
            crossterm::event::KeyCode::Char('\0'),
            crossterm::event::KeyCode::Char('я'),
            crossterm::event::KeyCode::Char('🚀'),
        ];

        for i in 0..5_000 {
            let key = strange_keys[i % strange_keys.len()];
            state.handle_key(crossterm::event::KeyEvent::from(key));
        }
        assert!(state.is_running);
    }

    #[test]
    fn test_settings_esc_rolls_back_changes_and_respects_empty_programs() {
        let initial_config = AppConfig {
            language: crate::config::Language::Ru,
            tui_theme: crate::config::TuiTheme::Dark,
            enabled_programs: Vec::new(),
            ..Default::default()
        };
        let mut state = AppState::new(initial_config);
        // Empty enabled_programs must keep all programs disabled in TUI
        assert!(state.programs.iter().all(|p| !p.enabled));

        // Open settings wizard from main TUI
        state.handle_key(crossterm::event::KeyEvent::from(
            crossterm::event::KeyCode::Char('s'),
        ));
        assert!(state.is_wizard);

        // Select English (index 1) on Language step
        state.handle_key(crossterm::event::KeyEvent::from(
            crossterm::event::KeyCode::Down,
        ));
        state.handle_key(crossterm::event::KeyEvent::from(
            crossterm::event::KeyCode::Char(' '),
        ));
        assert_eq!(state.config.language, crate::config::Language::En);

        // Move to Appearance and select Light theme (index 1)
        state.handle_key(crossterm::event::KeyEvent::from(
            crossterm::event::KeyCode::Right,
        ));
        state.handle_key(crossterm::event::KeyEvent::from(
            crossterm::event::KeyCode::Down,
        ));
        state.handle_key(crossterm::event::KeyEvent::from(
            crossterm::event::KeyCode::Char(' '),
        ));
        assert_eq!(state.config.tui_theme, crate::config::TuiTheme::Light);
        assert!(!state.is_dark_mode());

        // Press Esc -> must cancel and roll back language & theme
        state.handle_key(crossterm::event::KeyEvent::from(
            crossterm::event::KeyCode::Esc,
        ));
        assert!(!state.is_wizard);
        assert_eq!(state.config.language, crate::config::Language::Ru);
        assert_eq!(state.config.tui_theme, crate::config::TuiTheme::Dark);
        assert!(state.is_dark_mode());
    }

    #[test]
    fn test_render_palette_narrow_and_scrolling_modal_input() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).expect("failed to create test terminal");

        let config = AppConfig {
            language: crate::config::Language::Ru,
            ..Default::default()
        };
        let mut state = AppState::new(config);
        let seed = material_colors::color::Rgb::new(0x42, 0x85, 0xf4);
        state.palette = Some(GeneratedPalette::from_seed_color(seed));
        state.wallpaper_path = Some("/home/user/very/long/path/to/wallpapers/nature.png".into());
        state.open_wallpaper_input();
        if let Some(ref mut inp) = state.wallpaper_input {
            inp.input =
                "/home/user/extremely/long/nested/directory/structure/wallpaper_final_v2.png"
                    .into();
            inp.cursor_pos = inp.input.len();
        }

        terminal
            .draw(|f| {
                views::render_palette(f, f.area(), &state);
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let full_text: String = (0..24)
            .flat_map(|y| (0..80).map(move |x| buffer[(x, y)].symbol()))
            .collect();
        assert!(
            full_text.contains("█"),
            "Horizontally scrolled modal must keep cursor visible"
        );
        assert!(
            full_text.contains("wallpaper_final_v2.png"),
            "Tail of long path around cursor must be visible"
        );
    }
}
