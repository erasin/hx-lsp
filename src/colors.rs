use lsp_types::{Color, ColorInformation, Position, Range};
use ropey::Rope;

pub fn extract_colors(doc: &Rope) -> Vec<ColorInformation> {
    let mut colors = Vec::new();
    let text_len = doc.len_chars();

    let mut i = 0;
    while i + 7 <= text_len {
        if doc.char(i) == '#' {
            // Check if the next 6 characters are valid hex
            if (1..=6).all(|offset| {
                let c = doc.char(i + offset);
                c.is_ascii_hexdigit()
            }) {
                let hex_text: String = (0..7).map(|offset| doc.char(i + offset)).collect();
                if let Some(color) = parse_color(&hex_text) {
                    let start_idx = i;
                    let end_idx = i + 7;

                    let start_line = doc.char_to_line(start_idx);
                    let start_col = start_idx - doc.line_to_char(start_line);

                    let end_line = doc.char_to_line(end_idx);
                    let end_col = end_idx - doc.line_to_char(end_line);

                    colors.push(ColorInformation {
                        range: Range {
                            start: Position {
                                line: start_line as u32,
                                character: start_col as u32,
                            },
                            end: Position {
                                line: end_line as u32,
                                character: end_col as u32,
                            },
                        },
                        color,
                    });
                }
                i += 7; // Skip past the matched color
                continue;
            }
        }
        i += 1;
    }

    colors
}

pub fn parse_color(text: &str) -> Option<Color> {
    if text.len() == 7 && text.starts_with('#') {
        if let (Ok(r), Ok(g), Ok(b)) = (
            u8::from_str_radix(&text[1..3], 16),
            u8::from_str_radix(&text[3..5], 16),
            u8::from_str_radix(&text[5..7], 16),
        ) {
            return Some(Color {
                red: r as f32 / 255.0,
                green: g as f32 / 255.0,
                blue: b as f32 / 255.0,
                alpha: 1.0,
            });
        }
    }
    None
}
