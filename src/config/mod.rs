//! Configuration management, paths, and settings.

pub mod paths;
pub mod settings;
pub mod state;

pub use paths::AppPaths;
pub use settings::{AppConfig, DisplayServer, FontMode, Language, TuiTheme, WallpaperBackend};
pub use state::AppStateCache;
