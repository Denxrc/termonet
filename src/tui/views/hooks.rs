//! Step 3: Integration method selection and application reload hook commands.

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::tui::app::AppState;

/// Renders the Step 3 view (reload hook commands configuration).
pub fn render_hooks(frame: &mut Frame, area: Rect, state: &AppState) {
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

    let hook_items = [
        ("Waybar", "killall -SIGUSR2 waybar", "Instant bar reload"),
        ("Kitty", "killall -SIGUSR1 kitty", "Live color scheme reload"),
        ("Dunst", "killall -SIGUSR1 dunst", "Notification daemon reload"),
        ("Hyprland", "hyprctl reload", "Hyprland compositor reload"),
        ("Custom Hook", "~/.config/termonet/post-hook.sh", "User post-theme script"),
    ];

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(2), Constraint::Min(8)])
        .split(area);

    let desc_line = Line::from(vec![
        Span::styled(" Safe Decoupled Execution: ", Style::default().fg(secondary_color)),
        Span::raw("All reload hooks are executed with detached stdin/stdout to prevent UI blocking."),
    ]);
    frame.render_widget(Paragraph::new(desc_line), chunks[0]);

    let mut hook_lines = Vec::new();
    for (prog, cmd, note) in hook_items {
        hook_lines.push(Line::from(vec![
            Span::styled(" [⚙] ", Style::default().fg(primary_color)),
            Span::styled(format!("{:<15}", prog), Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
            Span::styled(format!("{:<36}", cmd), Style::default().fg(Color::Cyan)),
            Span::styled(format!("// {}", note), Style::default().fg(Color::DarkGray)),
        ]));
        hook_lines.push(Line::raw(""));
    }

    let hooks_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(primary_color))
        .title(" Application Reload Hooks & Automation ");

    frame.render_widget(Paragraph::new(hook_lines).block(hooks_block), chunks[1]);
}
