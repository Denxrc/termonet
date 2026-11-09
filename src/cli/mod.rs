//! Headless CLI trigger mode execution.

pub mod state;

pub use state::AppStateCache;
use std::path::PathBuf;

/// Command line execution arguments.
#[derive(Debug, Clone, Default)]
pub struct CliArgs {
    pub apply: bool,
    pub force: bool,
    pub wallpaper: Option<String>,
    pub no_backup: bool,
    pub backup: bool,
}

/// Executes headless apply without TUI if `--apply` flag is passed.
/// Returns `Ok(true)` if headless mode was executed, `Ok(false)` if interactive TUI should proceed.
pub fn run_cli_mode(args: &CliArgs) -> crate::error::Result<bool> {
    if !args.apply {
        return Ok(false);
    }

    let mut config = crate::config::AppConfig::load()?.unwrap_or_default();
    if args.no_backup {
        config.create_backups = false;
    } else if args.backup {
        config.create_backups = true;
    }

    // 1. Resolve wallpaper path
    let wallpaper_path: PathBuf = if let Some(ref manual) = args.wallpaper {
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

    // 2. Check cache to avoid idle duplicate executions
    let cache = AppStateCache::load().unwrap_or_default();
    if !args.force && cache.is_up_to_date(&wallpaper_path) {
        println!(
            "Theme is already up to date with active wallpaper: {}",
            wallpaper_path.display()
        );
        return Ok(true);
    }

    println!(
        "Generating Material You palette for: {}",
        wallpaper_path.display()
    );
    let palette = crate::core::GeneratedPalette::from_image_path(&wallpaper_path)?;
    let engine = crate::template::TemplateEngine::new();

    // 3. Load rules and apply enabled programs
    let rules = crate::template::ProgramRule::load_all()?;
    let mut applied_count = 0;
    for rule in &rules {
        if config.enabled_programs.iter().any(|p| p.eq_ignore_ascii_case(&rule.name)) {
            rule.apply(&engine, &palette, config.tui_theme.is_dark(), config.create_backups)?;
            applied_count += 1;
            println!("  ✔ Applied theme for: {}", rule.name);

            // Execute reload hook if present
            if let Some(ref cmd) = rule.reload_cmd {
                let status = std::process::Command::new("sh")
                    .arg("-c")
                    .arg(cmd)
                    .stdin(std::process::Stdio::null())
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null())
                    .status();
                if let Ok(st) = status
                    && st.success()
                {
                    println!("    Hook executed: {}", cmd);
                }
            }
        }
    }

    // 4. Update cache
    let mut new_cache = cache;
    new_cache.last_wallpaper_path = Some(wallpaper_path.display().to_string());
    let _ = new_cache.save();

    println!(
        "Successfully applied Material You theme across {} program(s)!",
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
}
