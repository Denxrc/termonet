//! Responsive top bar rendering with pixel/standard title and step navigation tabs.

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::i18n::get_translations;
use crate::tui::app::{AppState, Step};
use crate::tui::font::PixelFont;

/// Renders the top header bar including logo and step indicators.
pub fn render_header(frame: &mut Frame, area: Rect, state: &AppState) {
    let t = get_translations(state.config.language);

    // Accent colors from generated palette if available, or vibrant Material defaults
    let (primary_color, secondary_color, bg_color) = if let Some(ref pal) = state.palette {
        let scheme = if state.preview_dark { &pal.dark } else { &pal.light };
        (
            Color::from(scheme.primary),
            Color::from(scheme.secondary),
            Color::from(scheme.surface),
        )
    } else {
        (
            Color::Rgb(173, 198, 255), // Default Material You blue
            Color::Rgb(191, 198, 220),
            Color::Rgb(17, 19, 24),
        )
    };

    // Responsive check: if area height is large enough and width >= 45, render pixel logo
    let use_pixel = area.height >= 7 && area.width >= 48;

    if use_pixel {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // 3-line 5px pixel font logo
                Constraint::Length(1), // Subtitle / Wallpaper source
                Constraint::Length(3), // Step navigation tabs block
            ])
            .split(area);

        // 1. Pixel Logo
        let pixel_lines = PixelFont::render_lines(
            "TERMONET",
            Style::default()
                .fg(primary_color)
                .add_modifier(Modifier::BOLD),
        );
        let logo_paragraph = Paragraph::new(pixel_lines).alignment(Alignment::Center);
        frame.render_widget(logo_paragraph, chunks[0]);

        // 2. Wallpaper indicator
        let wp_text = state.wallpaper_path.as_deref().unwrap_or("No wallpaper selected");
        let wp_line = Line::from(vec![
            Span::styled(" Wallpaper: ", Style::default().fg(secondary_color)),
            Span::styled(wp_text, Style::default().fg(Color::White).add_modifier(Modifier::ITALIC)),
        ]);
        let wp_paragraph = Paragraph::new(wp_line).alignment(Alignment::Center);
        frame.render_widget(wp_paragraph, chunks[1]);

        // 3. Navigation Tabs
        render_step_tabs(frame, chunks[2], state, t, primary_color, secondary_color, bg_color);
    } else {
        // Compact single-line header fallback
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // Compact title & wallpaper
                Constraint::Length(3), // Step navigation tabs block
            ])
            .split(area);

        let compact_title = Line::from(vec![
            Span::styled(" [ TERMONET ] ", Style::default().fg(primary_color).add_modifier(Modifier::BOLD)),
            Span::styled(" • ", Style::default().fg(secondary_color)),
            Span::styled(
                state.wallpaper_path.as_deref().unwrap_or("Manual Mode"),
                Style::default().fg(Color::Gray),
            ),
        ]);
        frame.render_widget(Paragraph::new(compact_title).alignment(Alignment::Center), chunks[0]);

        render_step_tabs(frame, chunks[1], state, t, primary_color, secondary_color, bg_color);
    }
}

fn render_step_tabs(
    frame: &mut Frame,
    area: Rect,
    state: &AppState,
    t: &crate::i18n::Translations,
    primary_color: Color,
    secondary_color: Color,
    _bg_color: Color,
) {
    let steps = [
        (Step::Palette, t.tab_palette),
        (Step::Programs, t.tab_programs),
        (Step::Hooks, t.tab_hooks),
        (Step::Apply, t.tab_apply),
    ];

    let mut tab_spans = Vec::new();
    for (step, label) in steps {
        let is_active = state.current_step == step;
        if is_active {
            tab_spans.push(Span::styled(
                format!(" [● {}] ", label),
                Style::default()
                    .fg(primary_color)
                    .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
            ));
        } else {
            tab_spans.push(Span::styled(
                format!("  ○ {}  ", label),
                Style::default().fg(secondary_color),
            ));
        }
        tab_spans.push(Span::raw("   "));
    }

    let tabs_paragraph = Paragraph::new(Line::from(tab_spans))
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::BOTTOM)
                .border_style(Style::default().fg(secondary_color)),
        );

    frame.render_widget(tabs_paragraph, area);
}
