//! Termonet binary entry point.

use std::env;
use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use termonet::cli::{CliArgs, run_cli_mode_with_context};
use termonet::config::AppConfig;
use termonet::core::GeneratedPalette;
use termonet::error::{Result, TermonetError};
use termonet::registry::{AutoUpdateOutcome, CatalogStatus, RegistryService, RegistryTransport};
use termonet::tui::{AppState, run_tui};

#[derive(Debug, PartialEq, Eq)]
enum CliAction {
    Exit,
    ListRules,
    SyncRepos,
    Install {
        specs: Vec<String>,
        yes: bool,
        force: bool,
        no_backup: bool,
        backup: bool,
    },
    RemoveRule {
        id: String,
        yes: bool,
    },
    UpdateRules {
        force: bool,
        no_backup: bool,
        backup: bool,
    },
    DebugWallpaper,
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
    println!("  -f, --force              Force re-application or overwrite locally edited rules");
    println!(
        "  -c, --color <HEX>        Generate palette directly from seed HEX color (e.g. '#385e72')"
    );
    println!("      --toggle-theme       Instantly toggle between dark and light themes and apply");
    println!("      --theme <MODE>       Set and apply theme mode ('dark' or 'light')");
    println!("\nRegistry & Template Management:");
    println!("      --list-rules         List installed, local custom, and repository programs");
    println!(
        "      --install <ID>       Install rule & template from repository (supports repo/id)"
    );
    println!("      --remove-rule <ID>   Uninstall a registry-managed rule & template");
    println!("      --update-rules       Sync repositories and update unmodified installed rules");
    println!("      --reset-rules        Force-reset tracked rules to upstream (creates .bak)");
    println!("      --sync-repos         Refresh cached repository indexes without installing");
    println!("  -y, --yes                Confirm install/remove prompts non-interactively");
    println!("\nGeneral Options:");
    println!("  -w, --wallpaper <PATH>   Path to wallpaper image (PNG, JPG, GIF, WebP)");
    println!("  -i, --wizard, --init     Launch first-run configuration wizard");
    println!("      --no-backup          Disable creation of backup copies (.bak, .original)");
    println!("      --backup             Enable creation of backup copies (.bak, .original)");
    println!("      --debug-wallpaper    Print active wallpaper managers diagnostic report");
    println!("  -V, --version            Print version information");
    println!("  -h, --help               Print help information");
    println!("\nTUI Controls:");
    println!("  Left / Right (h / l)     Switch between steps / windows (1..4)");
    println!("  1, 2, 3, 4               Jump directly to specific step");
    println!("  Up / Down (k / j)        Navigate programs and hooks list");
    println!("  Space                    Toggle program/hook or install available [↓] program");
    println!("  u                        Sync repositories and update installed rules (in Step 2)");
    println!("  d / Delete               Remove downloaded registry program files (in Step 2)");
    println!("  Enter                    Apply theme (in Step 4) / Toggle or save (in Wizard)");
    println!(
        "  Tab / m                  Cycle active monitor wallpapers (in Step 1, multi-monitor)"
    );
    println!("  w / o                    Enter manual wallpaper path (in Step 1)");
    println!("  r                        Auto-detect wallpaper from active daemon (in Step 1)");
    println!("  s / F2                   Open settings wizard");
    println!("  q / Esc                  Quit");
}

#[must_use]
fn should_run_weekly_auto_update(cli_args: &CliArgs) -> bool {
    cli_args.apply && !cli_args.toggle_theme && cli_args.theme.is_none()
}

fn current_epoch_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn apply_backup_flags(cfg: &mut AppConfig, no_backup: bool, backup: bool) {
    if no_backup {
        cfg.create_backups = false;
    } else if backup {
        cfg.create_backups = true;
    }
}

