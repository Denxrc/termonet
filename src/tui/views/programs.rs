//! Step 2: Supported applications list, output paths, and selection counters.

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::tui::app::AppState;

/// Renders the Step 2 view (program checkboxes and path list).
pub fn render_programs(frame: &mut Frame, area: Rect, state: &AppState) {
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

    let sample_programs = [
        ("Kitty", "~/.config/kitty/colors-termonet.conf", true),
        ("Waybar", "~/.config/waybar/colors.css", true),
        ("Rofi", "~/.config/rofi/colors.rasi", true),
        ("Alacritty", "~/.config/alacritty/colors.toml", false),
        ("Dunst", "~/.config/dunst/dunstrc.d/colors.conf", false),
        ("Hyprland", "~/.config/hypr/colors.conf", true),
        ("Sway", "~/.config/sway/colors.conf", false),
    ];

    let selected_count = sample_programs.iter().filter(|(_, _, s)| *s).count();
    let total_count = sample_programs.len();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(2), Constraint::Min(8)])
        .split(area);

    let counter_line = Line::from(vec![
        Span::styled(" Selected Applications: ", Style::default().fg(secondary_color)),
        Span::styled(
            format!("{}/{} active", selected_count, total_count),
            Style::default().fg(primary_color).add_modifier(Modifier::BOLD),
        ),
        Span::raw(" (Full selection & custom TOML rule management enabled in v1.x)"),
    ]);
    frame.render_widget(Paragraph::new(counter_line), chunks[0]);

    let mut program_lines = Vec::new();
    for (name, path, is_selected) in sample_programs {
        let (checkbox, name_style) = if is_selected {
            (
                Span::styled("[✔] ", Style::default().fg(primary_color).add_modifier(Modifier::BOLD)),
                Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
            )
        } else {
            (
                Span::styled("[ ] ", Style::default().fg(Color::DarkGray)),
                Style::default().fg(Color::DarkGray),
            )
        };

        program_lines.push(Line::from(vec![
            checkbox,
            Span::styled(format!("{:<14}", name), name_style),
            Span::styled(" ➔  Output: ", Style::default().fg(secondary_color)),
            Span::styled(path, Style::default().fg(Color::Gray)),
        ]));
        program_lines.push(Line::raw(""));
    }

    let programs_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(primary_color))
        .title(" Supported Applications & Config Destinations ");

    frame.render_widget(Paragraph::new(program_lines).block(programs_block), chunks[1]);
}
