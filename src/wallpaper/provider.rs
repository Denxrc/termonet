//! Wallpaper provider trait, registry, and concrete providers.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::error::{Result, TermonetError};
use crate::wallpaper::monitor::MonitorWallpapers;
use crate::wallpaper::path::clean_wallpaper_path;

/// Diagnostic inspection details for an individual wallpaper provider.
#[derive(Debug, Clone)]
pub struct ProviderDiagnostic {
    pub provider_id: &'static str,
    pub display_name: &'static str,
    pub available: bool,
    pub raw_output: Option<String>,
    pub resolved_monitors: Option<HashMap<String, PathBuf>>,
    pub error: Option<String>,
}

/// Common trait implemented by all wallpaper manager integrations.
pub trait WallpaperProvider: Send + Sync {
    /// Unique provider identifier (e.g. "awww", "hyprpaper", "kde").
    fn id(&self) -> &'static str;

    /// Human-readable display name.
    fn display_name(&self) -> &'static str;

    /// Fast non-blocking check whether this provider's service/daemon is running or configured.
    fn is_available(&self) -> bool;

    /// Queries the provider and returns active monitors with validated wallpaper paths.
    fn query(&self) -> Result<MonitorWallpapers>;

    /// Executes deep diagnostic query for `--debug-wallpaper` reporting.
    fn query_diagnostic(&self) -> ProviderDiagnostic;
}

/// Provider for `awww` and `swww` daemons.
#[derive(Debug, Default, Clone, Copy)]
pub struct AwwwProvider;

impl WallpaperProvider for AwwwProvider {
    fn id(&self) -> &'static str {
        "awww"
    }

    fn display_name(&self) -> &'static str {
        "awww / swww"
    }

    fn is_available(&self) -> bool {
        // Check if awww or swww binaries exist
        which("awww") || which("swww")
    }

    fn query(&self) -> Result<MonitorWallpapers> {
        crate::wallpaper::wayland::query_awww_or_swww()
    }

    fn query_diagnostic(&self) -> ProviderDiagnostic {
        let mut raw = None;
        let mut error = None;
        let mut resolved = None;

        if let Ok(out) = Command::new("awww").args(["query", "--all"]).output() {
            let stdout = String::from_utf8_lossy(&out.stdout).to_string();
            raw = Some(format!("Command 'awww query --all':\n{}", stdout.trim()));
            if out.status.success() {
                let parsed = crate::wallpaper::wayland::parse_awww_or_swww_output(&stdout);
                if !parsed.is_empty() {
                    resolved = Some(parsed);
                }
            } else {
                error = Some(String::from_utf8_lossy(&out.stderr).to_string());
            }
        } else if let Ok(out) = Command::new("swww").arg("query").output() {
            let stdout = String::from_utf8_lossy(&out.stdout).to_string();
            raw = Some(format!("Command 'swww query':\n{}", stdout.trim()));
            if out.status.success() {
                let parsed = crate::wallpaper::wayland::parse_awww_or_swww_output(&stdout);
                if !parsed.is_empty() {
                    resolved = Some(parsed);
                }
            } else {
                error = Some(String::from_utf8_lossy(&out.stderr).to_string());
            }
        } else {
            error = Some("Neither awww nor swww binary could be executed".into());
        }

        ProviderDiagnostic {
            provider_id: self.id(),
            display_name: self.display_name(),
            available: self.is_available(),
            raw_output: raw,
            resolved_monitors: resolved,
            error,
        }
    }
}

/// Provider for `hyprpaper`.
#[derive(Debug, Default, Clone, Copy)]
pub struct HyprpaperProvider;

