<div align="center">

```text
▀█▀ █▀▀▀ █▀▀█ █▀▄▀█ █▀▀█ █▄ █ █▀▀▀ ▀█▀
 █  █▀▀  █▄▄▀ █ █ █ █  █ █ ▀█ █▀▀   █ 
 █  █▄▄▄ █  █ █   █ █▄▄█ █  █ █▄▄▄  █ 
```

**Fast, dynamic Material You system theme and palette generator for Linux**

[![CI](https://img.shields.io/github/actions/workflow/status/Denxrc/termonet/ci.yml?branch=master&label=CI&logo=githubactions&logoColor=white)](https://github.com/Denxrc/termonet/actions/workflows/ci.yml)
[![Release](https://img.shields.io/badge/release-v1.6.0-blue?logo=github&logoColor=white)](https://github.com/Denxrc/termonet/releases)
[![Rust](https://img.shields.io/badge/rust-edition_2024-orange?logo=rust&logoColor=white)](Cargo.toml)
[![Platform](https://img.shields.io/badge/linux-wayland_%7C_x11-blueviolet?logo=linux&logoColor=white)]()
[![License](https://img.shields.io/badge/license-MIT-green)](LICENSE)
[![XERJ Corpus Hub](https://hub.xerj.org/contributors/Denxrc/badge.svg)](https://hub.xerj.org/contributors/Denxrc)

[English](README.md) · [Русский](README.ru.md) · [Architecture](ARCHITECTURE.md) · [Releases](https://github.com/Denxrc/termonet/releases) · [Report Bug](.github/ISSUE_TEMPLATE/bug_report.md)

</div>

---

## ▸ Overview

**Termonet** extracts dominant colors from your active desktop wallpaper (or a direct HEX seed), derives harmonized **Material You (Material 3)** light/dark schemes alongside 16-color ANSI terminal palettes, and injects them safely into your application dotfiles without altering custom settings. It runs with zero root privileges and zero Python/Node runtime dependencies, exposing both an interactive `ratatui` dashboard and a sub-5ms headless CLI.

> **Status (`v1.6.0`)**: Interactive 4-step TUI + onboarding wizard, external template registry ([`Denxrc/termonet-apps`](https://github.com/Denxrc/termonet-apps) + custom `~/.config/termonet/repos/*.toml`), multi-monitor wallpaper discovery (`awww`, `swww`, `hyprpaper`, `wpaperd`, `swaybg`, GNOME, KDE Plasma, `feh`, `nitrogen`), MiniJinja templating, and headless CLI (`--apply`, `--toggle-theme`, `--theme`, `-c/--color`, `--install`) are production-ready. Static Linux `x86_64` and `aarch64` builds are published on [GitHub Releases](https://github.com/Denxrc/termonet/releases).

---

## ▸ Quick Start

Download a prebuilt static binary (`x86_64` or `aarch64`, zero `glibc` version lock-in), verify its checksum, and launch Termonet:

```bash
# 1. Download and verify v1.6.0 (auto-detects x86_64 / aarch64)
VERSION="v1.6.0" ARCH="$(uname -m)"
curl -LO "https://github.com/Denxrc/termonet/releases/download/${VERSION}/termonet-${VERSION}-linux-${ARCH}.tar.gz"
curl -LO "https://github.com/Denxrc/termonet/releases/download/${VERSION}/SHA256SUMS.txt"
sha256sum --ignore-missing -c SHA256SUMS.txt
mkdir -p termonet-pkg && tar -xzf "termonet-${VERSION}-linux-${ARCH}.tar.gz" -C termonet-pkg
install -Dm755 termonet-pkg/termonet ~/.local/bin/termonet

# 2. Launch interactive TUI (opens onboarding wizard on first run)
termonet

# 3. Headless automation & instant dark/light toggle
termonet --apply
termonet --toggle-theme
```

---

## ▸ Design & Capabilities

| Pillar | Implementation (`v1.6.0`) |
| :--- | :--- |
| **Material You Core** | `material-colors` (`Variant::Content`) preserving atmospheric tones across 19 MD3 semantic roles + light/dark ANSI 16 palettes via HCT. |
| **Fast Image Pipeline** | In-memory 128×128 downscaling (`image`), frame-0 GIF/WebP extraction, Celebi quantization, and stack-allocated HEX/RGB serialization. |
| **External Apps Registry** | Zero built-in templates in the binary; official [`termonet-apps`](https://github.com/Denxrc/termonet-apps) + custom `repos/*.toml` with `ureq`/`rustls`, SHA-256 verification, in-memory pre-validation, and weekly auto-updates. |
| **Responsive TUI & Wizard** | `ratatui` 4-step workflow with custom 5px Unicode half-block logo, `EN`/`RU` localization, unified installed/remote catalog, confirmation warnings, and live monitor cycling (`Tab`/`m`). |
| **Broad Wallpaper Discovery** | Concurrent `std::thread::scope` probing across Wayland (`awww`, `swww`, `hyprpaper` socket/conf, `wpaperd`, `swaybg` `/proc`), GNOME, KDE Plasma, and X11 (`feh`, `nitrogen`). |
| **Non-Destructive Templates** | Embedded MiniJinja engine with isolated marker blocks (`>>> termonet colors >>>`), legacy Pywal replacement, and `.original` master backups when enabled. |
| **Symlink-Safe & Idempotent** | `safe_write()` resolves symlinks, preserves POSIX permissions, skips no-op writes, and `state.toml` fingerprints prevent idle reloads (`< 5 ms` on `--toggle-theme`). |

---

## ▸ Architecture

```text
Wallpaper Daemon / -w PATH / -c #HEX
        │
        ▼
┌─────────────────────────────────────────────┐
│  Termonet Core (src/core · src/wallpaper)   │  Pure Rust · No Root · ~3.5 MB
│  · Concurrent providers + path sanitizer    │
│  · 128x128 decode -> Celebi -> CAM16 + HCT  │
│  · state.toml fingerprint idempotency guard │
└─────────────────────────────────────────────┘
        │
        ├──► Interactive TUI (src/tui: Palette -> Programs -> Hooks -> Apply)
        └──► Headless CLI (src/cli: --apply / --toggle-theme / --theme / -c)
                │
                ▼
     MiniJinja -> safe_write() + Marker Injection -> Detached Reload Hooks
```

> [!TIP]
> For subsystem breakdowns, Mermaid lifecycle diagrams, IPC timeouts, the 6 safety invariants, and contributor playbooks, read **[ARCHITECTURE.md](ARCHITECTURE.md)** ([Русский](ARCHITECTURE.ru.md)).

<details>
<summary>Repository & subsystem layout (8 modular subsystems)</summary>

```text
termonet/
├── Cargo.toml                  # Package manifest (Edition 2024, LTO, strip = true, panic = "abort")
└── src/
    ├── main.rs                 # Entry point, strict CLI argument parser, and dispatch
    ├── lib.rs & error.rs       # Crate root and typed TermonetError (thiserror)
    ├── cli/                    # Headless runner (--apply, --toggle-theme, --theme, -c/--color)
    ├── config/                 # XDG paths, Schema v3 config.toml, system dark detector, state.toml
    ├── core/                   # Image preprocessing, RgbColor HEX parser, 19-role MD3 + ANSI 16
    ├── registry/               # Remote registry client (ureq/rustls/sha2), catalog merge, install guards
    ├── wallpaper/              # WallpaperProvider trait, Wayland/DE/X11 backends, monitor resolution
    ├── template/               # MiniJinja engine, symlink-safe safe_write(), backups, marker injection
    ├── tui/                    # Ratatui event loop, TerminalGuard RAII, 5px PixelFont, 6 views
    └── i18n/                   # English and Russian localization catalogs
```

</details>

---

## ▸ CLI, TUI & Compositor Integration

### Common Commands

```bash
termonet                                # Launch interactive TUI (auto-detects wallpaper)
termonet --apply                        # Headless: apply theme from active wallpaper and exit
termonet --toggle-theme                 # Instantly flip dark/light mode (< 5 ms from cache)
termonet --theme dark                   # Set and apply explicit theme mode ('dark' or 'light')
termonet -c "#385e72" -a                # Generate & apply palette directly from HEX color
termonet --apply -w ~/Pictures/wall.png # Apply specific wallpaper in headless mode
termonet --list-rules                   # List installed, local, and remote repository programs
termonet --install kitty                # Preview warnings, verify SHA-256, and install Kitty
termonet --update-rules                 # Sync repos and update unmodified installed programs
termonet --debug-wallpaper              # Inspect raw outputs from all wallpaper providers
```

<details>
<summary>Full CLI options reference (--help)</summary>

```text
Usage: termonet [OPTIONS] [WALLPAPER_PATH]

Headless / Automation Options:
  -a, --apply              Headless mode: apply theme from active wallpaper and exit
  -f, --force              Force re-application or overwrite locally edited rules
  -c, --color <HEX>        Generate palette directly from seed HEX color (e.g. '#385e72')
      --toggle-theme       Instantly toggle between dark and light themes and apply
      --theme <MODE>       Set and apply theme mode ('dark' or 'light')

Registry & Template Management:
      --list-rules         List installed, local custom, and repository programs
      --install <ID>       Install rule & template from repository (supports repo/id)
      --remove-rule <ID>   Uninstall a registry-managed rule & template
      --update-rules       Sync repositories and update unmodified installed rules
      --reset-rules        Force-reset tracked rules to upstream (creates .bak)
      --sync-repos         Refresh cached repository indexes without installing
  -y, --yes                Confirm install/remove prompts non-interactively

General Options:
  -w, --wallpaper <PATH>   Path to wallpaper image (PNG, JPG, GIF, WebP)
  -i, --wizard, --init     Launch first-run configuration wizard
      --no-backup          Disable creation of backup copies (.bak, .original)
      --backup             Enable creation of backup copies (.bak, .original)
      --debug-wallpaper    Print active wallpaper managers diagnostic report
  -V, --version            Print version information
  -h, --help               Print help information
```

</details>

<details>
<summary>TUI workflow & keyboard shortcuts</summary>

| Step | Screen | Purpose |
| :---: | :--- | :--- |
| **1** | **Palette** | Preview seed color, 19 MD3 roles, and ANSI 16 swatches; switch monitors or enter path/HEX |
| **2** | **Programs** | Toggle installed/local targets or install `[↓]` repository programs with security previews |
| **3** | **Hooks** | Enable or disable reload commands (`hyprctl reload`, `killall -SIGUSR2 waybar`) |
| **4** | **Apply** | Review pending writes and detached hooks before committing to disk |

| Shortcut | Action |
| :--- | :--- |
| `Left` / `Right` (`h` / `l`) | Switch between steps (1..4) |
| `1`, `2`, `3`, `4` | Jump directly to step 1..4 |
| `Up` / `Down` (`k` / `j`) | Navigate lists and wizard options |
| `Space` | Toggle program/hook or install available `[↓]` program |
| `u` | Sync repositories and update installed rules (Step 2) |
| `d` / `Delete` | Remove tracked registry program files (Step 2) |
| `Enter` | Apply theme (Step 4) / Confirm or save (Wizard / Modal) |
| `Tab` / `m` | Cycle active monitor wallpapers (Step 1, multi-monitor) |
| `w` / `o` | Open modal to enter wallpaper path or HEX color (`#385e72`, `0x...`) |
| `r` | Re-run wallpaper auto-detection from active daemon (Step 1) |
| `s` / `F2` | Open setup / settings wizard |
| `Esc` | Close modal / roll back settings wizard / quit on main screen |
| `q` / `Ctrl+C` | Quit Termonet |

</details>

<details>
<summary>Compositor bindings, wallpaper hooks & systemd service (Hyprland, Sway, waypaper, systemd)</summary>

Because Termonet has no background daemon and uses `~/.local/state/termonet/state.toml` to skip redundant work, you can bind it directly to hotkeys, hooks, or a timer:

#### Hyprland (`~/.config/hypr/hyprland.conf` / `hyprland.lua`)

```ini
exec-once = termonet --apply
bind = $mainMod, T, exec, termonet --toggle-theme
```

#### Sway (`~/.config/sway/config`)

```ini
exec termonet --apply
bindsym $mod+t exec termonet --toggle-theme
```

#### Wallpaper Switch Script / `waypaper`

```bash
# Shell wrapper for awww / swww
awww img "$1" && termonet --apply -w "$1"
```

```ini
# ~/.config/waypaper/config.ini
post_command = termonet --apply -w $wallpaper
```

#### `systemd` User Service & Timer

```ini
# ~/.config/systemd/user/termonet.service
[Unit]
Description=Update Material You theme from active desktop wallpaper

[Service]
Type=oneshot
ExecStart=%h/.local/bin/termonet --apply
```

```ini
# ~/.config/systemd/user/termonet.timer
[Unit]
Description=Periodically check active wallpaper for Termonet

[Timer]
OnBootSec=10s
OnUnitActiveSec=30s

[Install]
WantedBy=timers.target
```

```bash
systemctl --user enable --now termonet.timer
```

</details>

---

## ▸ Configuration, Repositories & Custom Templates

Termonet stores its commented configuration at `~/.config/termonet/config.toml` (Schema v3; missing keys use defaults, and minimal user configs without the generated header are preserved untouched), repository sources in `~/.config/termonet/repos/*.toml`, and registry state/cache in `~/.local/state/termonet/installed.toml` and `~/.local/state/termonet/registry_cache/`:

```toml
language = "en"              # "ru" | "en"
tui_theme = "dark"           # "dark" | "light" | "system"
font_mode = "pixel"          # "pixel" | "standard"
display_server = "wayland"   # "wayland" | "x11"
wallpaper_backend = "auto"   # "auto", "awww", "swww", "hyprpaper", "wpaperd", "swaybg", "gnome", "kde", "feh", "nitrogen", "manual"
enabled_programs = []
auto_update_rules = true     # Weekly check (7 days) on TUI startup or plain --apply
create_backups = false
```

<details>
<summary>Full commented config.toml reference (Schema v3) & custom repositories</summary>

```toml
# ==============================================================================
# Termonet Configuration
# Path: ~/.config/termonet/config.toml
# Schema Version: 3
# ==============================================================================

language = "en"
tui_theme = "dark"
font_mode = "pixel"
display_server = "wayland"
wallpaper_backend = "auto"

# Primary monitor name (uncomment to specify)
# primary_monitor = "eDP-1"

# Manual path to a wallpaper image file (used when wallpaper_backend = "manual" or as fallback)
# manual_wallpaper_path = "/path/to/wallpaper.png"

enabled_programs = []
auto_update_rules = true
create_backups = false
```

To add a custom template repository, place a `.toml` file inside `~/.config/termonet/repos/` (e.g. `my-rice.toml`):

```toml
name = "my-rice"
url = "https://raw.githubusercontent.com/<user>/<repo>/main"
priority = 100
enabled = true
```

</details>

<details>
<summary>Creating custom rules, templates, variables & non-destructive backups</summary>

Each custom target consists of a rule file (`~/.config/termonet/rules/<app>.toml`) and a MiniJinja template (`~/.config/termonet/templates/<app>.tmpl`):

1. **Rule file (`~/.config/termonet/rules/foot.toml`)**:
   ```toml
   version = 2
   name = "Foot"
   template = "foot.tmpl"
   output_path = "~/.config/foot/colors-termonet.ini"
   target_config = "~/.config/foot/foot.ini"
   import_line = "include = ~/.config/foot/colors-termonet.ini"
   import_position = "bottom"   # "top" | "bottom"
   comment_style = "hash"       # optional: "hash" | "blockcss" | "slash" | "doubledash"
   reload_cmd = ""
   ```

2. **Template file (`~/.config/termonet/templates/foot.tmpl`)**:
   ```ini
   # termonet-version: 4
   [colors]
   background = {{ terminal.background.strip }}
   foreground = {{ terminal.foreground.strip }}
   regular0 = {{ terminal.color0.strip }}
   regular1 = {{ terminal.color1.strip }}
   ...
   bright7 = {{ terminal.color15.strip }}
   ```

3. **Available template variables & filters**:
   - **Active theme (`active.*` / `theme.*`)** plus explicit **`dark.*`** & **`light.*`**: `primary`, `on_primary`, `primary_container`, `on_primary_container`, `secondary`, `on_secondary`, `secondary_container`, `on_secondary_container`, `tertiary`, `on_tertiary`, `error`, `on_error`, `surface`, `on_surface`, `surface_variant`, `on_surface_variant`, `background`, `on_background`, `outline`.
   - **Terminal ANSI (`terminal.*`, `terminal_dark.*`, `terminal_light.*`)**: `color0`..`color15`, `background`, `foreground`, `cursor`, plus **`seed_color`**.
   - **Color properties & filters**: `.hex` (`#adc6ff`), `.strip` (`adc6ff`), `.rgb` (`173, 198, 255`), `.r`/`.g`/`.b`, and filters `| hex`, `| strip` (`stripped`), `| rgb`, `| hypr_rgb` (`rgb(adc6ff)`), `| hypr_rgba("ee")` (`rgba(adc6ffee)`).

4. **Non-destructive safety**:
   - When `create_backups = true`, Termonet creates an immutable `<file>.original` once before the first `target_config` edit and a `<file>.bak` prior to updates; generated palette files (`output_path`) are written atomically via `safe_write()` without `.bak` clutter.
   - Imports are isolated between language-aware markers (`# >>> termonet colors >>> ... # <<< termonet colors <<<`), replacing legacy Pywal imports cleanly in place.

</details>

---

## ▸ Build from Source & Verification

Requires Linux (Wayland or X11) and Rust (`edition = "2024"`, `1.85+`):

```bash
git clone https://github.com/Denxrc/termonet.git && cd termonet
cargo build --release
install -Dm755 target/release/termonet ~/.local/bin/termonet
```

<details>
<summary>Contributor QA checks</summary>

```bash
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test --locked
```

</details>

---

## ▸ Roadmap

Released versions (`v0.1.0` – `v1.6.0`) and detailed changelogs are available on [GitHub Releases](https://github.com/Denxrc/termonet/releases).

- [ ] **v1.6.x — Ecosystem Expansion & Stabilization**
  - [x] External decoupled template registry architecture (`Denxrc/termonet-apps` + custom `repos/*.toml` + CLI/TUI/weekly sync)
  - [ ] Community templates in `termonet-apps` for GTK 3/4 (`gtk.css`), terminals (`foot`, `alacritty`, `ghostty`), notifications (`mako`, `dunst`), and `fuzzel`
  - [ ] Real-world testing and stabilization on GNOME, KDE Plasma, and X11 (`feh`, `nitrogen`)
- [ ] **v2.0 — Advanced Color Science, Presets & Visual UX**
  - [ ] Material 3 scheme variant customization (`vibrant`, `expressive`, `tonal-spot`, `content`, `fidelity`, `monochrome`) via config & `--variant`
  - [ ] Material 3 contrast adjustment (`-1.0` to `1.0`) via `--contrast`
  - [ ] Interactive seed color candidate selector and code syntax/contrast mockup preview in TUI
  - [ ] Built-in static presets (Catppuccin Mocha/Latte, Nord, Gruvbox, Tokyo Night) via `--preset <name>`
- [ ] **v3.0+ — Desktop Ecosystem & Bold Ideas**
  - [ ] XDG Desktop Portal synchronization (`org.freedesktop.appearance.color-scheme`) via D-Bus
  - [ ] Independent per-monitor theme profiles for distinct wallpapers
  - [ ] Palette export to `base16`, `json`, and `css`
  - [ ] *[Experimental / Planned]* Windows port (Win32 wallpaper query, Windows Terminal `settings.json`, PowerShell `$PROFILE`, GlazeWM/Zebar)

---

<div align="center">

<sub>MIT License · Crafted in Rust for Linux · <a href="ARCHITECTURE.md">Architecture</a> · <a href="README.ru.md">Русская версия</a></sub>

</div>
