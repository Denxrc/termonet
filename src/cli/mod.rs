//! Headless CLI trigger mode execution.

pub use crate::config::AppStateCache;
use std::path::PathBuf;

/// Command line execution arguments.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CliArgs {
    pub apply: bool,
    pub force: bool,
    pub wallpaper: Option<String>,
    pub color: Option<String>,
    pub toggle_theme: bool,
    pub theme: Option<String>,
    pub no_backup: bool,
    pub backup: bool,
}

/// Executes headless apply without TUI if `--apply`, `--toggle-theme`, or `--theme` is passed.
/// Returns `Ok(true)` if headless mode was executed, `Ok(false)` if interactive TUI should proceed.
pub fn run_cli_mode(args: &CliArgs) -> crate::error::Result<bool> {
    let config = crate::config::AppConfig::load()?.unwrap_or_default();
    run_cli_mode_with_context(args, config, None, None, None)
}

/// Headless execution with configurable context (useful for testing and embedding).
pub fn run_cli_mode_with_context(
    args: &CliArgs,
    mut config: crate::config::AppConfig,
    custom_rules: Option<Vec<crate::template::ProgramRule>>,
    custom_state_file: Option<&std::path::Path>,
    custom_config_file: Option<&std::path::Path>,
) -> crate::error::Result<bool> {
    if !args.apply && !args.toggle_theme && args.theme.is_none() {
        return Ok(false);
    }

    if args.no_backup {
        config.create_backups = false;
    } else if args.backup {
        config.create_backups = true;
    }

    let cache = if let Some(state_path) = custom_state_file {
        AppStateCache::load_from_path(state_path).unwrap_or_default()
    } else {
        AppStateCache::load().unwrap_or_default()
    };

    let config_fp = if custom_rules.is_none() {
        Some(AppStateCache::compute_config_fingerprint(
            &config.enabled_programs,
            crate::config::AppPaths::rules_dir().ok().as_deref(),
            crate::config::AppPaths::templates_dir().ok().as_deref(),
        ))
    } else {
        Some(AppStateCache::compute_config_fingerprint(
            &config.enabled_programs,
            None,
            None,
        ))
    };

    // 1. Determine target theme mode (is_dark: bool) without persisting config until apply succeeds
    let (is_dark, theme_modified) = if let Some(ref theme_str) = args.theme {
        let dark = match theme_str.to_lowercase().as_str() {
            "dark" => true,
            "light" => false,
            other => {
                return Err(crate::error::TermonetError::Config(format!(
                    "Invalid theme mode: '{other}'. Expected 'dark' or 'light'"
                )));
            }
        };
        config.tui_theme = if dark {
            crate::config::TuiTheme::Dark
        } else {
            crate::config::TuiTheme::Light
        };
        (dark, true)
    } else if args.toggle_theme {
        let current_is_dark = cache
            .last_is_dark
            .unwrap_or_else(|| config.tui_theme.is_dark());
        let new_is_dark = !current_is_dark;
        config.tui_theme = if new_is_dark {
            crate::config::TuiTheme::Dark
        } else {
            crate::config::TuiTheme::Light
        };
        (new_is_dark, true)
    } else {
        (config.tui_theme.is_dark(), false)
    };

    let mut wallpaper_path_buf: Option<PathBuf> = None;
    let fallback_wall_buf: Option<PathBuf> = cache.last_wallpaper_path.as_ref().map(PathBuf::from);
    let cached_wallpaper_still_exists = fallback_wall_buf.as_deref().is_none_or(|p| p.exists());

    // 2. Resolve Palette and CacheSource (checking cache before expensive image decoding)
    let (palette, source) = if let Some(ref hex_str) = args.color {
        let rgb = crate::core::RgbColor::from_hex(hex_str)?;
        let src = crate::config::CacheSource::Color(rgb);
        if !args.force && cache.is_up_to_date_with_config(&src, is_dark, config_fp.as_deref()) {
            println!("Theme is already up to date.");
            return Ok(true);
        }
        let palette = crate::core::GeneratedPalette::from_seed_color(rgb.into());
        (palette, src)
    } else if (args.toggle_theme || args.theme.is_some()) && args.wallpaper.is_none() {
        if cached_wallpaper_still_exists
            && let Some(ref cached_hex) = cache.last_seed_color
            && let Ok(rgb) = crate::core::RgbColor::from_hex(cached_hex)
        {
            let source = match fallback_wall_buf.as_deref() {
                Some(p) => crate::config::CacheSource::Wallpaper(p),
                None => crate::config::CacheSource::Color(rgb),
            };
            if !args.force
                && cache.is_up_to_date_with_config(&source, is_dark, config_fp.as_deref())
            {
                println!("Theme is already up to date.");
                return Ok(true);
            }
            let palette = crate::core::GeneratedPalette::from_seed_color(rgb.into());
            (palette, source)
        } else {
            // No valid cached seed/wallpaper -> fallback to active wallpaper
            let wp = crate::wallpaper::get_current_wallpaper(
                &config.wallpaper_backend,
                config.manual_wallpaper_path.as_deref(),
                config.primary_monitor.as_deref(),
            )?;
            let wp_path = wallpaper_path_buf.insert(wp).as_path();
            let source = crate::config::CacheSource::Wallpaper(wp_path);
            if !args.force
                && cache.is_up_to_date_with_config(&source, is_dark, config_fp.as_deref())
            {
                println!(
                    "Theme is already up to date with active wallpaper: {}",
                    wp_path.display()
                );
                return Ok(true);
            }
            let palette = crate::core::GeneratedPalette::from_image_path(wp_path)?;
            (palette, source)
        }
    } else {
        // Standard wallpaper-based apply
        let wp: PathBuf = if let Some(ref manual) = args.wallpaper {
            let p = crate::config::paths::AppPaths::expand_tilde(manual);
            if !p.exists() {
                return Err(crate::error::TermonetError::WallpaperNotFound(format!(
                    "Specified wallpaper not found: {}",
                    p.display()
                )));
            }
            p
        } else {
            crate::wallpaper::get_current_wallpaper(
                &config.wallpaper_backend,
                config.manual_wallpaper_path.as_deref(),
                config.primary_monitor.as_deref(),
            )?
        };

        let wp_path = wallpaper_path_buf.insert(wp).as_path();
        let source = crate::config::CacheSource::Wallpaper(wp_path);
        if !args.force && cache.is_up_to_date_with_config(&source, is_dark, config_fp.as_deref()) {
            println!(
                "Theme is already up to date with active wallpaper: {}",
                wp_path.display()
            );
            return Ok(true);
        }

        println!("Generating Material You palette for: {}", wp_path.display());
        let palette = crate::core::GeneratedPalette::from_image_path(wp_path)?;
        (palette, source)
    };

    let engine = crate::template::TemplateEngine::new();

    // 3. Load rules and apply enabled programs
    let rules = if let Some(r) = custom_rules {
        r
    } else {
        crate::template::ProgramRule::load_all()?
    };

    let mut applied_count = 0;
    for rule in &rules {
        if rule.matches_enabled_list(&config.enabled_programs) {
            let hook_executed =
                rule.apply_with_hook(&engine, &palette, is_dark, config.create_backups, true)?;
            applied_count += 1;
            println!("  ✔ Applied theme for: {}", rule.name);
            if let Some(cmd) = rule.effective_reload_cmd() {
                if hook_executed {
                    println!("    Hook executed: {}", cmd);
                } else {
                    eprintln!("    Warning: hook failed or timed out: {}", cmd);
                }
            }
        }
    }

    if applied_count == 0 && !config.enabled_programs.is_empty() {
        eprintln!(
            "Warning: none of the enabled programs ({}) have installed rules in rules/. Run 'termonet --list-rules' and 'termonet --install <id>' to install them.",
            config.enabled_programs.join(", ")
        );
    }

    // 4. Persist config changes (if --theme / --toggle-theme) and update cache only after apply succeeds
    if theme_modified {
        if let Some(cfg_path) = custom_config_file {
            config.save_to_path(cfg_path)?;
        } else {
            config.save()?;
        }
    }

    let mut new_cache = cache;
    new_cache.update_source_with_fingerprint(&source, is_dark, &palette.seed_color, config_fp);
    if let Some(state_path) = custom_state_file {
        new_cache.save_to_path(state_path)?;
    } else {
        new_cache.save()?;
    }

    println!(
        "Successfully applied Material You theme ({}) across {} program(s)!",
        if is_dark { "dark" } else { "light" },
        applied_count
    );
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_mode_skips_when_apply_false() {
        let args = CliArgs {
            apply: false,
            force: false,
            wallpaper: None,
            ..Default::default()
        };
        let res = run_cli_mode(&args);
        assert!(!res.unwrap());
    }

    #[test]
    fn test_cli_mode_fails_on_missing_wallpaper() {
        let args = CliArgs {
            apply: true,
            wallpaper: Some("/non_existent_wall_xyz_12345.png".to_string()),
            ..Default::default()
        };
        let res = run_cli_mode(&args);
        assert!(res.is_err());
    }

    #[test]
    fn test_cli_color_apply_succeeds_without_wallpaper_daemon() {
        let temp_dir =
            std::env::temp_dir().join(format!("termonet_test_cli_color_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&temp_dir);
        let state_file = temp_dir.join("state.toml");

        let args = CliArgs {
            apply: true,
            force: true,
            color: Some("#385e72".to_string()),
            ..Default::default()
        };

        let config = crate::config::AppConfig {
            enabled_programs: Vec::new(), // isolate from real filesystem dotfiles
            ..Default::default()
        };

        let res =
            run_cli_mode_with_context(&args, config, Some(Vec::new()), Some(&state_file), None);
        assert!(res.is_ok());
        assert!(res.unwrap());

        let cache = AppStateCache::load_from_path(&state_file).unwrap();
        assert_eq!(cache.last_seed_color.as_deref(), Some("#385e72"));
        assert_eq!(cache.last_wallpaper_path, None);

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_cli_theme_explicit_and_toggle() {
        let temp_dir =
            std::env::temp_dir().join(format!("termonet_test_cli_theme_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&temp_dir);
        let state_file = temp_dir.join("state.toml");
        let config_file = temp_dir.join("config.toml");

        let args_light = CliArgs {
            apply: true,
            force: true,
            color: Some("#385e72".to_string()),
            theme: Some("light".to_string()),
            ..Default::default()
        };

        let config = crate::config::AppConfig {
            enabled_programs: Vec::new(),
            ..Default::default()
        };

        run_cli_mode_with_context(
            &args_light,
            config.clone(),
            Some(Vec::new()),
            Some(&state_file),
            Some(&config_file),
        )
        .unwrap();

        let cfg = crate::config::AppConfig::load_from_path(&config_file).unwrap();
        assert_eq!(cfg.tui_theme, crate::config::TuiTheme::Light);
        let cache = AppStateCache::load_from_path(&state_file).unwrap();
        assert_eq!(cache.last_is_dark, Some(false));

        // Toggle theme
        let args_toggle = CliArgs {
            toggle_theme: true,
            ..Default::default()
        };
        run_cli_mode_with_context(
            &args_toggle,
            cfg,
            Some(Vec::new()),
            Some(&state_file),
            Some(&config_file),
        )
        .unwrap();

        let cfg2 = crate::config::AppConfig::load_from_path(&config_file).unwrap();
        assert_eq!(cfg2.tui_theme, crate::config::TuiTheme::Dark);
        let cache2 = AppStateCache::load_from_path(&state_file).unwrap();
        assert_eq!(cache2.last_is_dark, Some(true));

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_cli_theme_does_not_mutate_config_on_apply_failure() {
        let temp_dir = std::env::temp_dir().join(format!(
            "termonet_test_cli_transactional_{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&temp_dir);
        std::fs::create_dir_all(&temp_dir).unwrap();
        let state_file = temp_dir.join("state.toml");
        let config_file = temp_dir.join("config.toml");

        let initial_config = crate::config::AppConfig {
            tui_theme: crate::config::TuiTheme::Dark,
            enabled_programs: vec!["Broken".into()],
            ..Default::default()
        };
        initial_config.save_to_path(&config_file).unwrap();

        let failing_rule = crate::template::ProgramRule {
            id: String::new(),
            version: 1,
            name: "Broken".into(),
            template: "missing_nonexistent_template_xyz.tmpl".into(),
            output_path: temp_dir.join("out.conf"),
            target_config: None,
            import_line: None,
            import_position: crate::template::ImportPosition::Bottom,
            reload_cmd: None,
            comment_style: None,
        };

        let args = CliArgs {
            theme: Some("light".into()),
            color: Some("#385e72".into()),
            apply: true,
            ..Default::default()
        };

        let res = run_cli_mode_with_context(
            &args,
            initial_config,
            Some(vec![failing_rule]),
            Some(&state_file),
            Some(&config_file),
        );
        assert!(res.is_err());

        // config_file must still have Dark theme because apply failed
        let persisted = crate::config::AppConfig::load_from_path(&config_file).unwrap();
        assert_eq!(persisted.tui_theme, crate::config::TuiTheme::Dark);

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
