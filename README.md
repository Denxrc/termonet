<div align="center">

```text
▀█▀ █▀▀▀ █▀▀█ █▀▄▀█ █▀▀█ █▄ █ █▀▀▀ ▀█▀
 █  █▀▀  █▄▄▀ █ █ █ █  █ █ ▀█ █▀▀   █ 
 █  █▄▄▄ █  █ █   █ █▄▄█ █  █ █▄▄▄  █ 
```

# Termonet

**Fast, dynamic Material You system theme and palette generator for Linux**

[![Rust Edition 2024](https://img.shields.io/badge/rust-edition_2024-orange.svg)](https://www.rust-lang.org/)
[![Version](https://img.shields.io/badge/version-1.2.0-blue.svg)](Cargo.toml)
[![License: MIT](https://img.shields.io/badge/license-MIT-green.svg)](LICENSE)
[![Platform](https://img.shields.io/badge/platform-Linux%20(Wayland%20%2F%20X11)-lightgrey.svg)]()
[![XERJ Corpus Hub](https://hub.xerj.org/contributors/Denxrc/badge.svg)](https://hub.xerj.org/contributors/Denxrc)

[English](README.md) | [Русский](README.ru.md)

</div>

---

## 🌟 Overview

> [!NOTE]
> **XERJ Corpus Hub**: Termonet is indexed in the [XERJ Corpus Hub](https://hub.xerj.org/contributors/Denxrc) as a worked example for Rust TUI architecture, path handling, and dynamic theming.

**Termonet** is a lightweight, zero-privilege, dynamic theme engine and palette generator for Linux desktop environments (Wayland and X11), written in **Rust** (edition 2024).

It extracts dominant hues from your desktop wallpapers and calculates authentic, mathematically harmonized **Google Material You (Material 3)** color roles and 16-color ANSI terminal palettes. Termonet features a clean, responsive Terminal User Interface (TUI) powered by `ratatui` with an interactive setup wizard, real-time manual wallpaper input, and safe configuration management.

---

## ✨ Features

- 🎨 **Authentic Material You Engine**: Powered by `material-colors` using Google MCU's `Variant::Content` algorithm, faithfully preserving atmospheric tones and muted lighting without unnatural oversaturation.
- ⚡ **Blazingly Fast Image Processing**: In-memory 128×128 downsampling via `image`, achieving sub-second palette generation (~40 ms) on images of any resolution, including ultra-high 8K wallpapers and animated GIFs (first frame extraction).
- 🖥️ **Responsive TUI with Signature 5px Pixel Font**:
  - Custom 5px bitmap pixel logo rendered with Unicode half-blocks (`█`, `▀`, `▄`) with full Cyrillic and Latin support.
  - Automatically switches between pixel header and compact mode on smaller terminal windows.
- 🧙 **Interactive First-Run Wizard**:
  - Full localization in **English** and **Russian**.
  - Seamless setup for TUI themes (Dark / Light / System), font styles, display servers (Wayland / X11), and wallpaper backends.
  - Can be invoked anytime directly in-app via `s` or `F2`.
- 🖼️ **On-the-Fly Wallpaper Changing**:
  - Press `w` on the Palette screen to type or paste any image path in a modal popup.
  - Automatic `~` home directory expansion (`~/Pictures/wall.png` ➔ `/home/user/Pictures/wall.png`).
  - Real-time palette recalculation and automatic persistence to configuration.
- 📝 **Self-Documenting Configuration**:
  - Saves a clean, human-readable `~/.config/termonet/config.toml` complete with descriptive comments and allowed values for all options.
- 🛡️ **Zero-Privilege & Read-Only Safety**:
  - Operates entirely without root permissions (`sudo`).
  - All staging and validations occur in memory prior to disk writes.

---

## 🧭 TUI Navigation & Controls

Termonet organizes its workflow into 4 focused steps:

| Step | Screen | Description |
| :---: | :--- | :--- |
| **1** | **Palette** | Preview Material You color roles and ANSI terminal swatches; press `w` to enter a new wallpaper path |
| **2** | **Programs** | Toggle supported applications (Kitty, Waybar, Rofi, Hyprland) or custom targets |
| **3** | **Hooks** | Configure application reload hooks and commands (e.g. `hyprctl reload`, `killall -SIGUSR2 waybar`) |
| **4** | **Apply** | Review planned file writes and service reload commands before final execution |

### Keyboard Shortcuts

| Shortcut | Action |
| :--- | :--- |
| `Left` / `Right` (`h` / `l`) | Switch between steps / windows (1..4) |
| `1`, `2`, `3`, `4` | Jump directly to step 1..4 |
| `Up` / `Down` (`k` / `j`) | Navigate items in lists |
| `Space` | Toggle selected program or hook checkbox |
| `w` | Open manual wallpaper path input dialog (in Step 1) |
| `r` | Auto-detect wallpaper from active wallpaper daemon (in Step 1) |
| `s` / `F2` | Open setup / settings wizard |
| `Esc` | Close input modal / cancel settings wizard |
| `q` | Quit Termonet |

---

## 🚀 Installation & Building

### Prerequisites

- A Linux system (Wayland or X11)
- Rust toolchain (1.85+ with edition 2024 support):
  ```bash
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
  ```

### Build from Source

```bash
git clone https://github.com/Denxrc/termonet.git
cd termonet
cargo build --release
```

The compiled binary will be located at `target/release/termonet`. You can install it to your user path:

```bash
install -Dm755 target/release/termonet ~/.local/bin/termonet
```

---

## 💻 Usage

```bash
# Launch interactive TUI (auto-detects active wallpaper from running daemon or config)
termonet

# Headless mode: apply theme across all enabled programs from active wallpaper and exit
termonet --apply

# Force headless re-application (bypassing the state cache)
termonet --apply --force

# Apply a specific wallpaper image in headless mode
termonet --apply -w ~/Pictures/Wallpapers/forest.png

# Launch TUI with a specific wallpaper image
termonet -w ~/Pictures/Wallpapers/forest.png

# Launch directly into the setup wizard
termonet --init

# Synchronize default rules and templates to latest version
termonet --reset-rules
```

### CLI Options

```text
Usage: termonet [OPTIONS] [WALLPAPER_PATH]

Headless / Automation Options:
  -a, --apply              Headless mode: apply theme from active wallpaper and exit
  -f, --force              Force re-application ignoring cached wallpaper state
      --no-backup          Disable creation of backup copies (.bak, .original)
      --backup             Enable creation of backup copies (.bak, .original)

General Options:
  -w, --wallpaper <PATH>   Path to wallpaper image (PNG, JPG, GIF, WebP)
  -i, --wizard, --init     Launch first-run configuration wizard
      --reset-rules        Force reset default rules and templates to latest
  -h, --help               Print help information
```

---

## ⚙️ Configuration

Termonet configuration is stored at `~/.config/termonet/config.toml`. The file is fully commented:

```toml
# ==============================================================================
# Termonet Configuration
# Path: ~/.config/termonet/config.toml
# ==============================================================================

# Interface language ("ru" | "en")
language = "en"

# Visual color theme variant for TUI ("dark" | "light" | "system")
tui_theme = "dark"

# Header font rendering mode ("pixel" | "standard")
font_mode = "pixel"

# Display server protocol ("wayland" | "x11")
display_server = "wayland"

# Wallpaper detection backend
# Options: "auto", "swww", "hyprpaper", "awww", "wpaperd", "swaybg", "gnome", "kde", "feh", "nitrogen", "manual"
wallpaper_backend = "auto"

# Primary monitor name (uncomment to specify)
# primary_monitor = "eDP-1"

# Manual wallpaper path (uncomment to specify)
# manual_wallpaper_path = "~/Pictures/wallpapers/forest.png"

# Target applications
enabled_programs = [
    "kitty",
    "waybar",
    "rofi",
    "hyprland"
]

# Safety & backups (set to false for clean direct overwrites without .bak clutter)
create_backups = false
```

---

## 🎨 Creating Custom Rules & Templates

Termonet is designed from the ground up to be fully modular: you can add dynamic theme generation for **any** application (terminal emulators, status bars, launchers, editors) simply by adding two files in `~/.config/termonet/`.

### 1. Theme Architecture

Each application integration consists of two files:
1. **Rule File** (`~/.config/termonet/rules/<app>.toml`): Defines metadata, target file paths, position, and reload commands.
2. **Template File** (`~/.config/termonet/templates/<app>.tmpl`): MiniJinja template rendered with Material You color roles.

### 2. Rule File Specification (`rules/<name>.toml`)

```toml
# Rule schema version (enables automatic default migrations)
version = 2

# Application display name (shown in TUI and Wizard)
name = "Foot"

# Template file located in ~/.config/termonet/templates/
template = "foot.tmpl"

# Destination path where rendered colors will be written
output_path = "~/.config/foot/colors-termonet.ini"

# (Optional) Main configuration file where import directive is injected
target_config = "~/.config/foot/foot.ini"

# (Optional) Import directive line injected into target_config
import_line = "include = ~/.config/foot/colors-termonet.ini"

# Insertion position ("top" or "bottom")
import_position = "bottom"

# (Optional) Comment style override ("hash" | "blockcss" | "slash" | "doubledash")
# If omitted, auto-detected from target_config extension
comment_style = "hash"

# (Optional) Shell command to reload colors without restarting the app
reload_cmd = ""
```

### 3. Template Variables & Filters (`templates/<name>.tmpl`)

Templates have access to all **Google Material You (Material 3)** roles (in both Dark and Light variants) as well as the harmonized 16-color ANSI terminal palette.

#### Available Context Variables

- **Material You Roles (`dark.*` and `light.*`)**:
  - `dark.primary`, `dark.on_primary`, `dark.primary_container`, `dark.on_primary_container`
  - `dark.secondary`, `dark.on_secondary`, `dark.secondary_container`, `dark.on_secondary_container`
  - `dark.tertiary`, `dark.on_tertiary`
  - `dark.surface`, `dark.on_surface`, `dark.surface_variant`, `dark.on_surface_variant`
  - `dark.background`, `dark.on_background`, `dark.outline`
- **Terminal ANSI Palette (`terminal.*`)**:
  - `terminal.color0` ... `terminal.color15`
  - `terminal.background`, `terminal.foreground`, `terminal.cursor`
- **Wallpaper Seed Color**: `seed_color`

#### Color Formatting Properties

Every color object exposes direct properties:
- `.hex`: Lowercase hex with leading hash (`#adc6ff`)
- `.strip`: Lowercase hex without hash (`adc6ff`)
- `.rgb`: Comma-separated channels (`173, 198, 255`)
- `.r`, `.g`, `.b`: Individual 8-bit integer channels (`0..255`)

#### Built-in MiniJinja Filters

- `{{ dark.primary | hex }}` ➔ Guaranteed `#adc6ff`
- `{{ dark.primary | strip }}` (or `stripped`) ➔ `adc6ff`
- `{{ dark.primary | rgb }}` ➔ `173, 198, 255`
- `{{ dark.primary | hypr_rgb }}` ➔ `rgb(adc6ff)`
- `{{ dark.primary | hypr_rgba("ee") }}` ➔ `rgba(adc6ffee)`

### 4. Practical Example: Adding Foot Terminal

1. Create `~/.config/termonet/rules/foot.toml`:
   ```toml
   version = 2
   name = "Foot"
   template = "foot.tmpl"
   output_path = "~/.config/foot/colors-termonet.ini"
   target_config = "~/.config/foot/foot.ini"
   import_line = "include = ~/.config/foot/colors-termonet.ini"
   import_position = "bottom"
   ```

2. Create `~/.config/termonet/templates/foot.tmpl`:
   ```ini
   # Termonet Foot Terminal Theme
   # termonet-version: 2

   [colors]
   background = {{ terminal.background.strip }}
   foreground = {{ terminal.foreground.strip }}
   regular0 = {{ terminal.color0.strip }}
   regular1 = {{ terminal.color1.strip }}
   regular2 = {{ terminal.color2.strip }}
   regular3 = {{ terminal.color3.strip }}
   regular4 = {{ terminal.color4.strip }}
   regular5 = {{ terminal.color5.strip }}
   regular6 = {{ terminal.color6.strip }}
   regular7 = {{ terminal.color7.strip }}
   bright0 = {{ terminal.color8.strip }}
   bright1 = {{ terminal.color9.strip }}
   bright2 = {{ terminal.color10.strip }}
   bright3 = {{ terminal.color11.strip }}
   bright4 = {{ terminal.color12.strip }}
   bright5 = {{ terminal.color13.strip }}
   bright6 = {{ terminal.color14.strip }}
   bright7 = {{ terminal.color15.strip }}
   ```

3. Launch `termonet` — your new application will immediately appear in the step 2 list!

### 5. Non-Destructive Safety & Auto-Backups

Termonet guarantees configuration safety:
- **Master Backup (`.original`)**: Created once on the very first injection and **never overwritten**, ensuring your untouched original configs can always be restored.
- **Safety Backup (`.bak`)**: Created prior to every file write.
- **Managed Marker Blocks**: Import lines are wrapped in recognizable comments:
  ```ini
  # >>> termonet colors >>>
  include colors-termonet.conf
  # <<< termonet colors <<<
  ```
  If a legacy Pywal import was present, Termonet replaces it cleanly in-place without duplicating lines.
- **Automatic Asset Sync**: Force-sync or upgrade embedded defaults at any time:
  ```bash
  termonet --update-rules
  ```

---

## 🗺️ Roadmap

- [x] **v0.x (Prototype & Core Base)**:
  - [x] Responsive TUI on `ratatui` with 5px pixel font logo
  - [x] Material You generation via `material-colors` (`Variant::Content`)
  - [x] First-run setup wizard and human-readable TOML persistence
  - [x] Bilingual interface (Russian & English)
  - [x] In-app manual wallpaper path input and auto-loading
- [x] **v1.x (Full Wayland Release)**:
  - [x] MiniJinja template engine with custom filters and TrueColor / ANSI roles
  - [x] Built-in out-of-the-box rules and templates (Kitty, Waybar, Rofi, Hyprland Lua)
  - [x] Safe marker injection, legacy theme (Pywal) replacement, and `.original` master backups
  - [x] Two-tier asset versioning and automatic migration (`--update-rules`)
  - [x] Automatic Wayland wallpaper extraction (`awww`, `swww`, `hyprpaper`, `wpaperd`, `swaybg`) and DE (`GNOME`, `KDE`)
  - [x] Multi-monitor duplicate filter with primary monitor selection (`primary_monitor`)
  - [x] Headless CLI auto-apply mode (`--apply`, `--force`) with idle state caching
- [ ] **v2.x (Ecosystem Expansion)**:
  - [ ] X11 auto-detection (`feh`, `nitrogen`)
  - [ ] On-the-fly CLI color scheme overrides
  - [ ] Expanded community template library

---

## 📄 License

This project is licensed under the [MIT License](LICENSE).
