//! Termonet binary entry point.

use std::env;
use std::path::PathBuf;
use termonet::cli::{CliArgs, run_cli_mode};
use termonet::config::AppConfig;
use termonet::core::GeneratedPalette;
use termonet::error::Result;
use termonet::tui::{AppState, run_tui};

enum CliAction {
    Exit,
    Run {
        cli_args: CliArgs,
        wallpaper_path: Option<String>,
        force_wizard: bool,
    },
}

fn print_help() {
    println!("Termonet v{}", env!("CARGO_PKG_VERSION"));
    println!("Usage: termonet [OPTIONS] [WALLPAPER_PATH]");
    println!("\nHeadless / Automation Options:");
    println!(
        "  -a, --apply              Headless mode: apply theme from active wallpaper and exit"
    );
    println!("  -f, --force              Force re-application ignoring cached wallpaper state");
    println!("  -c, --color <HEX>        Generate palette directly from seed HEX color (e.g. '#385e72')");
    println!("      --toggle-theme       Instantly toggle between dark and light themes and apply");
    println!("      --theme <MODE>       Set and apply theme mode ('dark' or 'light')");
    println!("\nGeneral Options:");
    println!("  -w, --wallpaper <PATH>   Path to wallpaper image (PNG, JPG, GIF, WebP)");
    println!("  -i, --wizard, --init     Launch first-run configuration wizard");
    println!("      --no-backup          Disable creation of backup copies (.bak, .original)");
    println!("      --backup             Enable creation of backup copies (.bak, .original)");
    println!(
        "      --reset-rules, --update-rules Force reset default rules and templates to latest"
    );
    println!("      --debug-wallpaper    Print active wallpaper managers diagnostic report");
    println!("  -h, --help               Print help information");
    println!("\nTUI Controls:");
    println!("  Left / Right (h / l)     Switch between steps / windows (1..4)");
    println!("  1, 2, 3, 4               Jump directly to specific step");
    println!("  Up / Down (k / j)        Navigate programs and hooks list");
    println!("  Space                    Toggle selected program or hook");
    println!("  Enter                    Apply theme (in Step 4) / Confirm selection (in Wizard)");
    println!(
        "  Tab / m                  Cycle active monitor wallpapers (in Step 1, multi-monitor)"
    );
    println!("  w / o                    Enter manual wallpaper path (in Step 1)");
    println!("  r                        Auto-detect wallpaper from active daemon (in Step 1)");
    println!("  s / F2                   Open settings wizard");
    println!("  q / Esc                  Quit");
}

fn handle_reset_rules() -> Result<()> {
    if let Ok(config_dir) = termonet::config::AppPaths::config_dir() {
        let cfg = AppConfig::load()?.unwrap_or_default();
        let updated =
            termonet::template::sync_default_assets(config_dir, true, cfg.create_backups)?;
        println!("Default rules and templates synchronized:");
        if updated.is_empty() {
            println!("  All rules and templates are already up to date.");
        } else {
            for item in updated {
                println!("  ✔ {}", item);
            }
        }
    }
    Ok(())
}