impl WallpaperProvider for HyprpaperProvider {
    fn id(&self) -> &'static str {
        "hyprpaper"
    }

    fn display_name(&self) -> &'static str {
        "hyprpaper"
    }

    fn is_available(&self) -> bool {
        which("hyprctl") || which("hyprpaper") || std::env::var_os("HYPRLAND_INSTANCE_SIGNATURE").is_some()
    }

    fn query(&self) -> Result<MonitorWallpapers> {
        crate::wallpaper::wayland::query_hyprpaper()
    }

    fn query_diagnostic(&self) -> ProviderDiagnostic {
        let mut raw = None;
        let mut error = None;
        let mut resolved = None;

        if let Ok(out) = Command::new("hyprctl").args(["hyprpaper", "listactive"]).output() {
            let stdout = String::from_utf8_lossy(&out.stdout).to_string();
            raw = Some(format!("Command 'hyprctl hyprpaper listactive':\n{}", stdout.trim()));
            if out.status.success() {
                let parsed = crate::wallpaper::wayland::parse_hyprpaper_active_output(&stdout);
                if !parsed.is_empty() {
                    resolved = Some(parsed);
                }
            } else {
                error = Some(String::from_utf8_lossy(&out.stderr).to_string());
            }
        }

        if resolved.is_none() {
            let mut candidates = Vec::new();
            if let Some(config_dir) = std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from) {
                candidates.push(config_dir.join("hypr/hyprpaper.conf"));
            }
            if let Some(home) = std::env::var_os("HOME").map(PathBuf::from) {
                candidates.push(home.join(".config/hypr/hyprpaper.conf"));
            }
            for conf in candidates {
                if conf.exists() && let Ok(content) = std::fs::read_to_string(&conf) {
                    let parsed = crate::wallpaper::wayland::parse_hyprpaper_conf(&content);
                    if !parsed.is_empty() {
                        resolved = Some(parsed);
                        raw = Some(format!("File {} ({} lines)", conf.display(), content.lines().count()));
                        break;
                    }
                }
            }
        }

        ProviderDiagnostic {
            provider_id: self.id(),
            display_name: self.display_name(),
            available: self.is_available(),
            raw_output: raw,
            resolved_monitors: resolved,
            error,
        }
    }
}

/// Provider for `wpaperd`.
#[derive(Debug, Default, Clone, Copy)]
pub struct WpaperdProvider;

impl WallpaperProvider for WpaperdProvider {
    fn id(&self) -> &'static str {
        "wpaperd"
    }

    fn display_name(&self) -> &'static str {
        "wpaperd"
    }

    fn is_available(&self) -> bool {
        which("wpaperctl") || which("wpaperd")
    }

    fn query(&self) -> Result<MonitorWallpapers> {
        crate::wallpaper::wayland::query_wpaperd()
    }

    fn query_diagnostic(&self) -> ProviderDiagnostic {
        let mut raw = None;
        let mut error = None;
        let mut resolved = None;

        if let Ok(out) = Command::new("wpaperctl").arg("all-wallpapers").output() {
            let stdout = String::from_utf8_lossy(&out.stdout).to_string();
            raw = Some(format!("Command 'wpaperctl all-wallpapers':\n{}", stdout.trim()));
            if out.status.success() {
                let parsed = crate::wallpaper::wayland::parse_wpaperctl_output(&stdout);
                if !parsed.is_empty() {
                    resolved = Some(parsed);
                }
            } else {
                error = Some(String::from_utf8_lossy(&out.stderr).to_string());
            }
        }

        ProviderDiagnostic {
            provider_id: self.id(),
            display_name: self.display_name(),
            available: self.is_available(),
            raw_output: raw,
            resolved_monitors: resolved,
            error,
        }
    }
}

/// Provider for `swaybg`.
#[derive(Debug, Default, Clone, Copy)]
pub struct SwaybgProvider;

impl WallpaperProvider for SwaybgProvider {
    fn id(&self) -> &'static str {
        "swaybg"
    }

    fn display_name(&self) -> &'static str {
        "swaybg"
    }

    fn is_available(&self) -> bool {
        which("swaybg") || crate::wallpaper::wayland::query_swaybg().is_ok()
    }

    fn query(&self) -> Result<MonitorWallpapers> {
        crate::wallpaper::wayland::query_swaybg()
    }

    fn query_diagnostic(&self) -> ProviderDiagnostic {
        let mut raw = None;
        let mut resolved = None;
        let mut error = None;

        match crate::wallpaper::wayland::query_swaybg() {
            Ok(monitors) => {
                raw = Some(format!("Scanned /proc: detected {} active monitor target(s)", monitors.outputs.len()));
                resolved = Some(monitors.outputs);
            }
            Err(e) => {
                error = Some(e.to_string());
            }
        }

        ProviderDiagnostic {
            provider_id: self.id(),
            display_name: self.display_name(),
            available: self.is_available(),
            raw_output: raw,
            resolved_monitors: resolved,
            error,
        }
    }
}

