//! Terminal initialization, raw mode handling, and RAII safe teardown.

use crate::error::Result;
use crossterm::{
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use std::io::{self, Stdout};
use std::ops::{Deref, DerefMut};

pub type TuiTerminal = Terminal<CrosstermBackend<Stdout>>;

/// RAII Terminal Guard ensuring cooked mode and main screen buffer are always restored on drop.
pub struct TerminalGuard {
    terminal: TuiTerminal,
}

impl TerminalGuard {
    /// Initializes terminal in raw mode with alternate screen buffer.
    pub fn new() -> Result<Self> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend)?;
        Ok(Self { terminal })
    }
}

impl Deref for TerminalGuard {
    type Target = TuiTerminal;

    fn deref(&self) -> &Self::Target {
        &self.terminal
    }
}

impl DerefMut for TerminalGuard {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.terminal
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(self.terminal.backend_mut(), LeaveAlternateScreen);
        let _ = self.terminal.show_cursor();
    }
}