fn parse_cli_args(args: &[String]) -> Result<CliAction> {
    let mut wallpaper_path: Option<String> = None;
    let mut force_wizard = false;
    let mut cli_args = CliArgs::default();

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "-h" | "--help" => {
                print_help();
                return Ok(CliAction::Exit);
            }
            "-a" | "--apply" => {
                cli_args.apply = true;
            }
            "-f" | "--force" => {
                cli_args.force = true;
            }
            "-c" | "--color" => {
                if i + 1 < args.len() && !args[i + 1].starts_with('-') {
                    cli_args.color = Some(args[i + 1].clone());
                    i += 1;
                } else {
                    return Err(termonet::error::TermonetError::Config(
                        "Option '-c/--color' requires a valid HEX color argument (e.g. '#385e72')".into(),
                    ));
                }
            }
            "--toggle-theme" => {
                cli_args.toggle_theme = true;
                cli_args.apply = true;
            }
            "--theme" => {
                if i + 1 < args.len() && !args[i + 1].starts_with('-') {
                    cli_args.theme = Some(args[i + 1].clone());
                    cli_args.apply = true;
                    i += 1;
                } else {
                    return Err(termonet::error::TermonetError::Config(
                        "Option '--theme' requires a mode argument ('dark' or 'light')".into(),
                    ));
                }
            }
            "-w" | "--wallpaper" => {
                if i + 1 < args.len() && !args[i + 1].starts_with('-') {
                    wallpaper_path = Some(args[i + 1].clone());
                    i += 1;
                } else {
                    return Err(termonet::error::TermonetError::Config(
                        "Option '-w/--wallpaper' requires a valid file path argument".into(),
                    ));
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
                handle_reset_rules()?;
                return Ok(CliAction::Exit);
            }
            "--debug-wallpaper" => {
                let report = termonet::wallpaper::provider::run_all_diagnostics();
                println!("{report}");
                return Ok(CliAction::Exit);
            }
            arg if !arg.starts_with('-') => {
                wallpaper_path = Some(arg.to_string());
            }
            _ => {}
        }
        i += 1;
    }

    cli_args.wallpaper = wallpaper_path.clone();

    Ok(CliAction::Run {
        cli_args,
        wallpaper_path,
        force_wizard,
    })
}

fn resolve_tui_wallpaper(state: &mut AppState, cli_wallpaper: Option<&str>) -> Option<PathBuf> {
    if let Some(cli_p) = cli_wallpaper {
        let p = termonet::config::paths::AppPaths::expand_tilde(cli_p);
        if p.exists() {
            return Some(p);
        }
        eprintln!("Warning: Specified wallpaper not found: {}", cli_p);
        return None;
    }

    if state.config.wallpaper_backend != termonet::config::WallpaperBackend::Manual
        && let Ok(monitors) = termonet::wallpaper::get_wallpaper_monitors(
            &state.config.wallpaper_backend,
            state.config.manual_wallpaper_path.as_deref(),
        )
        && monitors.has_divergent_wallpapers()
    {
        let sorted = monitors.sorted_outputs();
        state.active_monitors = sorted;
        state.current_monitor_idx = if let Some(ref primary) = state.config.primary_monitor {
            state
                .active_monitors
                .iter()
                .position(|(name, _)| name == primary)
                .unwrap_or(0)
        } else {
            0
        };
    }

    let wp = if !state.active_monitors.is_empty() {
        state
            .active_monitors
            .get(state.current_monitor_idx)
            .map(|(_, p)| p.clone())
    } else {
        termonet::wallpaper::get_current_wallpaper(
            &state.config.wallpaper_backend,
            state.config.manual_wallpaper_path.as_deref(),
            state.config.primary_monitor.as_deref(),
        )
        .ok()
    };

    wp.or_else(|| {
        state
            .config
            .manual_wallpaper_path
            .as_ref()
            .map(|p| termonet::config::paths::AppPaths::expand_tilde(p))
            .filter(|p| p.exists())
    })
}

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    let (cli_args, wallpaper_path, force_wizard) = match parse_cli_args(&args)? {
        CliAction::Exit => return Ok(()),
        CliAction::Run {
            cli_args,
            wallpaper_path,
            force_wizard,
        } => (cli_args, wallpaper_path, force_wizard),
    };

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
    if cli_args.apply || cli_args.toggle_theme || cli_args.theme.is_some() {
        run_cli_mode(&cli_args)?;
        return Ok(());
    }

    // 2. Interactive TUI mode
    let mut state = if force_wizard || is_first_run {
        AppState::new_wizard(config)
    } else {
        AppState::new(config)
    };

    let resolved_wallpaper = resolve_tui_wallpaper(&mut state, wallpaper_path.as_deref());
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

    // Ensure terminal restores clean screen and normal cursor on unhandled panics
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let _ = crossterm::terminal::disable_raw_mode();
        let _ = crossterm::execute!(std::io::stdout(), crossterm::terminal::LeaveAlternateScreen);
        original_hook(panic_info);
    }));

    // Launch interactive TUI
    run_tui(state)?;

    Ok(())
}