/// Provider for GNOME Desktop.
#[derive(Debug, Default, Clone, Copy)]
pub struct GnomeProvider;

impl WallpaperProvider for GnomeProvider {
    fn id(&self) -> &'static str {
        "gnome"
    }

    fn display_name(&self) -> &'static str {
        "GNOME Desktop"
    }

    fn is_available(&self) -> bool {
        std::env::var("XDG_CURRENT_DESKTOP")
            .unwrap_or_default()
            .to_uppercase()
            .contains("GNOME")
            || which("gsettings")
    }

    fn query(&self) -> Result<MonitorWallpapers> {
        crate::wallpaper::de::query_gnome()
    }

    fn query_diagnostic(&self) -> ProviderDiagnostic {
        let mut raw = Vec::new();
        let mut resolved = None;
        let mut error = None;

        if let Ok(out) = Command::new("gsettings")
            .args(["get", "org.gnome.desktop.background", "picture-options"])
            .output()
        {
            raw.push(format!("picture-options: {}", String::from_utf8_lossy(&out.stdout).trim()));
        }

        if let Ok(out) = Command::new("gsettings")
            .args(["get", "org.gnome.desktop.interface", "color-scheme"])
            .output()
        {
            raw.push(format!("color-scheme: {}", String::from_utf8_lossy(&out.stdout).trim()));
        }

        if let Ok(out) = Command::new("gsettings")
            .args(["get", "org.gnome.desktop.background", "picture-uri-dark"])
            .output()
        {
            raw.push(format!("picture-uri-dark: {}", String::from_utf8_lossy(&out.stdout).trim()));
        }

        if let Ok(out) = Command::new("gsettings")
            .args(["get", "org.gnome.desktop.background", "picture-uri"])
            .output()
        {
            raw.push(format!("picture-uri: {}", String::from_utf8_lossy(&out.stdout).trim()));
        }

        match crate::wallpaper::de::query_gnome() {
            Ok(monitors) => {
                resolved = Some(monitors.outputs);
            }
            Err(e) => {
                error = Some(e.to_string());
            }
        }

        ProviderDiagnostic {
            provider_id: self.id(),
            display_name: self.display_name(),
            available: self.is_available(),
            raw_output: if raw.is_empty() { None } else { Some(raw.join("\n")) },
            resolved_monitors: resolved,
            error,
        }
    }
}

/// Provider for KDE Plasma Desktop.
#[derive(Debug, Default, Clone, Copy)]
pub struct KdeProvider;

impl WallpaperProvider for KdeProvider {
    fn id(&self) -> &'static str {
        "kde"
    }

    fn display_name(&self) -> &'static str {
        "KDE Plasma"
    }

    fn is_available(&self) -> bool {
        if let Some(config_dir) = std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from)
            && config_dir.join("plasma-org.kde.plasma.desktop-appletsrc").exists()
        {
            return true;
        }
        let home = std::env::var_os("HOME").map(PathBuf::from);
        let path = home.map(|h| h.join(".config/plasma-org.kde.plasma.desktop-appletsrc"));
        path.is_some_and(|p| p.exists())
    }

    fn query(&self) -> Result<MonitorWallpapers> {
        crate::wallpaper::de::query_kde()
    }

    fn query_diagnostic(&self) -> ProviderDiagnostic {
        let mut raw = None;
        let mut resolved = None;
        let mut error = None;

        match crate::wallpaper::de::query_kde() {
            Ok(monitors) => {
                raw = Some("Parsed ~/.config/plasma-org.kde.plasma.desktop-appletsrc containments".into());
                resolved = Some(monitors.outputs);
            }
            Err(e) => {
                error = Some(e.to_string());
            }
        }

        ProviderDiagnostic {
            provider_id: self.id(),
            display_name: self.display_name(),
            available: self.is_available(),
            raw_output: raw,
            resolved_monitors: resolved,
            error,
        }
    }
}

/// Provider for feh (X11).
#[derive(Debug, Default, Clone, Copy)]
pub struct FehProvider;

