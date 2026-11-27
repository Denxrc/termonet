//! Step 2: Interactive supported applications list, registry discovery, and confirmation modals.

use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
};

use crate::i18n::get_translations;
use crate::registry::CatalogStatus;
use crate::tui::app::AppState;

const SPACES: &str = "              ";

/// Renders the Step 2 view (interactive program selection, registry badges, and modals).
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

    let mut header_spans = vec![
        Span::styled(
            format!(" {}/{} ", selected_count, total_count),
            Style::default()
                .fg(primary_color)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(t.programs_desc, Style::default().fg(secondary_color)),
    ];
    if let Some(ref msg) = state.status_message {
        let color = if state.status_is_error {
            Color::LightRed
        } else {
            Color::LightGreen
        };
        header_spans.push(Span::raw("  •  "));
        header_spans.push(Span::styled(msg.as_str(), Style::default().fg(color)));
    } else if let Some(ref notice) = state.registry_notice {
        header_spans.push(Span::raw("  •  "));
        header_spans.push(Span::styled(
            notice.as_str(),
            Style::default().fg(Color::Yellow),
        ));
    }

    frame.render_widget(Paragraph::new(Line::from(header_spans)), chunks[0]);

    let mut program_lines = Vec::new();
    if state.programs.is_empty() {
        program_lines.push(Line::raw(""));
        program_lines.push(Line::from(vec![
            Span::raw("   "),
            Span::styled(t.programs_empty, Style::default().fg(text_muted)),
        ]));
    } else {
        for (idx, program) in state.programs.iter().enumerate() {
            let is_cursor = idx == state.selected_program_idx;

            let cursor_marker = if is_cursor {
                Span::styled(
                    " ➔ ",
                    Style::default()
                        .fg(primary_color)
                        .add_modifier(Modifier::BOLD),
                )
            } else {
                Span::raw("   ")
            };

            let checkbox = match &program.catalog_status {
                Some(CatalogStatus::Available {
                    compatible: true, ..
                }) => Span::styled(
                    "[↓] ",
                    Style::default()
                        .fg(secondary_color)
                        .add_modifier(Modifier::BOLD),
                ),
                Some(CatalogStatus::Available {
                    compatible: false, ..
                }) => Span::styled("[✕] ", Style::default().fg(Color::LightRed)),
                _ if program.enabled => Span::styled(
                    "[✔] ",
                    Style::default()
                        .fg(primary_color)
                        .add_modifier(Modifier::BOLD),
                ),
                _ => Span::styled("[ ] ", Style::default().fg(text_dim)),
            };

            // Underline ONLY the name itself dynamically without trailing padding
            let name_style = if is_cursor {
                Style::default()
                    .fg(text_color)
                    .add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
            } else if program.enabled {
                Style::default().fg(text_color)
            } else {
                Style::default().fg(text_dim)
            };

            let pad_len = 14usize
                .saturating_sub(program.name.chars().count())
                .min(SPACES.len());

            let mut row_spans = vec![
                cursor_marker,
                checkbox,
                Span::styled(&program.name, name_style),
                Span::raw(&SPACES[..pad_len]),
                Span::raw(" "),
                Span::styled(t.programs_output, Style::default().fg(secondary_color)),
                Span::raw(" "),
                Span::styled(&program.output_path, Style::default().fg(text_muted)),
            ];

            match &program.catalog_status {
                Some(CatalogStatus::Installed {
                    update_available,
                    locally_modified,
                    ..
                }) => {
                    if let Some((rv, tv)) = update_available {
                        row_spans.push(Span::styled(
                            format!("  [↑ v{rv}/t{tv}]"),
                            Style::default()
                                .fg(Color::LightGreen)
                                .add_modifier(Modifier::BOLD),
                        ));
                    }
                    if *locally_modified {
                        row_spans.push(Span::styled(
                            format!("  {}", t.programs_badge_edited),
                            Style::default().fg(Color::Yellow),
                        ));
                    }
                }
                Some(CatalogStatus::LocalCustom) => {
                    row_spans.push(Span::styled(
                        format!("  {}", t.programs_badge_local),
                        Style::default().fg(secondary_color),
                    ));
                }
                _ => {}
            }

            program_lines.push(Line::from(row_spans));
            program_lines.push(Line::raw(""));
        }
    }

    let inner_height = chunks[1].height.saturating_sub(2);
    let cursor_row = (state.selected_program_idx as u16).saturating_mul(2);
    let scroll_y = if inner_height > 0 && cursor_row + 1 >= inner_height {
        cursor_row + 2 - inner_height
    } else {
        0
    };

    let programs_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(primary_color))
        .title(t.programs_title);

    frame.render_widget(
        Paragraph::new(program_lines)
            .block(programs_block)
            .scroll((scroll_y, 0)),
        chunks[1],
    );

    if state.confirm_modal.is_some() {
        render_confirm_modal(frame, area, state);
    }
}

