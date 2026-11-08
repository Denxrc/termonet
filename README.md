<div align="center">

```text
 █▀█ █▀▀ █▀█ █▄█ █▀█ █▀█ █▀▀ █▀█
  █  █▀▀ █▀▄ █ █ █ █ █ █ █▀▀  █ 
```

# Termonet

**Fast, dynamic Material You system theme and palette generator for Linux**

[![Rust Edition 2024](https://img.shields.io/badge/rust-edition_2024-orange.svg)](https://www.rust-lang.org/)
[![Version](https://img.shields.io/badge/version-0.3.0-blue.svg)](Cargo.toml)
[![License: MIT](https://img.shields.io/badge/license-MIT-green.svg)](LICENSE)
[![Platform](https://img.shields.io/badge/platform-Linux%20(Wayland%20%2F%20X11)-lightgrey.svg)]()

[English](README.md) | [Русский](README.ru.md)

</div>

---

## 🌟 Overview

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
| **2** | **Programs** | Toggle supported applications (Kitty, Waybar, Rofi, Hyprland, Alacritty, Dunst, Sway) |
| **3** | **Hooks** | Configure application reload hooks and commands (e.g. `hyprctl reload`, `killall -SIGUSR2 waybar`) |
| **4** | **Apply** | Review planned file writes and service reload commands before final execution |

### Keyboard Shortcuts

| Shortcut | Action |
| :--- | :--- |
| `Tab` / `Shift+Tab` | Switch to next / previous step |
| `1`, `2`, `3`, `4` | Jump directly to step 1..4 |
| `Up` / `Down` (`k` / `j`) | Navigate items in lists |
| `Space` | Toggle selected program or hook checkbox |
| `w` | Open manual wallpaper path input dialog (in Step 1) |
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
# Launch interactive TUI (auto-detects wallpaper from config or launches wizard on first run)
termonet

# Launch with a specific wallpaper image
termonet -w ~/Pictures/Wallpapers/forest.png

# Launch directly into the setup wizard
termonet --init
```

### CLI Options

```text
Usage: termonet [OPTIONS] [WALLPAPER_PATH]

Options:
  -w, --wallpaper <PATH>   Path to wallpaper image (PNG, JPG, GIF, WebP)
  -i, --wizard, --init     Launch first-run configuration wizard
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
```

---

## 🗺️ Roadmap

- [x] **v0.x (Prototype & Core Base)**:
  - [x] Responsive TUI on `ratatui` with 5px pixel font logo
  - [x] Material You generation via `material-colors` (`Variant::Content`)
  - [x] First-run setup wizard and human-readable TOML persistence
  - [x] Bilingual interface (Russian & English)
  - [x] In-app manual wallpaper path input and auto-loading
- [ ] **v1.x (Full Wayland Release)**:
  - [ ] Template engine integration (`minijinja`) and bundled templates unpacking
  - [ ] Automatic Wayland wallpaper extraction (`swww`, `hyprpaper`, `awww`, `wpaperd`, `swaybg`, `GNOME`, `KDE`)
  - [ ] Multi-monitor duplicate filter with primary monitor selection
  - [ ] Headless CLI auto-apply flag (`--apply`) for systemd timers and scripts
- [ ] **v2.x (Ecosystem Expansion)**:
  - [ ] X11 auto-detection (`feh`, `nitrogen`)
  - [ ] On-the-fly CLI color scheme overrides
  - [ ] Expanded community template library

---

## 📄 License

This project is licensed under the [MIT License](LICENSE).
