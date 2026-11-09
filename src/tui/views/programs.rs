//! Step 2: Interactive supported applications list, output paths, and live toggles.

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::i18n::get_translations;
use crate::tui::app::AppState;

const SPACES: &str = "              ";

/// Renders the Step 2 view (interactive program selection and paths).
pub fn render_programs(frame: &mut Frame, area: Rect, state: &AppState) {
    let t = get_translations(state.config.language);

    let (primary_color, secondary_color) = state.ui_colors();
    let text_color = state.text_color();
    let text_muted = state.text_muted();
    let text_dim = state.text_dim();

    let selected_count = state.programs.iter().filter(|p| p.enabled).count();
    let total_count = state.programs.len();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(2), Constraint::Min(8)])
        .split(area);

    let counter_line = Line::from(vec![
        Span::styled(format!(" {}/{} ", selected_count, total_count), Style::default().fg(primary_color).add_modifier(Modifier::BOLD)),
        Span::styled(t.programs_desc, Style::default().fg(secondary_color)),
    ]);
    frame.render_widget(Paragraph::new(counter_line), chunks[0]);

    let mut program_lines = Vec::new();
    for (idx, program) in state.programs.iter().enumerate() {
        let is_cursor = idx == state.selected_program_idx;

        let cursor_marker = if is_cursor {
            Span::styled(" ➔ ", Style::default().fg(primary_color).add_modifier(Modifier::BOLD))
        } else {
            Span::raw("   ")
        };

        let checkbox = if program.enabled {
            Span::styled("[✔] ", Style::default().fg(primary_color).add_modifier(Modifier::BOLD))
        } else {
            Span::styled("[ ] ", Style::default().fg(text_dim))
        };

        // Underline ONLY the name itself dynamically without trailing padding
        let name_style = if is_cursor {
            Style::default().fg(text_color).add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
        } else if program.enabled {
            Style::default().fg(text_color)
        } else {
            Style::default().fg(text_dim)
        };

        let pad_len = 14usize.saturating_sub(program.name.len()).min(SPACES.len());

        program_lines.push(Line::from(vec![
            cursor_marker,
            checkbox,
            Span::styled(&program.name, name_style),
            Span::raw(&SPACES[..pad_len]),
            Span::raw(" "),
            Span::styled(t.programs_output, Style::default().fg(secondary_color)),
            Span::raw(" "),
            Span::styled(&program.output_path, Style::default().fg(text_muted)),
        ]));
        program_lines.push(Line::raw(""));
    }

    let programs_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(primary_color))
        .title(t.programs_title);

    frame.render_widget(Paragraph::new(program_lines).block(programs_block), chunks[1]);
}
