//! Step 4: Final generation confirmation, file writing, and service reloading.

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::i18n::get_translations;
use crate::tui::app::AppState;

/// Renders the Step 4 view (execution confirmation and write progress).
pub fn render_apply(frame: &mut Frame, area: Rect, state: &AppState) {
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

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5), // Big Action Card
            Constraint::Min(6),    // List of programs to be updated
        ])
        .split(area);

    // 1. Action Card
    let action_lines = vec![
        Line::raw(""),
        Line::from(vec![
            Span::styled(format!("   {}   ", t.apply_button), Style::default().fg(Color::Black).bg(primary_color).add_modifier(Modifier::BOLD)),
        ]),
        Line::raw(""),
        Line::from(vec![
            Span::styled(format!("{} ", t.apply_status), Style::default().fg(secondary_color)),
            Span::styled(
                state.status_message.as_deref().unwrap_or(t.apply_ready),
                Style::default().fg(Color::White).add_modifier(Modifier::ITALIC),
            ),
        ]),
    ];
    let action_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(primary_color))
        .title(t.apply_title);
    frame.render_widget(Paragraph::new(action_lines).alignment(Alignment::Center).block(action_block), chunks[0]);

    // 2. Summary of selected applications
    let mut summary_lines = vec![
        Line::from(vec![
            Span::styled(format!(" {} ", t.apply_summary), Style::default().fg(secondary_color).add_modifier(Modifier::BOLD)),
        ]),
        Line::raw(""),
    ];

    let active_programs: Vec<_> = state.programs.iter().filter(|p| p.enabled).collect();
    if active_programs.is_empty() {
        summary_lines.push(Line::from(vec![
            Span::styled("   (Нет выбранных программ для обновления)", Style::default().fg(Color::DarkGray)),
        ]));
    } else {
        for prog in active_programs {
            summary_lines.push(Line::from(vec![
                Span::styled("   • ", Style::default().fg(primary_color)),
                Span::styled(format!("{:<14}", prog.name), Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
                Span::styled(" ➔  ", Style::default().fg(secondary_color)),
                Span::styled(&prog.output_path, Style::default().fg(Color::Gray)),
            ]));
        }
    }

    let summary_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray))
        .title(" Сводка применения ");
    frame.render_widget(Paragraph::new(summary_lines).block(summary_block), chunks[1]);
}
