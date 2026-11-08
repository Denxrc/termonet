//! First-run interactive setup wizard view.

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::config::{DisplayServer, FontMode, Language, TuiTheme, WallpaperBackend};
use crate::config::paths::AppPaths;
use crate::i18n::get_translations;
use crate::tui::app::{AppState, WizardStep};
use crate::tui::font::PixelFont;

/// Renders the complete First-Run Wizard interface.
pub fn render_wizard(frame: &mut Frame, area: Rect, state: &AppState) {
    let t = get_translations(state.config.language);

    let (primary_color, secondary_color) = if let Some(ref pal) = state.palette {
        let scheme = match state.config.tui_theme {
            TuiTheme::Light => &pal.light,
            _ => &pal.dark,
        };
        (Color::from(scheme.primary), Color::from(scheme.secondary))
    } else {
        (Color::Rgb(173, 198, 255), Color::Rgb(191, 198, 220))
    };

    let use_pixel = area.height >= 24 && area.width >= 50 && state.config.font_mode == FontMode::Pixel;
    let header_height = if use_pixel { 6 } else { 3 };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(header_height), // Header with logo & wizard steps
            Constraint::Min(12),               // Active wizard step content
            Constraint::Length(1),             // Bottom footer with key hints
        ])
        .split(area);

    // 1. Header (Logo + Wizard Step Navigation)
    render_wizard_header(frame, chunks[0], state, primary_color, secondary_color, use_pixel);

    // 2. Main Step Card
    render_wizard_body(frame, chunks[1], state, primary_color, secondary_color);

    // 3. Footer Bar
    let is_opened_from_main = state
        .wizard_state
        .as_ref()
        .map(|w| w.opened_from_main)
        .unwrap_or(false);

    let mut footer_spans = vec![
        Span::styled(" [Tab/Enter] ", Style::default().fg(primary_color).add_modifier(Modifier::BOLD)),
        Span::styled(format!("{}  ", t.hint_next), Style::default().fg(Color::Gray)),
        Span::styled(" [Shift+Tab] ", Style::default().fg(primary_color).add_modifier(Modifier::BOLD)),
        Span::styled(format!("{}  ", t.hint_prev), Style::default().fg(Color::Gray)),
        Span::styled(" [1-5] ", Style::default().fg(primary_color).add_modifier(Modifier::BOLD)),
        Span::styled(format!("{}  ", t.hint_jump), Style::default().fg(Color::Gray)),
        Span::styled(" [↑/↓] ", Style::default().fg(primary_color).add_modifier(Modifier::BOLD)),
        Span::styled(format!("{}  ", t.hint_select), Style::default().fg(Color::Gray)),
        Span::styled(" [Space] ", Style::default().fg(primary_color).add_modifier(Modifier::BOLD)),
        Span::styled(format!("{}  ", t.hint_space), Style::default().fg(Color::Gray)),
    ];

    if is_opened_from_main {
        footer_spans.push(Span::styled(" [Esc] ", Style::default().fg(secondary_color).add_modifier(Modifier::BOLD)));
        footer_spans.push(Span::styled(format!("{}  ", t.hint_cancel), Style::default().fg(Color::Gray)));
        footer_spans.push(Span::styled(" [q] ", Style::default().fg(secondary_color).add_modifier(Modifier::BOLD)));
        footer_spans.push(Span::styled(t.hint_quit, Style::default().fg(Color::Gray)));
    } else {
        footer_spans.push(Span::styled(" [q/Esc] ", Style::default().fg(secondary_color).add_modifier(Modifier::BOLD)));
        footer_spans.push(Span::styled(t.hint_quit, Style::default().fg(Color::Gray)));
    }

    frame.render_widget(Paragraph::new(Line::from(footer_spans)).alignment(Alignment::Center), chunks[2]);

    // 4. Render wallpaper input modal over wizard if active
    if state.wallpaper_input.is_some() {
        crate::tui::views::palette::render_wallpaper_input_modal(frame, area, state);
    }
}

