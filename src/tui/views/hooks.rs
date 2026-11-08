//! Step 3: Integration method selection and application reload hook commands.

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::i18n::get_translations;
use crate::tui::app::AppState;

/// Renders the Step 3 view (reload hook commands configuration).
pub fn render_hooks(frame: &mut Frame, area: Rect, state: &AppState) {
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
        .constraints([Constraint::Length(2), Constraint::Min(8)])
        .split(area);

    let desc_line = Line::from(vec![
        Span::styled(format!(" {} ", t.hooks_desc), Style::default().fg(secondary_color)),
    ]);
    frame.render_widget(Paragraph::new(desc_line), chunks[0]);

    let mut hook_lines = Vec::new();
    for (idx, hook) in state.hooks.iter().enumerate() {
        let is_cursor = idx == state.selected_hook_idx;

        let cursor_marker = if is_cursor {
            Span::styled(" ➔ ", Style::default().fg(primary_color).add_modifier(Modifier::BOLD))
        } else {
            Span::raw("   ")
        };

        let checkbox = if hook.enabled {
            Span::styled("[✔] ", Style::default().fg(primary_color).add_modifier(Modifier::BOLD))
        } else {
            Span::styled("[ ] ", Style::default().fg(Color::DarkGray))
        };

        let name_style = if is_cursor {
            Style::default().fg(Color::White).add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
        } else if hook.enabled {
            Style::default().fg(Color::White)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        hook_lines.push(Line::from(vec![
            cursor_marker,
            checkbox,
            Span::styled(format!("{:<14}", hook.program), name_style),
            Span::styled(format!("{:<34}", hook.command), Style::default().fg(Color::Cyan)),
            Span::styled(format!(" // {}", hook.description), Style::default().fg(Color::DarkGray)),
        ]));
        hook_lines.push(Line::raw(""));
    }

    let hooks_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(primary_color))
        .title(t.hooks_title);

    frame.render_widget(Paragraph::new(hook_lines).block(hooks_block), chunks[1]);
}