impl WallpaperProvider for FehProvider {
    fn id(&self) -> &'static str {
        "feh"
    }

    fn display_name(&self) -> &'static str {
        "feh (X11)"
    }

    fn is_available(&self) -> bool {
        std::env::var_os("HOME")
            .map(PathBuf::from)
            .is_some_and(|h| h.join(".fehbg").exists())
    }

    fn query(&self) -> Result<MonitorWallpapers> {
        let home = std::env::var_os("HOME").map(PathBuf::from);
        if let Some(h) = home {
            let fehbg = h.join(".fehbg");
            if fehbg.exists() && let Ok(content) = std::fs::read_to_string(&fehbg) {
                if let Some(path) = clean_wallpaper_path(&content) {
                    let mut outputs = HashMap::new();
                    outputs.insert("default".to_string(), path);
                    return Ok(MonitorWallpapers { outputs });
                }
                for token in content.split_whitespace() {
                    let clean = token.trim_matches(|c| c == '\'' || c == '"');
                    if (clean.starts_with('/') || clean.starts_with('~'))
                        && let Some(path) = clean_wallpaper_path(clean)
                    {
                        let mut outputs = HashMap::new();
                        outputs.insert("default".to_string(), path);
                        return Ok(MonitorWallpapers { outputs });
                    }
                }
            }
        }
        Err(TermonetError::WallpaperNotFound("No active feh wallpaper found in ~/.fehbg".into()))
    }

    fn query_diagnostic(&self) -> ProviderDiagnostic {
        let mut raw = None;
        let mut resolved = None;
        let mut error = None;

        if let Some(home) = std::env::var_os("HOME").map(PathBuf::from) {
            let p = home.join(".fehbg");
            if p.exists() && let Ok(c) = std::fs::read_to_string(&p) {
                raw = Some(format!("Content of ~/.fehbg:\n{}", c.trim()));
            }
        }

        match self.query() {
            Ok(monitors) => resolved = Some(monitors.outputs),
            Err(e) => error = Some(e.to_string()),
        }

        ProviderDiagnostic {
            provider_id: self.id(),
            display_name: self.display_name(),
            available: self.is_available(),
            raw_output: raw,
            resolved_monitors: resolved,
            error,
        }
    }
}

/// Provider for nitrogen (X11).
#[derive(Debug, Default, Clone, Copy)]
pub struct NitrogenProvider;

impl WallpaperProvider for NitrogenProvider {
    fn id(&self) -> &'static str {
        "nitrogen"
    }

    fn display_name(&self) -> &'static str {
        "nitrogen (X11)"
    }

    fn is_available(&self) -> bool {
        std::env::var_os("HOME")
            .map(PathBuf::from)
            .is_some_and(|h| h.join(".config/nitrogen/bg-saved.cfg").exists())
    }

    fn query(&self) -> Result<MonitorWallpapers> {
        let home = std::env::var_os("HOME").map(PathBuf::from);
        if let Some(h) = home {
            let cfg = h.join(".config/nitrogen/bg-saved.cfg");
            if cfg.exists() && let Ok(content) = std::fs::read_to_string(&cfg) {
                let mut outputs = HashMap::new();
                let mut current_screen = "default".to_string();

                for line in content.lines() {
                    let trimmed = line.trim();
                    if trimmed.starts_with('[') && trimmed.ends_with(']') {
                        current_screen = trimmed[1..trimmed.len() - 1].to_string();
                    } else if let Some((k, v)) = trimmed.split_once('=')
                        && k.trim() == "file"
                        && let Some(path) = clean_wallpaper_path(v.trim())
                    {
                        outputs.insert(current_screen.clone(), path);
                    }
                }

                if !outputs.is_empty() {
                    return Ok(MonitorWallpapers { outputs });
                }
            }
        }
        Err(TermonetError::WallpaperNotFound("No active nitrogen wallpaper found in ~/.config/nitrogen/bg-saved.cfg".into()))
    }

    fn query_diagnostic(&self) -> ProviderDiagnostic {
        let mut raw = None;
        let mut resolved = None;
        let mut error = None;

        if let Some(home) = std::env::var_os("HOME").map(PathBuf::from) {
            let p = home.join(".config/nitrogen/bg-saved.cfg");
            if p.exists() && let Ok(c) = std::fs::read_to_string(&p) {
                raw = Some(format!("Content of ~/.config/nitrogen/bg-saved.cfg:\n{}", c.trim()));
            }
        }

        match self.query() {
            Ok(monitors) => resolved = Some(monitors.outputs),
            Err(e) => error = Some(e.to_string()),
        }

        ProviderDiagnostic {
            provider_id: self.id(),
            display_name: self.display_name(),
            available: self.is_available(),
            raw_output: raw,
            resolved_monitors: resolved,
            error,
        }
    }
}

