//! Terminal initialization, raw mode handling, and RAII safe teardown.

use std::io::{self, Stdout};
use std::ops::{Deref, DerefMut};
use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use crate::error::Result;

pub type TuiTerminal = Terminal<CrosstermBackend<Stdout>>;

/// RAII Terminal Guard ensuring cooked mode and main screen buffer are always restored on drop.
pub struct TerminalGuard {
    terminal: Option<TuiTerminal>,
}

impl TerminalGuard {
    /// Initializes terminal in raw mode with alternate screen buffer.
    pub fn new() -> Result<Self> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend)?;
        Ok(Self {
            terminal: Some(terminal),
        })
    }
}

impl Deref for TerminalGuard {
    type Target = TuiTerminal;

    fn deref(&self) -> &Self::Target {
        self.terminal.as_ref().expect("terminal not initialized")
    }
}

impl DerefMut for TerminalGuard {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.terminal.as_mut().expect("terminal not initialized")
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        if let Some(mut term) = self.terminal.take() {
            let _ = execute!(term.backend_mut(), LeaveAlternateScreen);
            let _ = term.show_cursor();
        } else {
            let _ = execute!(io::stdout(), LeaveAlternateScreen);
        }
    }
}
