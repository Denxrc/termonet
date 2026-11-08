//! Termonet binary entry point.

use std::env;
use std::path::Path;
use termonet::config::AppConfig;
use termonet::core::GeneratedPalette;
use termonet::error::Result;
use termonet::tui::{run_tui, AppState};

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    let config = AppConfig::default();
    let mut state = AppState::new(config);

    // Parse CLI arguments
    let mut wallpaper_path: Option<String> = None;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "-h" | "--help" => {
                println!("Termonet v{}", env!("CARGO_PKG_VERSION"));
                println!("Usage: termonet [OPTIONS] [WALLPAPER_PATH]");
                println!("\nOptions:");
                println!("  -w, --wallpaper <PATH>   Path to wallpaper image (PNG, JPG, GIF, WebP)");
                println!("  -h, --help               Print help information");
                println!("\nTUI Controls:");
                println!("  Tab / Shift+Tab          Switch between steps (1..4)");
                println!("  1, 2, 3, 4               Jump directly to specific step");
                println!("  Up / Down (k / j)        Navigate programs and hooks list");
                println!("  Space                    Toggle selected program or hook");
                println!("  q / Esc                  Quit");
                return Ok(());
            }
            "-w" | "--wallpaper" => {
                if i + 1 < args.len() {
                    wallpaper_path = Some(args[i + 1].clone());
                    i += 1;
                }
            }
            arg if !arg.starts_with('-') => {
                wallpaper_path = Some(arg.to_string());
            }
            _ => {}
        }
        i += 1;
    }

    // If no path was passed via CLI or file not found, start with clean state
    if let Some(ref path) = wallpaper_path {
        if Path::new(path).exists() {
            let palette = GeneratedPalette::from_image_path(path)?;
            state.palette = Some(palette);
            state.wallpaper_path = Some(path.clone());
        } else {
            eprintln!("Warning: Specified wallpaper not found: {}", path);
        }
    }

    if state.palette.is_none() {
        let default_seed = material_colors::color::Rgb::new(0x38, 0x5e, 0x72);
        let palette = GeneratedPalette::from_seed_color(default_seed);
        state.palette = Some(palette);
        state.wallpaper_path = None;
    }

    // Launch interactive TUI
    run_tui(state)?;

    Ok(())
}
