//! Terminal initialization, raw mode handling, and teardown.

use std::io::{self, Stdout};
use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use crate::error::Result;

pub type TuiTerminal = Terminal<CrosstermBackend<Stdout>>;

/// Initializes terminal in raw mode with alternate screen buffer.
pub fn init_terminal() -> Result<TuiTerminal> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let terminal = Terminal::new(backend)?;
    Ok(terminal)
}

/// Restores terminal back to original cooked mode and main screen.
pub fn restore_terminal(mut terminal: TuiTerminal) -> Result<()> {
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}