/// Renders centered registry warning/confirmation modal over the active view.
pub fn render_confirm_modal(frame: &mut Frame, area: Rect, state: &AppState) {
    let Some(ref modal) = state.confirm_modal else {
        return;
    };
    let t = get_translations(state.config.language);
    let (primary_color, secondary_color) = state.ui_colors();
    let text_color = state.text_color();
    let text_muted = state.text_muted();
    let text_dim = state.text_dim();

    let mut body_lines: Vec<Line<'_>> = vec![Line::raw("")];
    match &modal.action {
        crate::tui::app::ConfirmAction::InstallProgram { prepared, .. }
        | crate::tui::app::ConfirmAction::WizardInstall { prepared, .. }
            if !prepared.is_empty() =>
        {
            body_lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled(
                    t.registry_modal_install_intro,
                    Style::default().fg(text_muted),
                ),
            ]));
            body_lines.push(Line::raw(""));

            for item in prepared {
                let pv = &item.preview;
                body_lines.push(Line::from(vec![
                    Span::styled(
                        "  ▸ ",
                        Style::default()
                            .fg(primary_color)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        pv.name.as_str(),
                        Style::default().fg(text_color).add_modifier(Modifier::BOLD),
                    ),
                    Span::raw("  "),
                    Span::styled(
                        format!("[{}]", pv.repo),
                        Style::default().fg(secondary_color),
                    ),
                    Span::styled(
                        format!(
                            "  ({} v{} · {} v{})",
                            t.registry_modal_rule_label,
                            pv.rule_version,
                            t.registry_modal_template_label,
                            pv.template_version
                        ),
                        Style::default().fg(text_dim),
                    ),
                ]));
                body_lines.push(Line::from(vec![
                    Span::styled("    • ", Style::default().fg(text_dim)),
                    Span::styled(
                        t.registry_modal_field_output,
                        Style::default().fg(secondary_color),
                    ),
                    Span::raw(" "),
                    Span::styled(pv.output_path.as_str(), Style::default().fg(text_color)),
                ]));
                if let Some(ref target) = pv.target_config {
                    body_lines.push(Line::from(vec![
                        Span::styled("    • ", Style::default().fg(text_dim)),
                        Span::styled(
                            t.registry_modal_field_target,
                            Style::default().fg(secondary_color),
                        ),
                        Span::raw(" "),
                        Span::styled(target.as_str(), Style::default().fg(text_muted)),
                    ]));
                }
                if let Some(ref cmd) = pv.reload_cmd {
                    body_lines.push(Line::from(vec![
                        Span::styled(
                            "    ⚠ ",
                            Style::default()
                                .fg(Color::Yellow)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(
                            t.registry_modal_field_hook,
                            Style::default()
                                .fg(Color::Yellow)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::raw(" "),
                        Span::styled(
                            cmd.as_str(),
                            Style::default()
                                .fg(Color::Yellow)
                                .add_modifier(Modifier::BOLD),
                        ),
                    ]));
                }
                if pv.has_local_conflict {
                    body_lines.push(Line::from(vec![
                        Span::raw("    "),
                        Span::styled(
                            t.registry_modal_conflict_hint,
                            Style::default()
                                .fg(Color::Yellow)
                                .add_modifier(Modifier::BOLD),
                        ),
                    ]));
                }
                body_lines.push(Line::raw(""));
            }
        }
        _ => {
            for line in &modal.warning_lines {
                if line.starts_with("• ") {
                    body_lines.push(Line::from(vec![
                        Span::raw("    "),
                        Span::styled("• ", Style::default().fg(secondary_color)),
                        Span::styled(
                            line.trim_start_matches("• "),
                            Style::default().fg(text_color).add_modifier(Modifier::BOLD),
                        ),
                    ]));
                } else {
                    let is_warning = line.starts_with('⚠')
                        || line.starts_with("WARNING")
                        || line.contains("Modified");
                    let style = if is_warning {
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(text_muted)
                    };
                    body_lines.push(Line::from(vec![
                        Span::raw("  "),
                        Span::styled(line.as_str(), style),
                    ]));
                }
            }
            body_lines.push(Line::raw(""));
        }
    }

    let modal_width = area.width.saturating_sub(6).clamp(52, 86).min(area.width);
    let max_h = area.height.saturating_sub(2).max(1);
    let modal_height = (body_lines.len() as u16)
        .saturating_add(4)
        .min(max_h)
        .max(9.min(area.height));
    let x = area.x + (area.width.saturating_sub(modal_width)) / 2;
    let y = area.y + (area.height.saturating_sub(modal_height)) / 2;
    let popup_area = Rect::new(x, y, modal_width, modal_height);

    frame.render_widget(Clear, popup_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(primary_color))
        .title(modal.title.as_str());
    let inner_area = block.inner(popup_area);
    frame.render_widget(block, popup_area);

    if inner_area.height == 0 || inner_area.width == 0 {
        return;
    }

    let footer_h = if inner_area.height >= 4 { 2 } else { 1 };
    let modal_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(footer_h)])
        .split(inner_area);

    let max_scroll = (body_lines.len() as u16).saturating_sub(modal_chunks[0].height);
    let effective_scroll = modal.scroll.min(max_scroll);

    frame.render_widget(
        Paragraph::new(body_lines)
            .wrap(Wrap { trim: false })
            .scroll((effective_scroll, 0)),
        modal_chunks[0],
    );

    let hint_line = if max_scroll > 0 {
        format!("{}   [↑/↓] {}", t.registry_confirm_hint, "Scroll")
    } else {
        t.registry_confirm_hint.to_string()
    };
    let footer_lines = if footer_h >= 2 {
        vec![
            Line::raw(""),
            Line::from(Span::styled(
                hint_line,
                Style::default()
                    .fg(secondary_color)
                    .add_modifier(Modifier::BOLD),
            ))
            .alignment(Alignment::Center),
        ]
    } else {
        vec![
            Line::from(Span::styled(
                hint_line,
                Style::default()
                    .fg(secondary_color)
                    .add_modifier(Modifier::BOLD),
            ))
            .alignment(Alignment::Center),
        ]
    };
    frame.render_widget(Paragraph::new(footer_lines), modal_chunks[1]);
}
