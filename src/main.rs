//! Termonet binary entry point.

use std::env;
use termonet::core::GeneratedPalette;
use termonet::error::Result;

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();

    println!("========================================");
    println!("  Termonet v{} (Core Engine Active)", env!("CARGO_PKG_VERSION"));
    println!("========================================\n");

    if args.len() > 1 {
        let image_path = &args[1];
        println!("Analyzing wallpaper: {}", image_path);

        let palette = GeneratedPalette::from_image_path(image_path)?;

        println!("\n[Seed Dominant Color]");
        print_color_swatch("Seed", &palette.seed_color);

        println!("\n[Material You Roles (Dark Scheme)]");
        print_color_swatch("Primary", &palette.dark.primary);
        print_color_swatch("Secondary", &palette.dark.secondary);
        print_color_swatch("Tertiary", &palette.dark.tertiary);
        print_color_swatch("Surface", &palette.dark.surface);
        print_color_swatch("Background", &palette.dark.background);

        println!("\n[Terminal ANSI Palette (16 colors)]");
        let t = &palette.terminal;
        print_color_swatch("color0 (Black)", &t.color0);
        print_color_swatch("color1 (Red)", &t.color1);
        print_color_swatch("color2 (Green)", &t.color2);
        print_color_swatch("color3 (Yellow)", &t.color3);
        print_color_swatch("color4 (Blue)", &t.color4);
        print_color_swatch("color5 (Magenta)", &t.color5);
        print_color_swatch("color6 (Cyan)", &t.color6);
        print_color_swatch("color7 (White)", &t.color7);
        print_color_swatch("color8 (Bright Black)", &t.color8);
        print_color_swatch("color9 (Bright Red)", &t.color9);
        print_color_swatch("color10 (Bright Green)", &t.color10);
        print_color_swatch("color11 (Bright Yellow)", &t.color11);
        print_color_swatch("color12 (Bright Blue)", &t.color12);
        print_color_swatch("color13 (Bright Magenta)", &t.color13);
        print_color_swatch("color14 (Bright Cyan)", &t.color14);
        print_color_swatch("color15 (Bright White)", &t.color15);
    } else {
        println!("Usage: termonet <path-to-image>");
        println!("Running synthetic self-test...");
        let palette = GeneratedPalette::from_seed_color(material_colors::color::Rgb::new(0x2e, 0x7d, 0x32));
        println!("Generated sample palette with seed: {}", palette.seed_color.to_hex());
    }

    Ok(())
}

fn print_color_swatch(label: &str, color: &termonet::core::RgbColor) {
    // Truecolor terminal swatch: 48;2;R;G;B
    println!(
        "  \x1b[48;2;{r};{g};{b}m    \x1b[0m {label:<22} {hex}  (rgb: {rgb})",
        r = color.r,
        g = color.g,
        b = color.b,
        hex = color.to_hex(),
        rgb = color.to_rgb_string()
    );
}
