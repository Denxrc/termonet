//! Termonet binary entry point.

use std::env;
use termonet::cli::{run_cli_mode, CliArgs};
use termonet::config::AppConfig;
use termonet::core::GeneratedPalette;
use termonet::error::Result;
use termonet::tui::{run_tui, AppState};

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();

    // Parse CLI arguments
    let mut wallpaper_path: Option<String> = None;
    let mut force_wizard = false;
    let mut cli_args = CliArgs::default();

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "-h" | "--help" => {
                println!("Termonet v{}", env!("CARGO_PKG_VERSION"));
                println!("Usage: termonet [OPTIONS] [WALLPAPER_PATH]");
                println!("\nHeadless / Automation Options:");
                println!("  -a, --apply              Headless mode: apply theme from active wallpaper and exit");
                println!("  -f, --force              Force re-application ignoring cached wallpaper state");
                println!("\nGeneral Options:");
                println!("  -w, --wallpaper <PATH>   Path to wallpaper image (PNG, JPG, GIF, WebP)");
                println!("  -i, --wizard, --init     Launch first-run configuration wizard");
                println!("      --no-backup          Disable creation of backup copies (.bak, .original)");
                println!("      --backup             Enable creation of backup copies (.bak, .original)");
                println!("      --reset-rules        Force reset default rules and templates to latest");
                println!("      --debug-wallpaper    Print active wallpaper managers diagnostic report");
                println!("  -h, --help               Print help information");
                println!("\nTUI Controls:");
                println!("  Left / Right (h / l)     Switch between steps / windows (1..4)");
                println!("  1, 2, 3, 4               Jump directly to specific step");
                println!("  Up / Down (k / j)        Navigate programs and hooks list");
                println!("  Space                    Toggle selected program or hook");
                println!("  w                        Enter manual wallpaper path (in Step 1)");
                println!("  r                        Auto-detect wallpaper from active daemon (in Step 1)");
                println!("  s / F2                   Open settings wizard");
                println!("  q / Esc                  Quit");
                return Ok(());
            }
            "-a" | "--apply" => {
                cli_args.apply = true;
            }
            "-f" | "--force" => {
                cli_args.force = true;
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
            "--no-backup" => {
                cli_args.no_backup = true;
            }
            "--backup" => {
                cli_args.backup = true;
            }
            "--reset-rules" | "--update-rules" => {
                if let Ok(config_dir) = termonet::config::AppPaths::config_dir() {
                    let cfg = AppConfig::load()?.unwrap_or_default();
                    let updated = termonet::template::sync_default_assets(config_dir, true, cfg.create_backups)?;
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
            "--debug-wallpaper" => {
                let report = termonet::wallpaper::provider::run_all_diagnostics();
                println!("{report}");
                return Ok(());
            }
            arg if !arg.starts_with('-') => {
                wallpaper_path = Some(arg.to_string());
            }
            _ => {}
        }
        i += 1;
    }

    cli_args.wallpaper = wallpaper_path.clone();

    // Load configuration (auto-migrating outdated schemas if needed)
    let loaded_config = AppConfig::load()?;
    let is_first_run = loaded_config.is_none();
    let mut config = loaded_config.unwrap_or_default();
    if cli_args.no_backup {
        config.create_backups = false;
    } else if cli_args.backup {
        config.create_backups = true;
    }

    // Synchronize and auto-migrate default templates and rules into ~/.config/termonet/
    if let Ok(config_dir) = termonet::config::AppPaths::config_dir() {
        let _ = termonet::template::sync_default_assets(config_dir, false, config.create_backups);
    }

    // 1. Check if headless CLI mode was requested
    if cli_args.apply {
        run_cli_mode(&cli_args)?;
        return Ok(());
    }

    // 2. Interactive TUI mode
    let mut state = if force_wizard || is_first_run {
        AppState::new_wizard(config)
    } else {
        AppState::new(config)
    };

    // Resolve wallpaper for TUI:
    // Priority: CLI argument -> auto-detection via configured backend -> manual config path
    let resolved_wallpaper = if let Some(ref cli_p) = wallpaper_path {
        let p = termonet::config::paths::AppPaths::expand_tilde(cli_p);
        if p.exists() {
            Some(p)
        } else {
            eprintln!("Warning: Specified wallpaper not found: {}", cli_p);
            None
        }
    } else {
        termonet::wallpaper::get_current_wallpaper(
            &state.config.wallpaper_backend,
            state.config.manual_wallpaper_path.as_deref(),
            state.config.primary_monitor.as_deref(),
        )
        .ok()
        .or_else(|| {
            state
                .config
                .manual_wallpaper_path
                .as_ref()
                .map(|p| termonet::config::paths::AppPaths::expand_tilde(p))
                .filter(|p| p.exists())
        })
    };

    if let Some(path) = resolved_wallpaper
        && let Ok(palette) = GeneratedPalette::from_image_path(&path)
    {
        state.palette = Some(palette);
        state.wallpaper_path = Some(path.display().to_string());
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
