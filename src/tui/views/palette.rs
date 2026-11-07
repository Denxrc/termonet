//! Step 1: Palette preview, scheme selector, and wallpaper source indicator.

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::core::color::RgbColor;
use crate::tui::app::AppState;

/// Renders the Step 1 view (palette swatches and scheme controls).
pub fn render_palette(frame: &mut Frame, area: Rect, state: &AppState) {
    let Some(ref palette) = state.palette else {
        let no_palette_msg = Paragraph::new(Line::from(vec![
            Span::styled(
                "No wallpaper palette loaded. Pass an image path or use detection in v1.x.",
                Style::default().fg(Color::Yellow),
            ),
        ]))
        .alignment(ratatui::layout::Alignment::Center)
        .block(Block::default().borders(Borders::ALL).title(" Palette Preview "));
        frame.render_widget(no_palette_msg, area);
        return;
    };

    let scheme = if state.preview_dark {
        &palette.dark
    } else {
        &palette.light
    };

    let primary_color = Color::from(scheme.primary);
    let secondary_color = Color::from(scheme.secondary);

    // Split horizontally into 2 columns: Material You Roles (left) and Terminal ANSI Colors (right)
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    // --- Left Column: Material You Theme ---
    let theme_mode_label = if state.preview_dark {
        "Dark Scheme [d/l to toggle]"
    } else {
        "Light Scheme [d/l to toggle]"
    };

    let material_lines = vec![
        Line::from(vec![
            Span::styled(" Seed Dominant: ", Style::default().fg(secondary_color)),
            color_badge(&palette.seed_color),
            Span::raw(format!(" {} (rgb: {})", palette.seed_color.to_hex(), palette.seed_color.to_rgb_string())),
        ]),
        Line::raw(""),
        render_role_line("Primary", &scheme.primary),
        render_role_line("On Primary", &scheme.on_primary),
        render_role_line("Primary Container", &scheme.primary_container),
        render_role_line("Secondary", &scheme.secondary),
        render_role_line("Tertiary", &scheme.tertiary),
        render_role_line("Surface", &scheme.surface),
        render_role_line("Surface Variant", &scheme.surface_variant),
        render_role_line("Background", &scheme.background),
        render_role_line("Outline", &scheme.outline),
        render_role_line("Error", &scheme.error),
    ];

    let material_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(primary_color))
        .title(format!(" Material You Roles ({}) ", theme_mode_label));

    frame.render_widget(Paragraph::new(material_lines).block(material_block), columns[0]);

    // --- Right Column: Terminal ANSI Palette ---
    let t = &palette.terminal;
    let terminal_lines = vec![
        Line::from(vec![Span::styled(" [Normal ANSI Colors 0-7]", Style::default().fg(primary_color).add_modifier(Modifier::BOLD))]),
        render_terminal_line("0. Black", &t.color0),
        render_terminal_line("1. Red", &t.color1),
        render_terminal_line("2. Green", &t.color2),
        render_terminal_line("3. Yellow", &t.color3),
        render_terminal_line("4. Blue", &t.color4),
        render_terminal_line("5. Magenta", &t.color5),
        render_terminal_line("6. Cyan", &t.color6),
        render_terminal_line("7. White", &t.color7),
        Line::raw(""),
        Line::from(vec![Span::styled(" [Bright ANSI Colors 8-15]", Style::default().fg(primary_color).add_modifier(Modifier::BOLD))]),
        render_terminal_line("8. Bright Black", &t.color8),
        render_terminal_line("9. Bright Red", &t.color9),
        render_terminal_line("10. Bright Green", &t.color10),
        render_terminal_line("11. Bright Yellow", &t.color11),
        render_terminal_line("12. Bright Blue", &t.color12),
        render_terminal_line("13. Bright Magenta", &t.color13),
        render_terminal_line("14. Bright Cyan", &t.color14),
        render_terminal_line("15. Bright White", &t.color15),
    ];

    let terminal_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(secondary_color))
        .title(" Terminal 16-Color Palette ");

    frame.render_widget(Paragraph::new(terminal_lines).block(terminal_block), columns[1]);
}

fn color_badge(color: &RgbColor) -> Span<'static> {
    Span::styled(
        " ██ ",
        Style::default().fg(Color::from(*color)),
    )
}

fn render_role_line<'a>(label: &'static str, color: &RgbColor) -> Line<'a> {
    Line::from(vec![
        color_badge(color),
        Span::styled(format!(" {:<18}", label), Style::default().add_modifier(Modifier::BOLD)),
        Span::styled(format!(" {} ", color.to_hex()), Style::default().fg(Color::White)),
        Span::styled(format!("({})", color.to_rgb_string()), Style::default().fg(Color::DarkGray)),
    ])
}

fn render_terminal_line<'a>(label: &'static str, color: &RgbColor) -> Line<'a> {
    Line::from(vec![
        color_badge(color),
        Span::styled(format!(" {:<17}", label), Style::default()),
        Span::styled(format!(" {} ", color.to_hex()), Style::default().fg(Color::White)),
        Span::styled(format!("({})", color.to_rgb_string()), Style::default().fg(Color::DarkGray)),
    ])
}