fn handle_list_rules() -> Result<()> {
    let service = RegistryService::from_system_paths()?;
    service.ensure_default_repo_config()?;
    let _ = service.adopt_legacy_v1_5_assets();

    if let Err(err) = service.sync_indexes_if_empty() {
        eprintln!(
            "Warning: unable to fetch repository index ({err}); showing local/cached entries."
        );
    }

    let (catalog, warnings) = service.catalog()?;
    for warn in warnings {
        eprintln!("Warning: {warn}");
    }

    if catalog.is_empty() {
        println!("No programs found locally or in repository caches.");
        return Ok(());
    }

    for entry in catalog {
        let badge = match &entry.status {
            CatalogStatus::Installed {
                repo,
                update_available: Some((rv, tv)),
                locally_modified: true,
            } => format!("[installed:{repo}] [update v{rv}/t{tv}] [locally modified]"),
            CatalogStatus::Installed {
                repo,
                update_available: Some((rv, tv)),
                locally_modified: false,
            } => format!("[installed:{repo}] [update v{rv}/t{tv}]"),
            CatalogStatus::Installed {
                repo,
                update_available: None,
                locally_modified: true,
            } => format!("[installed:{repo}] [locally modified]"),
            CatalogStatus::Installed {
                repo,
                update_available: None,
                locally_modified: false,
            } => format!("[installed:{repo}]"),
            CatalogStatus::Available {
                repo,
                compatible: true,
            } => format!("[available:{repo}]"),
            CatalogStatus::Available {
                repo,
                compatible: false,
            } => {
                let req = entry.min_termonet_version.as_deref().unwrap_or("newer");
                format!("[available:{repo}] [requires termonet >= {req}]")
            }
            CatalogStatus::LocalCustom => "[local]".to_string(),
        };

        let desc = if entry.description.is_empty() {
            String::new()
        } else {
            format!(" - {}", entry.description)
        };
        println!("  {:<16} {:<20} {}{}", entry.id, entry.name, badge, desc);
    }

    Ok(())
}

