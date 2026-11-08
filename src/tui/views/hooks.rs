//! Step 3: Dynamic application reload hooks based on enabled programs.

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::i18n::get_translations;
use crate::tui::app::AppState;

/// Renders the Step 3 view (dynamic reload hooks for selected applications).
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

    let active_indices = state.active_hook_indices();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(2), Constraint::Min(8)])
        .split(area);

    let desc_line = Line::from(vec![
        Span::styled(format!(" {} ", t.hooks_desc), Style::default().fg(secondary_color)),
        Span::raw(" ("),
        Span::styled(format!("{}/{}", active_indices.iter().filter(|&&i| state.programs[i].hook_enabled).count(), active_indices.len()), Style::default().fg(primary_color).add_modifier(Modifier::BOLD)),
        Span::raw(" активных команд)"),
    ]);
    frame.render_widget(Paragraph::new(desc_line), chunks[0]);

    let mut hook_lines = Vec::new();
    if active_indices.is_empty() {
        hook_lines.push(Line::raw(""));
        hook_lines.push(Line::from(vec![
            Span::styled("   (Нет активных хуков: выбранные на шаге 2 программы не требуют перезапуска или отключены)", Style::default().fg(Color::DarkGray)),
        ]));
    } else {
        for (cursor_idx, &prog_idx) in active_indices.iter().enumerate() {
            let prog = &state.programs[prog_idx];
            let is_cursor = cursor_idx == state.selected_hook_idx;

            let cursor_marker = if is_cursor {
                Span::styled(" ➔ ", Style::default().fg(primary_color).add_modifier(Modifier::BOLD))
            } else {
                Span::raw("   ")
            };

            let checkbox = if prog.hook_enabled {
                Span::styled("[✔] ", Style::default().fg(primary_color).add_modifier(Modifier::BOLD))
            } else {
                Span::styled("[ ] ", Style::default().fg(Color::DarkGray))
            };

            // Underline ONLY the name itself dynamically without trailing padding
            let name_style = if is_cursor {
                Style::default().fg(Color::White).add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
            } else if prog.hook_enabled {
                Style::default().fg(Color::White)
            } else {
                Style::default().fg(Color::DarkGray)
            };

            let pad_len = 14usize.saturating_sub(prog.name.len());
            let cmd_text = prog.reload_cmd.as_deref().unwrap_or("");
            let desc_text = prog.reload_desc.as_deref().unwrap_or("");

            hook_lines.push(Line::from(vec![
                cursor_marker,
                checkbox,
                Span::styled(&prog.name, name_style),
                Span::raw(" ".repeat(pad_len)),
                Span::styled(format!("{:<34}", cmd_text), Style::default().fg(Color::Cyan)),
                Span::styled(format!(" // {}", desc_text), Style::default().fg(Color::DarkGray)),
            ]));
            hook_lines.push(Line::raw(""));
        }
    }

    let hooks_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(primary_color))
        .title(t.hooks_title);

    frame.render_widget(Paragraph::new(hook_lines).block(hooks_block), chunks[1]);
}