fn render_wizard_header(
    frame: &mut Frame,
    area: Rect,
    state: &AppState,
    primary_color: Color,
    secondary_color: Color,
    use_pixel: bool,
) {
    let t = get_translations(state.config.language);
    let active_step = state.wizard_state.as_ref().map(|w| w.step).unwrap_or_default();

    if use_pixel {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Pixel Logo
                Constraint::Length(1), // Subtitle
                Constraint::Length(2), // Step breadcrumbs
            ])
            .split(area);

        let pixel_lines = PixelFont::render_lines(
            "TERMONET",
            Style::default().fg(primary_color).add_modifier(Modifier::BOLD),
        );
        frame.render_widget(Paragraph::new(pixel_lines).alignment(Alignment::Center), chunks[0]);

        let subtitle = Line::from(vec![
            Span::styled(t.wizard_title, Style::default().fg(secondary_color).add_modifier(Modifier::BOLD)),
        ]);
        frame.render_widget(Paragraph::new(subtitle).alignment(Alignment::Center), chunks[1]);

        render_step_pills(frame, chunks[2], active_step, state, primary_color);
    } else {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // Title line
                Constraint::Length(2), // Step breadcrumbs
            ])
            .split(area);

        let title = Line::from(vec![
            Span::styled("TERMONET ", Style::default().fg(primary_color).add_modifier(Modifier::BOLD)),
            Span::styled("• ", Style::default().fg(secondary_color)),
            Span::styled(t.wizard_title, Style::default().fg(Color::White)),
        ]);
        frame.render_widget(Paragraph::new(title).alignment(Alignment::Center), chunks[0]);

        render_step_pills(frame, chunks[1], active_step, state, primary_color);
    }
}

fn render_step_pills(
    frame: &mut Frame,
    area: Rect,
    active_step: WizardStep,
    state: &AppState,
    primary_color: Color,
) {
    let t = get_translations(state.config.language);
    let steps = [
        (WizardStep::Language, t.wizard_step_lang),
        (WizardStep::Appearance, t.wizard_step_appearance),
        (WizardStep::Environment, t.wizard_step_env),
        (WizardStep::Programs, t.wizard_step_programs),
        (WizardStep::Finish, t.wizard_step_finish),
    ];

    let mut spans = Vec::new();
    for (i, (step, label)) in steps.into_iter().enumerate() {
        if i > 0 {
            spans.push(Span::styled("  ➔  ", Style::default().fg(Color::DarkGray)));
        }

        if step == active_step {
            spans.push(Span::styled(
                label,
                Style::default()
                    .fg(primary_color)
                    .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
            ));
        } else {
            spans.push(Span::styled(label, Style::default().fg(Color::Gray)));
        }
    }

    frame.render_widget(Paragraph::new(Line::from(spans)).alignment(Alignment::Center), area);
}

fn render_wizard_body(
    frame: &mut Frame,
    area: Rect,
    state: &AppState,
    primary_color: Color,
    secondary_color: Color,
) {
    let Some(wizard) = &state.wizard_state else {
        return;
    };
    let t = get_translations(state.config.language);

    match wizard.step {
        WizardStep::Language => render_step_language(frame, area, wizard, t, primary_color, secondary_color),
        WizardStep::Appearance => render_step_appearance(frame, area, wizard, t, primary_color, secondary_color),
        WizardStep::Environment => render_step_environment(frame, area, wizard, t, primary_color, secondary_color),
        WizardStep::Programs => render_step_programs(frame, area, wizard, t, primary_color, secondary_color),
        WizardStep::Finish => render_step_finish(frame, area, wizard, t, primary_color, secondary_color),
    }
}

fn render_step_language(
    frame: &mut Frame,
    area: Rect,
    wizard: &crate::tui::app::WizardState,
    t: &crate::i18n::Translations,
    primary_color: Color,
    secondary_color: Color,
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(primary_color))
        .title(t.wizard_lang_title);

    let options = [
        (t.wizard_lang_ru, Language::Ru),
        (t.wizard_lang_en, Language::En),
    ];

    let mut lines = vec![
        Line::raw(""),
        Line::from(vec![
            Span::styled(format!("   {} ", t.wizard_lang_prompt), Style::default().fg(secondary_color).add_modifier(Modifier::BOLD)),
        ]),
        Line::raw(""),
    ];

    for (idx, (label, lang)) in options.into_iter().enumerate() {
        let is_current_cursor = wizard.selected_idx == idx;
        let is_selected = wizard.config.language == lang;

        let cursor_span = if is_current_cursor {
            Span::styled("   ➔  ", Style::default().fg(primary_color).add_modifier(Modifier::BOLD))
        } else {
            Span::styled("      ", Style::default())
        };

        let radio_span = if is_selected {
            Span::styled("[●] ", Style::default().fg(primary_color).add_modifier(Modifier::BOLD))
        } else {
            Span::styled("[ ] ", Style::default().fg(Color::DarkGray))
        };

        let label_style = if is_current_cursor {
            Style::default().fg(Color::White).add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
        } else if is_selected {
            Style::default().fg(Color::White).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Gray)
        };

        lines.push(Line::from(vec![cursor_span, radio_span, Span::styled(label, label_style)]));
        lines.push(Line::raw(""));
    }

    lines.push(Line::from(vec![
        Span::styled(format!("   {}", t.wizard_lang_hint), Style::default().fg(Color::DarkGray)),
    ]));

    frame.render_widget(Paragraph::new(lines).block(block), area);
}

