//! Step 1: Palette preview, scheme selector, and wallpaper source indicator.

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
};

use crate::core::color::RgbColor;
use crate::i18n::get_translations;
use crate::tui::app::AppState;

/// Renders the Step 1 view (palette swatches and scheme controls).
pub fn render_palette(frame: &mut Frame, area: Rect, state: &AppState) {
    let t = get_translations(state.config.language);

    let Some(ref palette) = state.palette else {
        let warn_color = if state.is_dark_mode() {
            Color::Yellow
        } else {
            Color::Rgb(170, 90, 0)
        };
        let no_palette_msg = Paragraph::new(Line::from(vec![Span::styled(
            t.no_wallpaper,
            Style::default().fg(warn_color),
        )]))
        .alignment(ratatui::layout::Alignment::Center)
        .block(Block::default().borders(Borders::ALL).title(t.roles_title));
        frame.render_widget(no_palette_msg, area);
        return;
    };

    let (primary_color, secondary_color) = state.ui_colors();
    let is_dark = state.is_dark_mode();
    let scheme = palette.scheme_for(is_dark);
    let text_color = state.text_color();
    let text_dim = state.text_dim();

    // Split vertically: top wallpaper source card (3 lines for wrapped header + status) and main palette columns
    let v_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(8)])
        .split(area);

    // 1. Wallpaper & Seed indicator inside Step 1
    let custom_color_str;
    let (label_text, wp_text) = if let Some(ref color) = state.custom_seed_color {
        custom_color_str = format!("{} (HEX)", color.to_hex());
        (t.seed_color, custom_color_str.as_str())
    } else {
        (
            t.wallpaper_source,
            state.wallpaper_path.as_deref().unwrap_or(t.no_wallpaper),
        )
    };

    let mut spans = vec![
        Span::raw(" "),
        Span::styled(
            label_text,
            Style::default()
                .fg(secondary_color)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" "),
        Span::styled(wp_text, Style::default().fg(text_color)),
        Span::raw("   "),
        Span::styled(
            t.wallpaper_change_btn,
            Style::default()
                .fg(primary_color)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("  "),
        Span::styled(
            t.wallpaper_reload_btn,
            Style::default()
                .fg(primary_color)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" "),
    ];

    if state.active_monitors.len() > 1 {
        spans.push(Span::raw("   "));
        spans.push(Span::styled(
            t.display_label,
            Style::default()
                .fg(secondary_color)
                .add_modifier(Modifier::BOLD),
        ));
        spans.push(Span::raw(" "));
        for (idx, (name, _)) in state.active_monitors.iter().enumerate() {
            let is_active = idx == state.current_monitor_idx;
            if is_active {
                spans.push(Span::styled(
                    "[● ",
                    Style::default()
                        .fg(primary_color)
                        .add_modifier(Modifier::BOLD),
                ));
                spans.push(Span::styled(
                    name.as_str(),
                    Style::default()
                        .fg(primary_color)
                        .add_modifier(Modifier::BOLD),
                ));
                spans.push(Span::styled(
                    "] ",
                    Style::default()
                        .fg(primary_color)
                        .add_modifier(Modifier::BOLD),
                ));
            } else {
                spans.push(Span::styled("[○ ", Style::default().fg(text_dim)));
                spans.push(Span::styled(name.as_str(), Style::default().fg(text_dim)));
                spans.push(Span::styled("] ", Style::default().fg(text_dim)));
            }
        }
    }

    spans.push(Span::raw("    "));
    spans.push(Span::styled(
        t.seed_color,
        Style::default()
            .fg(secondary_color)
            .add_modifier(Modifier::BOLD),
    ));
    spans.push(Span::raw(" "));
    spans.push(color_badge(&palette.seed_color));
    spans.push(Span::styled(
        format!(" {} ", palette.seed_color.to_hex()),
        Style::default().fg(text_color).add_modifier(Modifier::BOLD),
    ));

    let wp_line = Line::from(spans);
    let mut header_lines = vec![wp_line];
    if let Some(ref status) = state.status_message {
        let is_err = state.status_is_error;
        let status_color = if is_err {
            if is_dark {
                Color::LightRed
            } else {
                Color::Rgb(180, 20, 20)
            }
        } else {
            primary_color
        };
        header_lines.push(Line::from(vec![
            Span::raw(" "),
            Span::styled(
                if is_err { "✗ " } else { "✔ " },
                Style::default()
                    .fg(status_color)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(status.as_str(), Style::default().fg(status_color)),
        ]));
    }
    frame.render_widget(
        Paragraph::new(header_lines).wrap(Wrap { trim: false }),
        v_chunks[0],
    );

    // 2. Split horizontally into 2 columns: Material You Roles (left) and Terminal ANSI Colors (right)
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(v_chunks[1]);

    // --- Left Column: Material You Theme ---
    let material_lines = vec![
        render_role_line(t.role_primary, &scheme.primary, text_color, text_dim),
        render_role_line(t.role_on_primary, &scheme.on_primary, text_color, text_dim),
        render_role_line(
            t.role_primary_container,
            &scheme.primary_container,
            text_color,
            text_dim,
        ),
        render_role_line(t.role_secondary, &scheme.secondary, text_color, text_dim),
        render_role_line(t.role_tertiary, &scheme.tertiary, text_color, text_dim),
        render_role_line(t.role_surface, &scheme.surface, text_color, text_dim),
        render_role_line(
            t.role_surface_variant,
            &scheme.surface_variant,
            text_color,
            text_dim,
        ),
        render_role_line(t.role_background, &scheme.background, text_color, text_dim),
        render_role_line(t.role_outline, &scheme.outline, text_color, text_dim),
        render_role_line(t.role_error, &scheme.error, text_color, text_dim),
    ];

    let material_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(primary_color))
        .title(t.roles_title);

    frame.render_widget(
        Paragraph::new(material_lines).block(material_block),
        columns[0],
    );

    // --- Right Column: Terminal ANSI Palette ---
    let term = palette.terminal_for(is_dark);
    let terminal_lines = vec![
        Line::from(vec![Span::styled(
            t.normal_colors,
            Style::default()
                .fg(primary_color)
                .add_modifier(Modifier::BOLD),
        )]),
        render_terminal_line(0, &term.color0, text_color, text_dim),
        render_terminal_line(1, &term.color1, text_color, text_dim),
        render_terminal_line(2, &term.color2, text_color, text_dim),
        render_terminal_line(3, &term.color3, text_color, text_dim),
        render_terminal_line(4, &term.color4, text_color, text_dim),
        render_terminal_line(5, &term.color5, text_color, text_dim),
        render_terminal_line(6, &term.color6, text_color, text_dim),
        render_terminal_line(7, &term.color7, text_color, text_dim),
        Line::raw(""),
        Line::from(vec![Span::styled(
            t.bright_colors,
            Style::default()
                .fg(primary_color)
                .add_modifier(Modifier::BOLD),
        )]),
        render_terminal_line(8, &term.color8, text_color, text_dim),
        render_terminal_line(9, &term.color9, text_color, text_dim),
        render_terminal_line(10, &term.color10, text_color, text_dim),
        render_terminal_line(11, &term.color11, text_color, text_dim),
        render_terminal_line(12, &term.color12, text_color, text_dim),
        render_terminal_line(13, &term.color13, text_color, text_dim),
        render_terminal_line(14, &term.color14, text_color, text_dim),
        render_terminal_line(15, &term.color15, text_color, text_dim),
    ];

    let terminal_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(secondary_color))
        .title(t.terminal_title);

    frame.render_widget(
        Paragraph::new(terminal_lines).block(terminal_block),
        columns[1],
    );

    // 3. Render wallpaper input modal dialog if active
    if state.wallpaper_input.is_some() {
        render_wallpaper_input_modal(frame, area, state);
    }
}

