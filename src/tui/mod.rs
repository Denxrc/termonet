//! Terminal User Interface (TUI) components and event loop.

pub mod app;
pub mod font;
pub mod terminal;
pub mod views;

pub use app::{AppState, Step};
pub use font::PixelFont;
pub use terminal::{init_terminal, restore_terminal, TuiTerminal};