fn render_step_appearance(
    frame: &mut Frame,
    area: Rect,
    wizard: &crate::tui::app::WizardState,
    t: &crate::i18n::Translations,
    primary_color: Color,
    secondary_color: Color,
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(primary_color))
        .title(t.wizard_appearance_title);

    let mut lines = vec![
        Line::raw(""),
        Line::from(vec![
            Span::styled(format!("   ── {} ──────────────────────────────────", t.wizard_theme_sec), Style::default().fg(secondary_color).add_modifier(Modifier::BOLD)),
        ]),
        Line::raw(""),
    ];

    let theme_options = [
        (t.wizard_theme_dark, TuiTheme::Dark),
        (t.wizard_theme_light, TuiTheme::Light),
        (t.wizard_theme_system, TuiTheme::System),
    ];

    for (idx, (label, theme)) in theme_options.into_iter().enumerate() {
        let is_current_cursor = wizard.selected_idx == idx;
        let is_selected = wizard.config.tui_theme == theme;

        let cursor_span = if is_current_cursor {
            Span::styled("   ➔  ", Style::default().fg(primary_color).add_modifier(Modifier::BOLD))
        } else {
            Span::styled("      ", Style::default())
        };

        let radio_span = if is_selected {
            Span::styled("[●] ", Style::default().fg(primary_color).add_modifier(Modifier::BOLD))
        } else {
            Span::styled("[ ] ", Style::default().fg(Color::DarkGray))
        };

        let label_style = if is_current_cursor {
            Style::default().fg(Color::White).add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
        } else if is_selected {
            Style::default().fg(Color::White).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Gray)
        };

        lines.push(Line::from(vec![cursor_span, radio_span, Span::styled(label, label_style)]));
    }

    lines.push(Line::raw(""));
    lines.push(Line::from(vec![
        Span::styled(format!("   ── {} ────────────────────────────────────────", t.wizard_font_sec), Style::default().fg(secondary_color).add_modifier(Modifier::BOLD)),
    ]));
    lines.push(Line::raw(""));

    let font_options = [
        (t.wizard_font_pixel, FontMode::Pixel),
        (t.wizard_font_standard, FontMode::Standard),
    ];

    for (offset, (label, mode)) in font_options.into_iter().enumerate() {
        let idx = 3 + offset;
        let is_current_cursor = wizard.selected_idx == idx;
        let is_selected = wizard.config.font_mode == mode;

        let cursor_span = if is_current_cursor {
            Span::styled("   ➔  ", Style::default().fg(primary_color).add_modifier(Modifier::BOLD))
        } else {
            Span::styled("      ", Style::default())
        };

        let radio_span = if is_selected {
            Span::styled("[●] ", Style::default().fg(primary_color).add_modifier(Modifier::BOLD))
        } else {
            Span::styled("[ ] ", Style::default().fg(Color::DarkGray))
        };

        let label_style = if is_current_cursor {
            Style::default().fg(Color::White).add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
        } else if is_selected {
            Style::default().fg(Color::White).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Gray)
        };

        lines.push(Line::from(vec![cursor_span, radio_span, Span::styled(label, label_style)]));
    }

    frame.render_widget(Paragraph::new(lines).block(block), area);
}

