//! Responsive top bar rendering with pixel/standard title and clean underlined step tabs.

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

/// Renders the top header bar including logo and clean step indicators.
pub fn render_header(frame: &mut Frame, area: Rect, state: &AppState) {
    let t = get_translations(state.config.language);

    // Accent colors from generated palette if available, or vibrant Material defaults
    let (primary_color, secondary_color) = if let Some(ref pal) = state.palette {
        let scheme = match state.config.tui_theme {
            crate::config::TuiTheme::Light => &pal.light,
            _ => &pal.dark,
        };
        (Color::from(scheme.primary), Color::from(scheme.secondary))
    } else {
        (Color::Rgb(173, 198, 255), Color::Rgb(191, 198, 220))
    };

    // Responsive check: if area height is large enough and width >= 48, render 3-line pixel logo
    let use_pixel = area.height >= 5 && area.width >= 48;

    if use_pixel {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // 3-line 5px pixel font logo
                Constraint::Length(2), // Step navigation tabs line
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

        // 2. Navigation Tabs
        render_step_tabs(frame, chunks[1], state, t, primary_color, secondary_color);
    } else {
        // Compact single-line header fallback
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // Compact title
                Constraint::Length(2), // Step navigation tabs line
            ])
            .split(area);

        let compact_title = Line::from(vec![
            Span::styled("TERMONET", Style::default().fg(primary_color).add_modifier(Modifier::BOLD)),
        ]);
        frame.render_widget(Paragraph::new(compact_title).alignment(Alignment::Center), chunks[0]);

        render_step_tabs(frame, chunks[1], state, t, primary_color, secondary_color);
    }
}

fn render_step_tabs(
    frame: &mut Frame,
    area: Rect,
    state: &AppState,
    t: &crate::i18n::Translations,
    primary_color: Color,
    secondary_color: Color,
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
            // Clean, non-cluttered active indicator: just bold and underlined with primary color
            tab_spans.push(Span::styled(
                label,
                Style::default()
                    .fg(primary_color)
                    .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
            ));
        } else {
            // Inactive tabs: clean secondary color
            tab_spans.push(Span::styled(
                label,
                Style::default().fg(secondary_color),
            ));
        }
        tab_spans.push(Span::raw("       ")); // Clean spacing between tabs
    }

    let tabs_paragraph = Paragraph::new(Line::from(tab_spans))
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::BOTTOM)
                .border_style(Style::default().fg(Color::DarkGray)),
        );

    frame.render_widget(tabs_paragraph, area);
}
