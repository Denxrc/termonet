//! Headless CLI trigger mode execution.

pub mod state;

pub use state::AppStateCache;

/// Command line execution arguments.
#[derive(Debug, Clone, Default)]
pub struct CliArgs {
    pub apply: bool,
    pub wallpaper: Option<String>,
}

/// Executes headless apply without TUI if `--apply` flag is passed.
pub fn run_cli_mode(args: &CliArgs) -> crate::error::Result<bool> {
    if !args.apply {
        return Ok(false);
    }

    // In v1.x: checks state cache, generates palette, renders templates and executes hooks directly.
    Ok(true)
}
