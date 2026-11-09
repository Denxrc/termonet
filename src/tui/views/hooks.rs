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

const SPACES: &str = "              ";

/// Renders the Step 3 view (dynamic reload hooks for selected applications).
pub fn render_hooks(frame: &mut Frame, area: Rect, state: &AppState) {
    let t = get_translations(state.config.language);

    let (primary_color, secondary_color) = state.ui_colors();
    let text_color = state.text_color();
    let text_dim = state.text_dim();
    let cmd_color = if state.is_dark_mode() {
        Color::Cyan
    } else if let Some(scheme) = state.active_scheme() {
        Color::from(scheme.tertiary)
    } else {
        Color::Blue
    };

    let active_indices = state.active_hook_indices();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(2), Constraint::Min(8)])
        .split(area);

    let desc_line = Line::from(vec![
        Span::styled(format!(" {} ", t.hooks_desc), Style::default().fg(secondary_color)),
        Span::raw(" ("),
        Span::styled(format!("{}/{}", active_indices.iter().filter(|&&i| state.programs[i].hook_enabled).count(), active_indices.len()), Style::default().fg(primary_color).add_modifier(Modifier::BOLD)),
        Span::styled(format!(" {})", t.hooks_active), Style::default().fg(secondary_color)),
    ]);
    frame.render_widget(Paragraph::new(desc_line), chunks[0]);

    let mut hook_lines = Vec::new();
    if active_indices.is_empty() {
        hook_lines.push(Line::raw(""));
        hook_lines.push(Line::from(vec![
            Span::styled(format!("   {}", t.hooks_empty), Style::default().fg(text_dim)),
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
                Span::styled("[ ] ", Style::default().fg(text_dim))
            };

            // Underline ONLY the name itself dynamically without trailing padding
            let name_style = if is_cursor {
                Style::default().fg(text_color).add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
            } else if prog.hook_enabled {
                Style::default().fg(text_color)
            } else {
                Style::default().fg(text_dim)
            };

            let pad_len = 14usize.saturating_sub(prog.name.len()).min(SPACES.len());
            let cmd_text = prog.reload_cmd.as_deref().unwrap_or("");
            let desc_text = prog.reload_desc.as_deref().unwrap_or("");

            hook_lines.push(Line::from(vec![
                cursor_marker,
                checkbox,
                Span::styled(&prog.name, name_style),
                Span::raw(&SPACES[..pad_len]),
                Span::styled(format!("{:<34}", cmd_text), Style::default().fg(cmd_color)),
                Span::styled(format!(" // {}", desc_text), Style::default().fg(text_dim)),
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