fn handle_sync_repos() -> Result<()> {
    let service = RegistryService::from_system_paths()?;
    service.ensure_default_repo_config()?;
    let warnings = service.sync_indexes()?;
    println!("Repository indexes synchronized.");
    for warn in warnings {
        eprintln!("Warning: {warn}");
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn execute_install_with_io<T: RegistryTransport, R: BufRead, W: Write>(
    service: &RegistryService<T>,
    specs: &[String],
    yes: bool,
    force: bool,
    config: &mut AppConfig,
    config_file: &Path,
    reader: &mut R,
    writer: &mut W,
) -> Result<bool> {
    let prepared = service.prepare_install(specs, force)?;
    if prepared.is_empty() {
        return Ok(false);
    }

    writeln!(
        writer,
        "The following template package(s) will be installed:"
    )?;
    for item in &prepared {
        let p = &item.preview;
        writeln!(
            writer,
            "  • {} ({}) from [{}] (rule v{}, template v{})",
            p.id, p.name, p.repo, p.rule_version, p.template_version
        )?;
        writeln!(writer, "      output_path:   {}", p.output_path)?;
        if let Some(ref target) = p.target_config {
            writeln!(writer, "      target_config: {target}")?;
        }
        if let Some(ref cmd) = p.reload_cmd {
            writeln!(writer, "      WARNING reload_cmd (runs in shell): {cmd}")?;
        }
        if p.has_local_conflict {
            writeln!(
                writer,
                "      WARNING: local edits exist and will be overwritten (--force)"
            )?;
        }
    }

    if !yes {
        write!(writer, "Proceed with installation? [y/N]: ")?;
        writer.flush()?;
        let mut answer = String::new();
        reader.read_line(&mut answer)?;
        let confirmed = matches!(answer.trim().to_ascii_lowercase().as_str(), "y" | "yes");
        if !confirmed {
            writeln!(writer, "Installation cancelled.")?;
            return Ok(false);
        }
    }

    let installed_ids = service.commit_install(&prepared, force, config.create_backups)?;
    for id in &installed_ids {
        if !config
            .enabled_programs
            .iter()
            .any(|p| p.eq_ignore_ascii_case(id))
        {
            config.enabled_programs.push(id.clone());
        }
    }
    config.save_to_path(config_file)?;

    for id in &installed_ids {
        writeln!(writer, "  ✔ Installed and enabled: {id}")?;
    }
    Ok(true)
}

fn execute_remove_rule_with_io<T: RegistryTransport, R: BufRead, W: Write>(
    service: &RegistryService<T>,
    id: &str,
    yes: bool,
    config: &mut AppConfig,
    config_file: &Path,
    reader: &mut R,
    writer: &mut W,
) -> Result<bool> {
    termonet::registry::validate_program_id(id)?;

    if !yes {
        write!(
            writer,
            "Remove registry rule & template files for '{id}'? [y/N]: "
        )?;
        writer.flush()?;
        let mut answer = String::new();
        reader.read_line(&mut answer)?;
        let confirmed = matches!(answer.trim().to_ascii_lowercase().as_str(), "y" | "yes");
        if !confirmed {
            writeln!(writer, "Removal cancelled.")?;
            return Ok(false);
        }
    }

    service.remove_program(id)?;
    config
        .enabled_programs
        .retain(|p| !p.eq_ignore_ascii_case(id));
    config.save_to_path(config_file)?;
    writeln!(writer, "  ✔ Removed: {id}")?;
    Ok(true)
}

fn handle_update_rules(force: bool, no_backup: bool, backup: bool) -> Result<()> {
    let mut cfg = AppConfig::load()?.unwrap_or_default();
    apply_backup_flags(&mut cfg, no_backup, backup);

    let service = RegistryService::from_system_paths()?;
    service.ensure_default_repo_config()?;
    let _ = service.adopt_legacy_v1_5_assets();

    let report = service.update_installed(force, cfg.create_backups)?;
    println!("Registry rules and templates synchronized:");
    for item in &report.updated {
        println!("  ✔ Updated: {item}");
    }
    for item in &report.skipped_modified {
        eprintln!(
            "  ! Skipped locally modified '{item}' (use --update-rules --force or --reset-rules to overwrite with .bak)"
        );
    }
    for item in &report.skipped_incompatible {
        eprintln!("  ! Skipped '{item}': requires a newer Termonet version");
    }
    for err in &report.errors {
        eprintln!("  ! Warning: {err}");
    }
    if report.updated.is_empty()
        && report.skipped_modified.is_empty()
        && report.skipped_incompatible.is_empty()
    {
        println!("  All installed rules and templates are already up to date.");
    }
    Ok(())
}

fn parse_cli_args(args: &[String]) -> Result<CliAction> {
    let mut wallpaper_path: Option<String> = None;
    let mut force_wizard = false;
    let mut list_rules = false;
    let mut sync_repos = false;
    let mut install_specs: Vec<String> = Vec::new();
    let mut remove_rule: Option<String> = None;
    let mut update_rules = false;
    let mut reset_rules_alias = false;
    let mut yes = false;
    let mut debug_wallpaper = false;
    let mut cli_args = CliArgs::default();

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "-V" | "--version" => {
                println!("termonet {}", env!("CARGO_PKG_VERSION"));
                return Ok(CliAction::Exit);
            }
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
            "-y" | "--yes" => {
                yes = true;
            }
            "-c" | "--color" => {
                if i + 1 < args.len() && !args[i + 1].starts_with('-') {
                    cli_args.color = Some(args[i + 1].clone());
                    i += 1;
                } else {
                    return Err(TermonetError::Config(
                        "Option '-c/--color' requires a valid HEX color argument (e.g. '#385e72')"
                            .into(),
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
                    return Err(TermonetError::Config(
                        "Option '--theme' requires a mode argument ('dark' or 'light')".into(),
                    ));
                }
            }
            "-w" | "--wallpaper" => {
                if i + 1 < args.len() && !args[i + 1].starts_with('-') {
                    wallpaper_path = Some(args[i + 1].clone());
                    i += 1;
                } else {
                    return Err(TermonetError::Config(
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
            "--list-rules" => {
                list_rules = true;
            }
            "--sync-repos" => {
                sync_repos = true;
            }
            "--install" => {
                if i + 1 < args.len() && !args[i + 1].starts_with('-') {
                    let mut parsed_any = false;
                    for part in args[i + 1].split(',') {
                        let trimmed = part.trim();
                        if !trimmed.is_empty() {
                            install_specs.push(trimmed.to_string());
                            parsed_any = true;
                        }
                    }
                    if !parsed_any {
                        return Err(TermonetError::Config(
                            "Option '--install' requires at least one non-empty program ID".into(),
                        ));
                    }
                    i += 1;
                } else {
                    return Err(TermonetError::Config(
                        "Option '--install' requires a program ID (e.g. 'kitty' or 'official/kitty')"
                            .into(),
                    ));
                }
            }
            "--remove-rule" => {
                if i + 1 < args.len() && !args[i + 1].starts_with('-') {
                    remove_rule = Some(args[i + 1].trim().to_string());
                    i += 1;
                } else {
                    return Err(TermonetError::Config(
                        "Option '--remove-rule' requires a program ID".into(),
                    ));
                }
            }
            "--update-rules" => {
                update_rules = true;
            }
            "--reset-rules" => {
                update_rules = true;
                reset_rules_alias = true;
            }
            "--debug-wallpaper" => {
                debug_wallpaper = true;
            }
            arg if !arg.starts_with('-') => {
                wallpaper_path = Some(arg.to_string());
            }
            unknown => {
                return Err(TermonetError::Config(format!(
                    "Unknown option: '{unknown}'. Run 'termonet --help' for usage."
                )));
            }
        }
        i += 1;
    }

    let registry_action_count = usize::from(list_rules)
        + usize::from(sync_repos)
        + usize::from(!install_specs.is_empty())
        + usize::from(remove_rule.is_some())
        + usize::from(update_rules);

    if registry_action_count > 1 {
        return Err(TermonetError::Config(
            "Registry options (--list-rules, --sync-repos, --install, --remove-rule, --update-rules/--reset-rules) cannot be combined with each other"
                .into(),
        ));
    }

    let has_run_mode_flags = cli_args.apply
        || cli_args.toggle_theme
        || cli_args.theme.is_some()
        || cli_args.color.is_some()
        || wallpaper_path.is_some()
        || force_wizard
        || debug_wallpaper;

    if registry_action_count > 0 && has_run_mode_flags {
        return Err(TermonetError::Config(
            "Registry management options cannot be combined with --apply, --theme, --toggle-theme, --wallpaper, --wizard, or --debug-wallpaper"
                .into(),
        ));
    }

    if list_rules {
        return Ok(CliAction::ListRules);
    }
    if sync_repos {
        return Ok(CliAction::SyncRepos);
    }
    if !install_specs.is_empty() {
        return Ok(CliAction::Install {
            specs: install_specs,
            yes,
            force: cli_args.force,
            no_backup: cli_args.no_backup,
            backup: cli_args.backup,
        });
    }
    if let Some(id) = remove_rule {
        return Ok(CliAction::RemoveRule { id, yes });
    }
    if update_rules {
        return Ok(CliAction::UpdateRules {
            force: cli_args.force || reset_rules_alias,
            no_backup: cli_args.no_backup,
            backup: cli_args.backup || (reset_rules_alias && !cli_args.no_backup),
        });
    }
    if debug_wallpaper {
        return Ok(CliAction::DebugWallpaper);
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
        state.status_message = Some(format!("Specified wallpaper not found: {}", cli_p));
        return None;
    }

    let wp = if state.config.wallpaper_backend != termonet::config::WallpaperBackend::Manual {
        if let Ok(monitors) = termonet::wallpaper::get_wallpaper_monitors(
            &state.config.wallpaper_backend,
            state.config.manual_wallpaper_path.as_deref(),
        ) {
            if monitors.has_divergent_wallpapers() {
                let sorted = monitors.sorted_outputs();
                state.active_monitors = sorted;
                state.current_monitor_idx = if let Some(ref primary) = state.config.primary_monitor
                {
                    state
                        .active_monitors
                        .iter()
                        .position(|(name, _)| name == primary)
                        .unwrap_or(0)
                } else {
                    0
                };
                state
                    .active_monitors
                    .get(state.current_monitor_idx)
                    .map(|(_, p)| p.clone())
            } else {
                monitors.resolve_primary(state.config.primary_monitor.as_deref())
            }
        } else {
            None
        }
    } else {
        None
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
        CliAction::ListRules => {
            handle_list_rules()?;
            return Ok(());
        }
        CliAction::SyncRepos => {
            handle_sync_repos()?;
            return Ok(());
        }
        CliAction::Install {
            specs,
            yes,
            force,
            no_backup,
            backup,
        } => {
            let service = RegistryService::from_system_paths()?;
            service.ensure_default_repo_config()?;
            let _ = service.adopt_legacy_v1_5_assets();
            let mut cfg = AppConfig::load()?.unwrap_or_default();
            apply_backup_flags(&mut cfg, no_backup, backup);
            let cfg_file = termonet::config::AppPaths::config_file()?;
            let stdin = std::io::stdin();
            let stdout = std::io::stdout();
            let mut reader = stdin.lock();
            let mut writer = stdout.lock();
            execute_install_with_io(
                &service,
                &specs,
                yes,
                force,
                &mut cfg,
                &cfg_file,
                &mut reader,
                &mut writer,
            )?;
            return Ok(());
        }
        CliAction::RemoveRule { id, yes } => {
            let service = RegistryService::from_system_paths()?;
            let mut cfg = AppConfig::load()?.unwrap_or_default();
            let cfg_file = termonet::config::AppPaths::config_file()?;
            let stdin = std::io::stdin();
            let stdout = std::io::stdout();
            let mut reader = stdin.lock();
            let mut writer = stdout.lock();
            execute_remove_rule_with_io(
                &service,
                &id,
                yes,
                &mut cfg,
                &cfg_file,
                &mut reader,
                &mut writer,
            )?;
            return Ok(());
        }
        CliAction::UpdateRules {
            force,
            no_backup,
            backup,
        } => {
            handle_update_rules(force, no_backup, backup)?;
            return Ok(());
        }
        CliAction::DebugWallpaper => {
            let report = termonet::wallpaper::provider::run_all_diagnostics();
            println!("{report}");
            return Ok(());
        }
        CliAction::Run {
            cli_args,
            wallpaper_path,
            force_wizard,
        } => (cli_args, wallpaper_path, force_wizard),
    };

    let loaded_config = AppConfig::load()?;
    let is_first_run = loaded_config.is_none();
    let mut config = loaded_config.unwrap_or_default();
    apply_backup_flags(&mut config, cli_args.no_backup, cli_args.backup);

    // 1. Fast-path check for headless CLI mode (skip registry network checks on --toggle-theme / --theme)
    if cli_args.apply || cli_args.toggle_theme || cli_args.theme.is_some() {
        if should_run_weekly_auto_update(&cli_args)
            && let Ok(service) = RegistryService::from_system_paths()
        {
            let _ = service.ensure_default_repo_config();
            let _ = service.adopt_legacy_v1_5_assets();
            match service.maybe_auto_update(&config, current_epoch_secs()) {
                AutoUpdateOutcome::Completed(report) if !report.updated.is_empty() => {
                    println!(
                        "Weekly registry auto-update updated: {}",
                        report.updated.join(", ")
                    );
                }
                AutoUpdateOutcome::FailedNonFatal(warn) => {
                    eprintln!("Warning: weekly registry check failed ({warn})");
                }
                _ => {}
            }
        }
        run_cli_mode_with_context(&cli_args, config, None, None, None)?;
        return Ok(());
    }

    // 2. Interactive TUI mode
    let mut state = if force_wizard || is_first_run {
        AppState::new_wizard(config)
    } else {
        AppState::new(config)
    };

    if let Ok(service) = RegistryService::from_system_paths() {
        let _ = service.ensure_default_repo_config();
        let _ = service.adopt_legacy_v1_5_assets();
        state.set_registry(std::sync::Arc::new(service));
        state.run_startup_auto_update(current_epoch_secs());
        state.refresh_catalog();
    }

    let resolved_wallpaper = if cli_args.color.is_none() {
        resolve_tui_wallpaper(&mut state, wallpaper_path.as_deref())
    } else {
        None
    };

    if let Some(ref hex_str) = cli_args.color {
        let color = termonet::core::RgbColor::from_hex(hex_str)?;
        let palette = GeneratedPalette::from_seed_color(color.into());
        state.palette = Some(palette);
        state.custom_seed_color = Some(color);
        state.wallpaper_path = None;
    } else if let Some(path) = resolved_wallpaper {
        match GeneratedPalette::from_image_path(&path) {
            Ok(palette) => {
                state.palette = Some(palette);
                state.wallpaper_path = Some(path.display().to_string());
                state.custom_seed_color = None;
            }
            Err(e) => {
                state.status_message = Some(format!(
                    "Failed to load wallpaper {}: {}",
                    path.display(),
                    e
                ));
            }
        }
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
        let _ = crossterm::execute!(
            std::io::stdout(),
            crossterm::terminal::LeaveAlternateScreen,
            crossterm::cursor::Show
        );
        original_hook(panic_info);
    }));

    // Launch interactive TUI
    run_tui(state)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_cli_args_rejects_unknown_flags() {
        let args = vec!["termonet".to_string(), "--unknown-flag".to_string()];
        assert!(parse_cli_args(&args).is_err());

        let typo = vec![
            "termonet".to_string(),
            "--theem".to_string(),
            "dark".to_string(),
        ];
        assert!(parse_cli_args(&typo).is_err());
    }

    #[test]
    fn test_parse_cli_args_defers_reset_rules_with_backup_flags() {
        let args = vec![
            "termonet".to_string(),
            "--reset-rules".to_string(),
            "--no-backup".to_string(),
        ];
        assert_eq!(
            parse_cli_args(&args).unwrap(),
            CliAction::UpdateRules {
                force: true,
                no_backup: true,
                backup: false
            }
        );

        let args_help = vec![
            "termonet".to_string(),
            "--reset-rules".to_string(),
            "--help".to_string(),
        ];
        assert_eq!(parse_cli_args(&args_help).unwrap(), CliAction::Exit);

        for flag in ["-V", "--version"] {
            let args_ver = vec!["termonet".to_string(), flag.to_string()];
            assert_eq!(parse_cli_args(&args_ver).unwrap(), CliAction::Exit);
        }
    }

    #[test]
    fn test_parse_cli_args_registry_commands() {
        assert_eq!(
            parse_cli_args(&["termonet".into(), "--list-rules".into()]).unwrap(),
            CliAction::ListRules
        );
        assert_eq!(
            parse_cli_args(&["termonet".into(), "--sync-repos".into()]).unwrap(),
            CliAction::SyncRepos
        );
        assert_eq!(
            parse_cli_args(&[
                "termonet".into(),
                "--install".into(),
                "official/waybar,kitty".into(),
                "--install".into(),
                "rofi".into(),
                "-y".into(),
            ])
            .unwrap(),
            CliAction::Install {
                specs: vec!["official/waybar".into(), "kitty".into(), "rofi".into()],
                yes: true,
                force: false,
                no_backup: false,
                backup: false,
            }
        );
        assert_eq!(
            parse_cli_args(&[
                "termonet".into(),
                "--remove-rule".into(),
                "kitty".into(),
                "--yes".into()
            ])
            .unwrap(),
            CliAction::RemoveRule {
                id: "kitty".into(),
                yes: true,
            }
        );
        assert_eq!(
            parse_cli_args(&["termonet".into(), "--update-rules".into()]).unwrap(),
            CliAction::UpdateRules {
                force: false,
                no_backup: false,
                backup: false,
            }
        );
        assert_eq!(
            parse_cli_args(&["termonet".into(), "--reset-rules".into()]).unwrap(),
            CliAction::UpdateRules {
                force: true,
                no_backup: false,
                backup: true,
            }
        );
    }

    #[test]
    fn test_parse_cli_args_rejects_combining_registry_and_apply() {
        for bad in [
            vec!["termonet", "--install", "kitty", "--apply"],
            vec!["termonet", "--update-rules", "--toggle-theme"],
            vec!["termonet", "--remove-rule", "kitty", "--theme", "dark"],
            vec!["termonet", "--install", "kitty", "--remove-rule", "rofi"],
            vec!["termonet", "--list-rules", "--apply"],
        ] {
            let owned: Vec<String> = bad.into_iter().map(str::to_string).collect();
            assert!(
                parse_cli_args(&owned).is_err(),
                "Expected conflict error for {owned:?}"
            );
        }
    }

    #[test]
    fn test_should_run_weekly_auto_update_excludes_toggle_and_theme() {
        assert!(should_run_weekly_auto_update(&CliArgs {
            apply: true,
            ..Default::default()
        }));
        assert!(!should_run_weekly_auto_update(&CliArgs {
            apply: true,
            toggle_theme: true,
            ..Default::default()
        }));
        assert!(!should_run_weekly_auto_update(&CliArgs {
            apply: true,
            theme: Some("dark".into()),
            ..Default::default()
        }));
        assert!(!should_run_weekly_auto_update(&CliArgs::default()));
    }

    #[test]
    fn test_cli_install_aborts_without_modifying_config_on_decline_or_error() {
        let temp_dir =
            std::env::temp_dir().join(format!("termonet_test_cli_install_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&temp_dir);
        let paths = termonet::registry::RegistryPaths {
            repos_dir: temp_dir.join("repos"),
            rules_dir: temp_dir.join("rules"),
            templates_dir: temp_dir.join("templates"),
            cache_dir: temp_dir.join("cache"),
            installed_file: temp_dir.join("installed.toml"),
            state_file: temp_dir.join("state.toml"),
        };

        struct EmptyMock;
        impl termonet::registry::RegistryTransport for EmptyMock {
            fn fetch_text(&self, url: &str, _max_bytes: usize) -> Result<String> {
                Err(termonet::TermonetError::Registry(format!(
                    "offline mock: {url}"
                )))
            }
        }

        let svc = termonet::registry::RegistryService::with_paths_and_transport(
            paths,
            EmptyMock,
            "1.6.0".into(),
        );
        svc.ensure_default_repo_config().unwrap();

        let mut cfg = AppConfig::default();
        let cfg_file = temp_dir.join("config.toml");
        cfg.save_to_path(&cfg_file).unwrap();

        let mut input = std::io::Cursor::new(b"n\n");
        let mut output = Vec::new();
        let res = execute_install_with_io(
            &svc,
            &["kitty".into()],
            false,
            false,
            &mut cfg,
            &cfg_file,
            &mut input,
            &mut output,
        );
        assert!(res.is_err());
        let saved = AppConfig::load_from_path(&cfg_file).unwrap();
        assert!(saved.enabled_programs.is_empty());

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_cli_install_preview_warning_decline_accept_and_remove() {
        let temp_dir = std::env::temp_dir().join(format!(
            "termonet_test_cli_install_flow_{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&temp_dir);
        let paths = termonet::registry::RegistryPaths {
            repos_dir: temp_dir.join("repos"),
            rules_dir: temp_dir.join("rules"),
            templates_dir: temp_dir.join("templates"),
            cache_dir: temp_dir.join("cache"),
            installed_file: temp_dir.join("installed.toml"),
            state_file: temp_dir.join("state.toml"),
        };

        let rule_txt = "version = 1\nname = \"Foot\"\ntemplate = \"foot.tmpl\"\noutput_path = \"~/.config/foot/colors.ini\"\ntarget_config = \"~/.config/foot/foot.ini\"\nreload_cmd = \"killall -SIGUSR1 foot\"\n";
        let tmpl_txt = "# termonet-version: 1\nfg = {{ active.primary.hex }}\n";
        let r_sha = termonet::registry::sha256_hex(rule_txt.as_bytes());
        let t_sha = termonet::registry::sha256_hex(tmpl_txt.as_bytes());
        let manifest = format!(
            "schema_version = 1\n\n[[programs]]\nid = \"foot\"\nname = \"Foot\"\ncategory = \"terminal\"\ndescription = \"Foot\"\nrule = \"rules/foot.toml\"\ntemplate = \"templates/foot.tmpl\"\nrule_version = 1\ntemplate_version = 1\nmin_termonet_version = \"1.6.0\"\nrule_sha256 = \"{r_sha}\"\ntemplate_sha256 = \"{t_sha}\"\n"
        );

        struct ValidMock {
            manifest: String,
            rule: String,
            tmpl: String,
        }
        impl termonet::registry::RegistryTransport for ValidMock {
            fn fetch_text(&self, url: &str, _max_bytes: usize) -> Result<String> {
                if url.ends_with("/registry.toml") {
                    Ok(self.manifest.clone())
                } else if url.ends_with("/rules/foot.toml") {
                    Ok(self.rule.clone())
                } else if url.ends_with("/templates/foot.tmpl") {
                    Ok(self.tmpl.clone())
                } else {
                    Err(TermonetError::Registry(format!("unexpected {url}")))
                }
            }
        }

        let svc = termonet::registry::RegistryService::with_paths_and_transport(
            paths.clone(),
            ValidMock {
                manifest,
                rule: rule_txt.into(),
                tmpl: tmpl_txt.into(),
            },
            "1.6.0".into(),
        );
        svc.ensure_default_repo_config().unwrap();

        let mut cfg = AppConfig::default();
        let cfg_file = temp_dir.join("config.toml");
        cfg.save_to_path(&cfg_file).unwrap();

        // 1. Decline prompt ('n') -> warns about reload_cmd, does NOT write files or config
        let mut input_no = std::io::Cursor::new(b"n\n");
        let mut output_no = Vec::new();
        let ok_no = execute_install_with_io(
            &svc,
            &["foot".into()],
            false,
            false,
            &mut cfg,
            &cfg_file,
            &mut input_no,
            &mut output_no,
        )
        .unwrap();
        assert!(!ok_no);
        let out_str = String::from_utf8(output_no).unwrap();
        assert!(out_str.contains("WARNING reload_cmd"));
        assert!(out_str.contains("killall -SIGUSR1 foot"));
        assert!(!paths.rules_dir.join("foot.toml").exists());
        assert!(
            AppConfig::load_from_path(&cfg_file)
                .unwrap()
                .enabled_programs
                .is_empty()
        );

        // 2. Accept prompt ('y') -> installs files and enables program
        let mut input_yes = std::io::Cursor::new(b"y\n");
        let mut output_yes = Vec::new();
        let ok_yes = execute_install_with_io(
            &svc,
            &["foot".into()],
            false,
            false,
            &mut cfg,
            &cfg_file,
            &mut input_yes,
            &mut output_yes,
        )
        .unwrap();
        assert!(ok_yes);
        assert!(paths.rules_dir.join("foot.toml").exists());
        assert_eq!(
            AppConfig::load_from_path(&cfg_file)
                .unwrap()
                .enabled_programs,
            vec!["foot".to_string()]
        );

        // 3. Remove rule -> deletes files and removes from enabled_programs
        let mut input_rm = std::io::Cursor::new(b"");
        let mut output_rm = Vec::new();
        let ok_rm = execute_remove_rule_with_io(
            &svc,
            "foot",
            true,
            &mut cfg,
            &cfg_file,
            &mut input_rm,
            &mut output_rm,
        )
        .unwrap();
        assert!(ok_rm);
        assert!(!paths.rules_dir.join("foot.toml").exists());
        assert!(
            AppConfig::load_from_path(&cfg_file)
                .unwrap()
                .enabled_programs
                .is_empty()
        );

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_stress_cli_invalid_flags_and_empty_install() {
        for bad in [
            vec!["termonet", "--install", "", "--apply"],
            vec!["termonet", "--install", ",,,", "--apply"],
            vec!["termonet", "--install", ",,,"],
            vec!["termonet", "--list-rules", "--sync-repos"],
            vec!["termonet", "--install", "kitty", "--update-rules"],
            vec!["termonet", "--reset-rules", "--remove-rule", "kitty"],
            vec!["termonet", "--install", "kitty", "--theme", "dark"],
        ] {
            let owned: Vec<String> = bad.iter().map(|s| s.to_string()).collect();
            assert!(
                parse_cli_args(&owned).is_err(),
                "Expected error for args {bad:?}"
            );
        }

        // Invalid --theme / --color values parsed cleanly must fail in run_cli_mode_with_context
        for runtime_bad in [
            vec!["termonet", "--theme", "neon"],
            vec!["termonet", "--color", "nothex", "--apply"],
        ] {
            let owned: Vec<String> = runtime_bad.iter().map(|s| s.to_string()).collect();
            let CliAction::Run { cli_args, .. } = parse_cli_args(&owned).unwrap() else {
                panic!("Expected CliAction::Run for {runtime_bad:?}");
            };
            assert!(
                termonet::cli::run_cli_mode_with_context(
                    &cli_args,
                    AppConfig::default(),
                    Some(Vec::new()),
                    None,
                    None,
                )
                .is_err(),
                "Expected runtime error for {runtime_bad:?}"
            );
        }

        // Invalid program ID in --remove-rule must fail before modifying config or filesystem
        let temp_dir =
            std::env::temp_dir().join(format!("termonet_cli_stress_rm_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&temp_dir);
        let paths = termonet::registry::RegistryPaths {
            repos_dir: temp_dir.join("repos"),
            rules_dir: temp_dir.join("rules"),
            templates_dir: temp_dir.join("templates"),
            cache_dir: temp_dir.join("cache"),
            installed_file: temp_dir.join("installed.toml"),
            state_file: temp_dir.join("state.toml"),
        };
        struct EmptyMock;
        impl termonet::registry::RegistryTransport for EmptyMock {
            fn fetch_text(&self, url: &str, _max_bytes: usize) -> Result<String> {
                Err(termonet::TermonetError::Registry(format!(
                    "offline mock: {url}"
                )))
            }
        }
        let svc = RegistryService::with_paths_and_transport(paths, EmptyMock, "1.6.0".into());
        let mut cfg = AppConfig::default();
        let cfg_file = temp_dir.join("config.toml");
        let mut input = std::io::Cursor::new(b"");
        let mut output = Vec::new();
        assert!(
            execute_remove_rule_with_io(
                &svc,
                "../etc/passwd",
                true,
                &mut cfg,
                &cfg_file,
                &mut input,
                &mut output,
            )
            .is_err()
        );
        let _ = std::fs::remove_dir_all(&temp_dir);

        assert_eq!(
            parse_cli_args(&["termonet".into(), "--help".into()]).unwrap(),
            CliAction::Exit
        );
        assert_eq!(
            parse_cli_args(&["termonet".into(), "--version".into()]).unwrap(),
            CliAction::Exit
        );
    }
}