fn render_step_environment(
    frame: &mut Frame,
    area: Rect,
    wizard: &crate::tui::app::WizardState,
    t: &crate::i18n::Translations,
    primary_color: Color,
    secondary_color: Color,
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(primary_color))
        .title(t.wizard_env_title);

    let mut lines = vec![
        Line::from(vec![
            Span::styled(format!("   ── {} ─────────────────────────────────", t.wizard_server_sec), Style::default().fg(secondary_color).add_modifier(Modifier::BOLD)),
        ]),
    ];

    let server_options = [
        (t.wizard_server_wayland, DisplayServer::Wayland),
        (t.wizard_server_x11, DisplayServer::X11),
    ];

    for (idx, (label, server)) in server_options.into_iter().enumerate() {
        let is_current_cursor = wizard.selected_idx == idx;
        let is_selected = wizard.config.display_server == server;

        let cursor_span = if is_current_cursor {
            Span::styled("   ➔  ", Style::default().fg(primary_color).add_modifier(Modifier::BOLD))
        } else {
            Span::styled("      ", Style::default())
        };

        let radio_span = if is_selected {
            Span::styled("[●] ", Style::default().fg(primary_color).add_modifier(Modifier::BOLD))
        } else {
            Span::styled("[ ] ", Style::default().fg(Color::DarkGray))
        };

        let label_style = if is_current_cursor {
            Style::default().fg(Color::White).add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
        } else if is_selected {
            Style::default().fg(Color::White).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Gray)
        };

        lines.push(Line::from(vec![cursor_span, radio_span, Span::styled(label, label_style)]));
    }

    lines.push(Line::from(vec![
        Span::styled(format!("   ── {} ────────────────────", t.wizard_backend_sec), Style::default().fg(secondary_color).add_modifier(Modifier::BOLD)),
    ]));

    let backend_options = [
        (t.wizard_backend_auto, WallpaperBackend::Auto),
        (t.wizard_backend_swww, WallpaperBackend::Swww),
        (t.wizard_backend_hyprpaper, WallpaperBackend::Hyprpaper),
        (t.wizard_backend_awww, WallpaperBackend::Awww),
        (t.wizard_backend_wpaperd, WallpaperBackend::Wpaperd),
        (t.wizard_backend_swaybg, WallpaperBackend::Swaybg),
        (t.wizard_backend_gnome, WallpaperBackend::Gnome),
        (t.wizard_backend_kde, WallpaperBackend::Kde),
        (t.wizard_backend_feh, WallpaperBackend::Feh),
        (t.wizard_backend_nitrogen, WallpaperBackend::Nitrogen),
        (t.wizard_backend_manual, WallpaperBackend::Manual),
    ];

    for (offset, (label, backend)) in backend_options.into_iter().enumerate() {
        let idx = 2 + offset;
        let is_current_cursor = wizard.selected_idx == idx;
        let is_selected = wizard.config.wallpaper_backend == backend;

        let cursor_span = if is_current_cursor {
            Span::styled("   ➔  ", Style::default().fg(primary_color).add_modifier(Modifier::BOLD))
        } else {
            Span::styled("      ", Style::default())
        };

        let radio_span = if is_selected {
            Span::styled("[●] ", Style::default().fg(primary_color).add_modifier(Modifier::BOLD))
        } else {
            Span::styled("[ ] ", Style::default().fg(Color::DarkGray))
        };

        let label_style = if is_current_cursor {
            Style::default().fg(Color::White).add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
        } else if is_selected {
            Style::default().fg(Color::White).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Gray)
        };

        let label_text = if backend == WallpaperBackend::Manual {
            if let Some(ref p) = wizard.config.manual_wallpaper_path {
                format!("{} ({})", label, p)
            } else {
                format!("{} — [w]", label)
            }
        } else {
            label.to_string()
        };

        lines.push(Line::from(vec![cursor_span, radio_span, Span::styled(label_text, label_style)]));
    }

    frame.render_widget(Paragraph::new(lines).block(block), area);
}

fn render_step_programs(
    frame: &mut Frame,
    area: Rect,
    wizard: &crate::tui::app::WizardState,
    t: &crate::i18n::Translations,
    primary_color: Color,
    secondary_color: Color,
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(primary_color))
        .title(t.wizard_programs_title);

    let rules = crate::template::ProgramRule::load_all().unwrap_or_default();

    let mut lines = vec![
        Line::raw(""),
        Line::from(vec![
            Span::styled(format!("   {} ", t.wizard_programs_desc), Style::default().fg(secondary_color).add_modifier(Modifier::BOLD)),
        ]),
        Line::raw(""),
    ];

    for (idx, rule) in rules.into_iter().enumerate() {
        let is_current_cursor = wizard.selected_idx == idx;
        let is_checked = wizard.config.enabled_programs.iter().any(|p| p.eq_ignore_ascii_case(&rule.name));

        let cursor_span = if is_current_cursor {
            Span::styled("   ➔  ", Style::default().fg(primary_color).add_modifier(Modifier::BOLD))
        } else {
            Span::styled("      ", Style::default())
        };

        let check_span = if is_checked {
            Span::styled("[✔] ", Style::default().fg(primary_color).add_modifier(Modifier::BOLD))
        } else {
            Span::styled("[ ] ", Style::default().fg(Color::DarkGray))
        };

        let name_style = if is_current_cursor {
            Style::default().fg(Color::White).add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
        } else if is_checked {
            Style::default().fg(Color::White).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Rgb(170, 170, 170))
        };

        let output_str = rule.output_path.to_string_lossy();

        lines.push(Line::from(vec![
            cursor_span,
            check_span,
            Span::styled(format!("{:<14}", rule.name), name_style),
            Span::styled(" ➔  ", Style::default().fg(secondary_color)),
            Span::styled(output_str.to_string(), Style::default().fg(Color::Rgb(200, 200, 200))),
        ]));
    }

    frame.render_widget(Paragraph::new(lines).block(block), area);
}

