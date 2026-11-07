//! 5-pixel height bitmap font renderer using Unicode half-blocks (▀, ▄, █).

use ratatui::style::Style;
use ratatui::text::{Line, Span};

/// 5-pixel height bitmap glyph: 5 rows represented as bitmasks, plus glyph width.
#[derive(Clone, Copy)]
struct Glyph {
    rows: [u8; 5],
    width: u8,
}

impl Glyph {
    const fn new(r0: u8, r1: u8, r2: u8, r3: u8, r4: u8, width: u8) -> Self {
        Self {
            rows: [r0, r1, r2, r3, r4],
            width,
        }
    }

    const fn get_pixel(&self, x: u8, y: u8) -> bool {
        if y >= 5 || x >= self.width {
            return false;
        }
        let bit = 1 << (self.width - 1 - x);
        (self.rows[y as usize] & bit) != 0
    }
}

/// Bitmap font engine that renders text into 3-line terminal blocks.
pub struct PixelFont;

impl PixelFont {
    /// Renders text into 3 terminal text lines using Unicode half-blocks.
    #[must_use]
    pub fn render(text: &str) -> [String; 3] {
        let mut lines = [String::new(), String::new(), String::new()];

        for ch in text.chars() {
            let glyph = Self::lookup_glyph(ch);
            for term_row in 0..3 {
                let y_top = term_row * 2;
                let y_bot = y_top + 1;

                for col in 0..glyph.width {
                    let top = glyph.get_pixel(col, y_top);
                    let bot = glyph.get_pixel(col, y_bot);

                    let block_char = match (top, bot) {
                        (true, true) => '█',
                        (true, false) => '▀',
                        (false, true) => '▄',
                        (false, false) => ' ',
                    };
                    lines[term_row as usize].push(block_char);
                }
                // Spacing between characters
                lines[term_row as usize].push(' ');
            }
        }

        lines
    }

    /// Renders text into Ratatui `Line` elements with the given `Style`.
    #[must_use]
    pub fn render_lines<'a>(text: &str, style: Style) -> Vec<Line<'a>> {
        let rendered = Self::render(text);
        rendered
            .into_iter()
            .map(|l| Line::from(vec![Span::styled(l, style)]))
            .collect()
    }

