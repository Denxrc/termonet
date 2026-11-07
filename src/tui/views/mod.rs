//! Individual TUI views and widgets.

pub mod apply;
pub mod header;
pub mod hooks;
pub mod palette;
pub mod programs;
pub mod wizard;

pub use apply::render_apply;
pub use header::render_header;
pub use hooks::render_hooks;
pub use palette::render_palette;
pub use programs::render_programs;
pub use wizard::render_wizard;
