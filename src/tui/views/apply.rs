//! Step 4: Final generation confirmation, file writing, and service reloading.

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::tui::app::AppState;

/// Renders the Step 4 view (execution confirmation and write progress).
pub fn render_apply(frame: &mut Frame, area: Rect, state: &AppState) {
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
            Constraint::Length(5), // Security notice
            Constraint::Length(7), // Big Action Card
            Constraint::Min(4),    // Backup & Safety stats
        ])
        .split(area);

    // 1. Security badge
    let security_lines = vec![
        Line::from(vec![
            Span::styled(" [READ-ONLY SAFETY ACTIVE] ", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
            Span::raw("Termonet does not modify any files until you explicitly trigger generation."),
        ]),
        Line::from(vec![
            Span::styled(" Dual-Layer Backup: ", Style::default().fg(secondary_color)),
            Span::raw("Master immutable backup (.original) + Safety snapshot (.bak) will be taken automatically."),
        ]),
        Line::from(vec![
            Span::styled(" In-Memory Staging: ", Style::default().fg(secondary_color)),
            Span::raw("Templates are rendered in RAM first; broken templates will never overwrite your live configs."),
        ]),
    ];
    let security_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray))
        .title(" Integrity & Safety Status ");
    frame.render_widget(Paragraph::new(security_lines).block(security_block), chunks[0]);

    // 2. Action Card
    let action_lines = vec![
        Line::raw(""),
        Line::from(vec![
            Span::styled("   ▶  [ PRESS 'ENTER' TO APPLY THEME & DISPATCH HOOKS ]   ", Style::default().fg(Color::Black).bg(primary_color).add_modifier(Modifier::BOLD)),
        ]),
        Line::raw(""),
        Line::from(vec![
            Span::styled("Status: ", Style::default().fg(secondary_color)),
            Span::styled(
                state.status_message.as_deref().unwrap_or("Ready for generation"),
                Style::default().fg(Color::White).add_modifier(Modifier::ITALIC),
            ),
        ]),
    ];
    let action_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(primary_color))
        .title(" Execution Trigger ");
    frame.render_widget(Paragraph::new(action_lines).alignment(Alignment::Center).block(action_block), chunks[1]);

    // 3. Roadmap Note
    let roadmap_lines = vec![
        Line::from(vec![
            Span::styled(" Roadmap Note (v0.x Prototype): ", Style::default().fg(secondary_color)),
            Span::raw("Direct template writing & hook dispatch will be activated in Milestone v1.0."),
        ]),
        Line::from(vec![
            Span::styled(" Headless CLI Trigger: ", Style::default().fg(secondary_color)),
            Span::raw("Will allow running step 4 directly via `termonet --apply` without opening TUI."),
        ]),
    ];
    let roadmap_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray))
        .title(" Architecture Overview ");
    frame.render_widget(Paragraph::new(roadmap_lines).block(roadmap_block), chunks[2]);
}