    fn lookup_glyph(c: char) -> Glyph {
        let upper = c.to_uppercase().next().unwrap_or(c);
        match upper {
            // Latin
            'A' => Glyph::new(0b0110, 0b1001, 0b1111, 0b1001, 0b1001, 4),
            'B' => Glyph::new(0b1110, 0b1001, 0b1110, 0b1001, 0b1110, 4),
            'C' => Glyph::new(0b0111, 0b1000, 0b1000, 0b1000, 0b0111, 4),
            'D' => Glyph::new(0b1110, 0b1001, 0b1001, 0b1001, 0b1110, 4),
            'E' => Glyph::new(0b1111, 0b1000, 0b1110, 0b1000, 0b1111, 4),
            'F' => Glyph::new(0b1111, 0b1000, 0b1110, 0b1000, 0b1000, 4),
            'G' => Glyph::new(0b0111, 0b1000, 0b1011, 0b1001, 0b0111, 4),
            'H' => Glyph::new(0b1001, 0b1001, 0b1111, 0b1001, 0b1001, 4),
            'I' => Glyph::new(0b111, 0b010, 0b010, 0b010, 0b111, 3),
            'J' => Glyph::new(0b0011, 0b0001, 0b0001, 0b1001, 0b0110, 4),
            'K' => Glyph::new(0b1001, 0b1010, 0b1100, 0b1010, 0b1001, 4),
            'L' => Glyph::new(0b1000, 0b1000, 0b1000, 0b1000, 0b1111, 4),
            'M' => Glyph::new(0b10001, 0b11011, 0b10101, 0b10001, 0b10001, 5),
            'N' => Glyph::new(0b1001, 0b1101, 0b1011, 0b1001, 0b1001, 4),
            'O' => Glyph::new(0b0110, 0b1001, 0b1001, 0b1001, 0b0110, 4),
            'P' => Glyph::new(0b1110, 0b1001, 0b1110, 0b1000, 0b1000, 4),
            'Q' => Glyph::new(0b0110, 0b1001, 0b1001, 0b1010, 0b0101, 4),
            'R' => Glyph::new(0b1110, 0b1001, 0b1110, 0b1010, 0b1001, 4),
            'S' => Glyph::new(0b0111, 0b1000, 0b0110, 0b0001, 0b1110, 4),
            'T' => Glyph::new(0b111, 0b010, 0b010, 0b010, 0b010, 3),
            'U' => Glyph::new(0b1001, 0b1001, 0b1001, 0b1001, 0b0110, 4),
            'V' => Glyph::new(0b1001, 0b1001, 0b1001, 0b0110, 0b0100, 4),
            'W' => Glyph::new(0b10001, 0b10001, 0b10101, 0b11011, 0b10001, 5),
            'X' => Glyph::new(0b1001, 0b1001, 0b0110, 0b1001, 0b1001, 4),
            'Y' => Glyph::new(0b1001, 0b1001, 0b0110, 0b0010, 0b0010, 4),
            'Z' => Glyph::new(0b1111, 0b0001, 0b0110, 0b1000, 0b1111, 4),

            // Cyrillic (Основные символы для заголовков Termonet)
            'А' => Glyph::new(0b0110, 0b1001, 0b1111, 0b1001, 0b1001, 4),
            'Б' => Glyph::new(0b1111, 0b1000, 0b1110, 0b1001, 0b1110, 4),
            'В' => Glyph::new(0b1110, 0b1001, 0b1110, 0b1001, 0b1110, 4),
            'Г' => Glyph::new(0b1111, 0b1000, 0b1000, 0b1000, 0b1000, 4),
            'Д' => Glyph::new(0b0110, 0b0110, 0b0110, 0b1111, 0b1001, 4),
            'Е' | 'Ё' => Glyph::new(0b1111, 0b1000, 0b1110, 0b1000, 0b1111, 4),
            'Ж' => Glyph::new(0b10101, 0b10101, 0b01110, 0b10101, 0b10101, 5),
            'З' => Glyph::new(0b1110, 0b0001, 0b0110, 0b0001, 0b1110, 4),
            'И' | 'Й' => Glyph::new(0b1001, 0b1001, 0b1011, 0b1101, 0b1001, 4),
            'К' => Glyph::new(0b1001, 0b1010, 0b1100, 0b1010, 0b1001, 4),
            'Л' => Glyph::new(0b0111, 0b1001, 0b1001, 0b1001, 0b1001, 4),
            'М' => Glyph::new(0b10001, 0b11011, 0b10101, 0b10001, 0b10001, 5),
            'Н' => Glyph::new(0b1001, 0b1001, 0b1111, 0b1001, 0b1001, 4),
            'О' => Glyph::new(0b0110, 0b1001, 0b1001, 0b1001, 0b0110, 4),
            'П' => Glyph::new(0b1111, 0b1001, 0b1001, 0b1001, 0b1001, 4),
            'Р' => Glyph::new(0b1110, 0b1001, 0b1110, 0b1000, 0b1000, 4),
            'С' => Glyph::new(0b0111, 0b1000, 0b1000, 0b1000, 0b0111, 4),
            'Т' => Glyph::new(0b111, 0b010, 0b010, 0b010, 0b010, 3),
            'У' => Glyph::new(0b1001, 0b1001, 0b0111, 0b0001, 0b1110, 4),
            'Ф' => Glyph::new(0b010, 0b111, 0b101, 0b111, 0b010, 3),
            'Х' => Glyph::new(0b1001, 0b1001, 0b0110, 0b1001, 0b1001, 4),
            'Ц' => Glyph::new(0b1001, 0b1001, 0b1001, 0b1111, 0b0001, 4),
            'Ч' => Glyph::new(0b1001, 0b1001, 0b0111, 0b0001, 0b0001, 4),
            'Ш' => Glyph::new(0b10101, 0b10101, 0b10101, 0b10101, 0b11111, 5),
            'Щ' => Glyph::new(0b10101, 0b10101, 0b10101, 0b11111, 0b00001, 5),
            'Ъ' => Glyph::new(0b1100, 0b0100, 0b0110, 0b0101, 0b0110, 4),
            'Ы' => Glyph::new(0b1010, 0b1010, 0b1110, 0b1010, 0b1110, 4),
            'Ь' => Glyph::new(0b1000, 0b1000, 0b1110, 0b1001, 0b1110, 4),
            'Э' => Glyph::new(0b1110, 0b0001, 0b0111, 0b0001, 0b1110, 4),
            'Ю' => Glyph::new(0b10110, 0b10101, 0b11101, 0b10101, 0b10110, 5),
            'Я' => Glyph::new(0b0111, 0b1001, 0b0111, 0b1001, 0b1001, 4),

            // Digits
            '0' => Glyph::new(0b0110, 0b1001, 0b1001, 0b1001, 0b0110, 4),
            '1' => Glyph::new(0b010, 0b110, 0b010, 0b010, 0b111, 3),
            '2' => Glyph::new(0b1110, 0b0001, 0b0110, 0b1000, 0b1111, 4),
            '3' => Glyph::new(0b1110, 0b0001, 0b0110, 0b0001, 0b1110, 4),
            '4' => Glyph::new(0b1001, 0b1001, 0b1111, 0b0001, 0b0001, 4),
            '5' => Glyph::new(0b1111, 0b1000, 0b1110, 0b0001, 0b1110, 4),
            '6' => Glyph::new(0b0111, 0b1000, 0b1110, 0b1001, 0b0110, 4),
            '7' => Glyph::new(0b1111, 0b0001, 0b0010, 0b0100, 0b0100, 4),
            '8' => Glyph::new(0b0110, 0b1001, 0b0110, 0b1001, 0b0110, 4),
            '9' => Glyph::new(0b0110, 0b1001, 0b0111, 0b0001, 0b1110, 4),

            // Punctuation and symbols
            '-' => Glyph::new(0b000, 0b000, 0b111, 0b000, 0b000, 3),
            ':' => Glyph::new(0b0, 0b1, 0b0, 0b1, 0b0, 1),
            '.' => Glyph::new(0b0, 0b0, 0b0, 0b0, 0b1, 1),
            '!' => Glyph::new(0b1, 0b1, 0b1, 0b0, 0b1, 1),
            ' ' => Glyph::new(0b00, 0b00, 0b00, 0b00, 0b00, 2),

            // Fallback square block
            _ => Glyph::new(0b111, 0b101, 0b101, 0b101, 0b111, 3),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_termonet() {
        let lines = PixelFont::render("TERMONET");
        assert_eq!(lines.len(), 3);
        assert!(!lines[0].is_empty());
        assert!(!lines[1].is_empty());
        assert!(!lines[2].is_empty());
    }

    #[test]
    fn test_render_cyrillic() {
        let lines = PixelFont::render("ПАЛИТРА");
        assert_eq!(lines.len(), 3);
        assert!(!lines[0].is_empty());
    }
}