/// Renders an interactive modal popup for typing or pasting manual wallpaper path.
pub fn render_wallpaper_input_modal(frame: &mut Frame, area: Rect, state: &AppState) {
    let Some(ref input_state) = state.wallpaper_input else {
        return;
    };
    let t = get_translations(state.config.language);

    let (primary_color, _) = state.ui_colors();
    let text_color = state.text_color();
    let text_muted = state.text_muted();
    let text_dim = state.text_dim();

    // Centered modal window: 72 columns wide or 75% width, 7 lines height
    let popup_area = centered_rect(75, 7, area);
    frame.render_widget(ratatui::widgets::Clear, popup_area);

    // Horizontally scroll input around cursor so long paths remain visible inside the modal
    let (raw_before, raw_after) = if input_state.input.is_char_boundary(input_state.cursor_pos) {
        (
            &input_state.input[..input_state.cursor_pos],
            &input_state.input[input_state.cursor_pos..],
        )
    } else {
        (input_state.input.as_str(), "")
    };
    let visible_cols = (popup_area.width as usize).saturating_sub(8).max(8);
    let before_chars = raw_before.chars().count();
    let max_before = visible_cols.saturating_sub(1);
    let visible_before = if before_chars > max_before {
        let skip = before_chars - max_before;
        raw_before
            .char_indices()
            .nth(skip)
            .map(|(idx, _)| &raw_before[idx..])
            .unwrap_or("")
    } else {
        raw_before
    };
    let remaining_cols = visible_cols
        .saturating_sub(visible_before.chars().count())
        .saturating_sub(1);
    let visible_after = raw_after
        .char_indices()
        .nth(remaining_cols)
        .map(|(idx, _)| &raw_after[..idx])
        .unwrap_or(raw_after);

    let mut lines = vec![
        Line::from(vec![Span::styled(
            format!("  {} ", t.wallpaper_input_prompt),
            Style::default().fg(text_muted),
        )]),
        Line::raw(""),
        Line::from(vec![
            Span::styled(
                "  > ",
                Style::default()
                    .fg(primary_color)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(visible_before, Style::default().fg(text_color)),
            Span::styled(
                "█",
                Style::default()
                    .fg(primary_color)
                    .add_modifier(Modifier::SLOW_BLINK),
            ),
            Span::styled(visible_after, Style::default().fg(text_color)),
        ]),
    ];

    if let Some(ref err) = input_state.error_msg {
        let err_color = if state.is_dark_mode() {
            Color::LightRed
        } else {
            Color::Rgb(180, 20, 20)
        };
        lines.push(Line::from(vec![
            Span::styled(
                "  ✗ ",
                Style::default().fg(err_color).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                err.as_str(),
                Style::default().fg(err_color).add_modifier(Modifier::BOLD),
            ),
        ]));
    } else {
        lines.push(Line::raw(""));
    }

    lines.push(Line::from(vec![
        Span::raw("  "),
        Span::styled(t.wallpaper_input_hint, Style::default().fg(text_dim)),
    ]));

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(
            Style::default()
                .fg(primary_color)
                .add_modifier(Modifier::BOLD),
        )
        .title(t.wallpaper_input_title);

    frame.render_widget(Paragraph::new(lines).block(block), popup_area);
}

/// Helper function to create a centered Rect within given area.
fn centered_rect(percent_x: u16, height: u16, r: Rect) -> Rect {
    let top_margin = r.height.saturating_sub(height) / 2;
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(top_margin),
            Constraint::Length(height),
            Constraint::Min(0),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

fn color_badge(color: &RgbColor) -> Span<'static> {
    Span::styled(" ██ ", Style::default().fg(Color::from(*color)))
}

const SPACES: &str = "                    ";

const TERMINAL_LABELS: [&str; 16] = [
    " 0. Black          ",
    " 1. Red            ",
    " 2. Green          ",
    " 3. Yellow         ",
    " 4. Blue           ",
    " 5. Magenta        ",
    " 6. Cyan           ",
    " 7. White          ",
    " 8. Bright Black   ",
    " 9. Bright Red     ",
    " 10. Bright Green  ",
    " 11. Bright Yellow ",
    " 12. Bright Blue   ",
    " 13. Bright Magenta",
    " 14. Bright Cyan   ",
    " 15. Bright White  ",
];

fn render_role_line<'a>(
    label: &'static str,
    color: &RgbColor,
    text_color: Color,
    text_dim: Color,
) -> Line<'a> {
    let pad_len = 20usize
        .saturating_sub(label.chars().count())
        .min(SPACES.len());
    Line::from(vec![
        color_badge(color),
        Span::raw(" "),
        Span::styled(
            label,
            Style::default().fg(text_color).add_modifier(Modifier::BOLD),
        ),
        Span::raw(&SPACES[..pad_len]),
        Span::raw(" "),
        Span::styled(color.to_hex(), Style::default().fg(text_color)),
        Span::raw(" ("),
        Span::styled(
            format!("{}, {}, {}", color.r, color.g, color.b),
            Style::default().fg(text_dim),
        ),
        Span::raw(")"),
    ])
}

fn render_terminal_line<'a>(
    label_idx: usize,
    color: &RgbColor,
    text_color: Color,
    text_dim: Color,
) -> Line<'a> {
    Line::from(vec![
        color_badge(color),
        Span::styled(TERMINAL_LABELS[label_idx], Style::default().fg(text_color)),
        Span::raw(" "),
        Span::styled(color.to_hex(), Style::default().fg(text_color)),
        Span::raw(" ("),
        Span::styled(
            format!("{}, {}, {}", color.r, color.g, color.b),
            Style::default().fg(text_dim),
        ),
        Span::raw(")"),
    ])
}