/// Helper function to check if a binary exists in system PATH.
fn which(binary: &str) -> bool {
    let Ok(path_var) = std::env::var("PATH") else {
        return false;
    };
    for dir in path_var.split(':') {
        let bin_path = Path::new(dir).join(binary);
        if bin_path.is_file() {
            return true;
        }
    }
    false
}

/// Instantiates and returns a vector of all supported wallpaper providers.
#[must_use]
pub fn all_providers() -> Vec<Box<dyn WallpaperProvider>> {
    vec![
        Box::new(AwwwProvider),
        Box::new(HyprpaperProvider),
        Box::new(WpaperdProvider),
        Box::new(SwaybgProvider),
        Box::new(GnomeProvider),
        Box::new(KdeProvider),
        Box::new(FehProvider),
        Box::new(NitrogenProvider),
    ]
}

/// Runs all provider diagnostics and returns a formatted terminal report.
#[must_use]
pub fn run_all_diagnostics() -> String {
    let mut out = String::new();
    out.push_str("==============================================================================\n");
    out.push_str("                        Termonet Wallpaper Diagnostics\n");
    out.push_str("==============================================================================\n\n");

    let desktop = std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_else(|_| "Unknown / Unset".into());
    let session_type = std::env::var("XDG_SESSION_TYPE").unwrap_or_else(|_| "Unknown".into());
    out.push_str(&format!("Desktop Environment: {}\n", desktop));
    out.push_str(&format!("Session Type:        {}\n", session_type));

    if let Some(sig) = std::env::var_os("HYPRLAND_INSTANCE_SIGNATURE") {
        out.push_str(&format!("Hyprland Signature:  {}\n", sig.to_string_lossy()));
    }
    if let Some(sock) = std::env::var_os("SWAYSOCK") {
        out.push_str(&format!("Sway Socket:         {}\n", sock.to_string_lossy()));
    }
    out.push_str("\n------------------------------------------------------------------------------\n");

    for provider in all_providers() {
        let diag = provider.query_diagnostic();
        let status = if diag.resolved_monitors.is_some() {
            "✔ ACTIVE & DETECTED"
        } else if diag.available {
            "⚡ AVAILABLE (Daemon/binary present, no active wallpaper)"
        } else {
            "✘ NOT FOUND / INACTIVE"
        };

        out.push_str(&format!("[{}]  {}\n", diag.display_name, status));

        if let Some(ref raw) = diag.raw_output {
            out.push_str("  Output / State:\n");
            for line in raw.lines() {
                out.push_str(&format!("    {}\n", line));
            }
        }

        if let Some(ref monitors) = diag.resolved_monitors {
            out.push_str("  Active Monitors:\n");
            for (mon, path) in monitors {
                out.push_str(&format!("    • {} => {}\n", mon, path.display()));
            }
        }

        if let Some(ref err) = diag.error {
            out.push_str(&format!("  Error: {}\n", err.trim()));
        }

        out.push('\n');
    }

    out.push_str("==============================================================================\n");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_providers_have_unique_ids() {
        let providers = all_providers();
        assert_eq!(providers.len(), 8);
        let mut ids = std::collections::HashSet::new();
        for p in &providers {
            assert!(ids.insert(p.id()), "Duplicate provider id: {}", p.id());
        }
    }

    #[test]
    fn test_run_all_diagnostics_produces_report() {
        let report = run_all_diagnostics();
        assert!(report.contains("Termonet Wallpaper Diagnostics"));
        assert!(report.contains("awww / swww"));
        assert!(report.contains("hyprpaper"));
        assert!(report.contains("swaybg"));
    }
}