fn render_step_finish(
    frame: &mut Frame,
    area: Rect,
    wizard: &crate::tui::app::WizardState,
    t: &crate::i18n::Translations,
    primary_color: Color,
    secondary_color: Color,
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(primary_color))
        .title(t.wizard_finish_title);

    let config_path = AppPaths::config_file()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| "~/.config/termonet/config.toml".into());

    let lang_str = match wizard.config.language {
        Language::Ru => "Русский",
        Language::En => "English",
    };

    let theme_str = match wizard.config.tui_theme {
        TuiTheme::Dark => t.val_dark,
        TuiTheme::Light => t.val_light,
        TuiTheme::System => t.val_system,
    };

    let font_str = match wizard.config.font_mode {
        FontMode::Pixel => t.val_pixel_font,
        FontMode::Standard => t.val_standard_font,
    };

    let server_str = match wizard.config.display_server {
        DisplayServer::Wayland => "Wayland",
        DisplayServer::X11 => "X11",
    };

    let backend_str = match wizard.config.wallpaper_backend {
        WallpaperBackend::Auto => t.wizard_backend_auto,
        WallpaperBackend::Swww => "swww",
        WallpaperBackend::Hyprpaper => "hyprpaper",
        WallpaperBackend::Awww => "awww",
        WallpaperBackend::Wpaperd => "wpaperd",
        WallpaperBackend::Swaybg => "swaybg",
        WallpaperBackend::Gnome => "GNOME",
        WallpaperBackend::Kde => "KDE Plasma",
        WallpaperBackend::Feh => "feh",
        WallpaperBackend::Nitrogen => "nitrogen",
        WallpaperBackend::Manual => t.wizard_backend_manual,
    };

    let programs_str = if wizard.config.enabled_programs.is_empty() {
        t.wizard_summary_programs_empty.to_string()
    } else {
        format!("{} {} ({})", wizard.config.enabled_programs.len(), t.wizard_summary_programs_count, wizard.config.enabled_programs.join(", "))
    };

    let lines = vec![
        Line::raw(""),
        Line::from(vec![
            Span::styled(format!("   ── {} ──────────────────────────", t.wizard_summary_sec), Style::default().fg(secondary_color).add_modifier(Modifier::BOLD)),
        ]),
        Line::raw(""),
        Line::from(vec![
            Span::styled(format!("      • {:<26}", t.wizard_summary_lang), Style::default().fg(Color::Gray)),
            Span::styled(lang_str, Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled(format!("      • {:<26}", t.wizard_summary_theme), Style::default().fg(Color::Gray)),
            Span::styled(theme_str, Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled(format!("      • {:<26}", t.wizard_summary_font), Style::default().fg(Color::Gray)),
            Span::styled(font_str, Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled(format!("      • {:<26}", t.wizard_summary_server), Style::default().fg(Color::Gray)),
            Span::styled(server_str, Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled(format!("      • {:<26}", t.wizard_summary_backend), Style::default().fg(Color::Gray)),
            Span::styled(backend_str, Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled(format!("      • {:<26}", t.wizard_summary_programs), Style::default().fg(Color::Gray)),
            Span::styled(programs_str, Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled(format!("      • {:<26}", t.wizard_summary_config_path), Style::default().fg(Color::Gray)),
            Span::styled(&config_path, Style::default().fg(Color::Rgb(255, 204, 102))),
        ]),
        Line::raw(""),
        Line::raw(""),
        Line::from(vec![
            Span::styled(format!("   {}   ", t.wizard_finish_btn), Style::default().fg(Color::Black).bg(primary_color).add_modifier(Modifier::BOLD)),
        ]),
        Line::raw(""),
        Line::from(vec![
            Span::styled(format!("   {}", t.wizard_finish_hint), Style::default().fg(Color::DarkGray)),
        ]),
    ];

    frame.render_widget(Paragraph::new(lines).block(block), area);
}
