//! Termonet binary entry point.

use std::env;
use termonet::config::AppConfig;
use termonet::core::GeneratedPalette;
use termonet::error::Result;
use termonet::tui::{run_tui, AppState};

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();

    // Parse CLI arguments
    let mut wallpaper_path: Option<String> = None;
    let mut force_wizard = false;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "-h" | "--help" => {
                println!("Termonet v{}", env!("CARGO_PKG_VERSION"));
                println!("Usage: termonet [OPTIONS] [WALLPAPER_PATH]");
                println!("\nOptions:");
                println!("  -w, --wallpaper <PATH>   Path to wallpaper image (PNG, JPG, GIF, WebP)");
                println!("  -i, --wizard, --init     Launch first-run configuration wizard");
                println!("      --reset-rules        Force reset default rules and templates to latest");
                println!("  -h, --help               Print help information");
                println!("\nTUI Controls:");
                println!("  Tab / Shift+Tab          Switch between steps (1..4)");
                println!("  1, 2, 3, 4               Jump directly to specific step");
                println!("  Up / Down (k / j)        Navigate programs and hooks list");
                println!("  Space                    Toggle selected program or hook");
                println!("  w                        Change / enter wallpaper path (in Step 1)");
                println!("  s / F2                   Open settings wizard");
                println!("  q / Esc                  Quit");
                return Ok(());
            }
            "-w" | "--wallpaper" => {
                if i + 1 < args.len() {
                    wallpaper_path = Some(args[i + 1].clone());
                    i += 1;
                }
            }
            "-i" | "--wizard" | "--init" => {
                force_wizard = true;
            }
            "--reset-rules" | "--update-rules" => {
                if let Ok(config_dir) = termonet::config::AppPaths::config_dir() {
                    let updated = termonet::template::sync_default_assets(config_dir, true)?;
                    println!("Default rules and templates synchronized:");
                    if updated.is_empty() {
                        println!("  All rules and templates are already up to date.");
                    } else {
                        for item in updated {
                            println!("  ✔ {}", item);
                        }
                    }
                }
                return Ok(());
            }
            arg if !arg.starts_with('-') => {
                wallpaper_path = Some(arg.to_string());
            }
            _ => {}
        }
        i += 1;
    }

    // Synchronize and auto-migrate default templates and rules into ~/.config/termonet/
    if let Ok(config_dir) = termonet::config::AppPaths::config_dir() {
        let _ = termonet::template::sync_default_assets(config_dir, false);
    }

    // Load configuration or trigger initial setup wizard if missing
    let loaded_config = AppConfig::load()?;
    let is_first_run = loaded_config.is_none();
    let config = loaded_config.unwrap_or_default();

    let mut state = if force_wizard || is_first_run {
        AppState::new_wizard(config)
    } else {
        AppState::new(config)
    };

    // If no path was passed via CLI or file not found, try manual config path or seed
    let effective_wallpaper = wallpaper_path.or_else(|| state.config.manual_wallpaper_path.clone());

    if let Some(ref raw_path) = effective_wallpaper {
        let path = termonet::config::paths::AppPaths::expand_tilde(raw_path);
        if path.exists() {
            let palette = GeneratedPalette::from_image_path(&path)?;
            state.palette = Some(palette);
            state.wallpaper_path = Some(path.display().to_string());
        } else {
            eprintln!("Warning: Specified wallpaper not found: {}", raw_path);
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
