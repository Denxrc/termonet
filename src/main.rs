//! Termonet binary entry point.

use termonet::config::AppConfig;
use termonet::error::Result;
use termonet::tui::AppState;

fn main() -> Result<()> {
    let config = AppConfig::default();
    let state = AppState::new(config);

    println!(
        "Termonet v{} initialized (active step: {:?}). Ready to launch.",
        env!("CARGO_PKG_VERSION"),
        state.current_step
    );

    Ok(())
}
