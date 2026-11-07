//! Termonet binary entry point.

use std::env;
use termonet::config::AppConfig;
use termonet::core::GeneratedPalette;
use termonet::error::Result;
use termonet::tui::{run_tui, AppState};

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();

    let config = AppConfig::default();
    let mut state = AppState::new(config);

    if args.len() > 1 {
        let arg = &args[1];
        if arg == "-h" || arg == "--help" {
            println!("Termonet v{}", env!("CARGO_PKG_VERSION"));
            println!("Usage: termonet [wallpaper-path]");
            println!("\nKeybindings in TUI:");
            println!("  Tab / Shift+Tab   Navigate between steps");
            println!("  1, 2, 3, 4        Jump to specific step");
            println!("  d / l             Toggle dark / light preview");
            println!("  q / Esc           Quit");
            return Ok(());
        }

        let wallpaper_path = arg.clone();
        println!("Extracting Material You palette from: {}", wallpaper_path);
        let palette = GeneratedPalette::from_image_path(&wallpaper_path)?;
        state.palette = Some(palette);
        state.wallpaper_path = Some(wallpaper_path);
    } else {
        // Fallback default theme for initial interactive run
        let default_seed = material_colors::color::Rgb::new(0x38, 0x5e, 0x72);
        let palette = GeneratedPalette::from_seed_color(default_seed);
        state.palette = Some(palette);
        state.wallpaper_path = Some("Default Balanced Theme".to_string());
    }

    // Launch interactive TUI
    run_tui(state)?;

    Ok(())
}
