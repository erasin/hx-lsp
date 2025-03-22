use async_lsp::lsp_types::{Color, ColorInformation, Position, Range};
use ropey::Rope;

/// 颜色格式解析配置
struct ColorFormat {
    prefix: &'static str,
    parser: fn(&str) -> Option<Color>,
}

/// 支持的颜色格式配置表
const COLOR_FORMATS: &[ColorFormat] = &[
    ColorFormat {
        prefix: "srgba(",
        parser: parse_srgba,
    },
    ColorFormat {
        prefix: "srgb(",
        parser: parse_srgb,
    },
    ColorFormat {
        prefix: "rgba(",
        parser: parse_rgba,
    },
    ColorFormat {
        prefix: "rgb(",
        parser: parse_rgb,
    },
    ColorFormat {
        prefix: "hsla(",
        parser: parse_hsla,
    },
    ColorFormat {
        prefix: "hsl(",
        parser: parse_hsl,
    },
    ColorFormat {
        prefix: "hsva(",
        parser: parse_hsva,
    },
    ColorFormat {
        prefix: "hsv(",
        parser: parse_hsv,
    },
];

#[allow(dead_code)]
fn parse_color(text: &str) -> Option<Color> {
    let lower_text = text.to_lowercase();
    for format in COLOR_FORMATS.iter() {
        if lower_text.starts_with(format.prefix) {
            return (format.parser)(&lower_text);
        }
    }

    None
}

/// 提取文本中的颜色
pub fn extract_colors(doc: &Rope) -> Vec<ColorInformation> {
    let mut colors = Vec::new();
    let text_len = doc.len_chars();
    let mut pos = 0;

    while pos < text_len {
        // 优先检测十六进制颜色（特殊格式）
        if let Some((end, color)) = detect_hex_color(doc, pos) {
            push_color_info(doc, pos, end, &mut colors, color);
            pos = end;
            continue;
        }

        // 使用模式匹配检测其他格式

        // 使用模式匹配检测其他格式
        let matched = COLOR_FORMATS.iter().find_map(|format| {
            let prefix_len = format.prefix.len();
            if pos + prefix_len > text_len {
                return None;
            }

            let prefix = doc.slice(pos..pos + prefix_len).to_string();
            if prefix.eq_ignore_ascii_case(format.prefix) {
                let start = pos + prefix_len;
                find_color_closure(doc, start).and_then(|(end_pos, color_str)| {
                    let full_str = format!("{}{})", format.prefix, color_str);
                    (format.parser)(&full_str).map(|color| (end_pos, color))
                })
            } else {
                None
            }
        });

        if let Some((end_pos, color)) = matched {
            push_color_info(doc, pos, end_pos + 1, &mut colors, color);
            pos = end_pos + 1;
        } else {
            pos += 1;
        }
    }

    colors
}

/// 检测十六进制颜色格式
fn detect_hex_color(doc: &Rope, start: usize) -> Option<(usize, Color)> {
    let text_len = doc.len_chars();
    if doc.char(start) != '#' || start + 7 > text_len {
        return None;
    }

    let hex_chars = (1..=6).all(|offset| doc.char(start + offset).is_ascii_hexdigit());
    if !hex_chars {
        return None;
    }

    let hex_text: String = (0..7).map(|offset| doc.char(start + offset)).collect();
    parse_hex(&hex_text).map(|color| (start + 7, color))
}

// 查找闭合括号并返回内容
fn find_color_closure(doc: &Rope, start: usize) -> Option<(usize, String)> {
    let mut depth = 1;
    let mut i = start;
    let mut color_str = String::new();

    while i < doc.len_chars() {
        let c = doc.char(i);
        match c {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return Some((i, color_str));
                }
            }
            _ => {}
        }
        color_str.push(c);
        i += 1;
    }
    None
}

// 统一添加颜色信息
fn push_color_info(
    doc: &Rope,
    start: usize,
    end: usize,
    colors: &mut Vec<ColorInformation>,
    color: Color,
) {
    let start_line = doc.char_to_line(start);
    let start_col = start - doc.line_to_char(start_line);

    let end_line = doc.char_to_line(end);
    let end_col = end - doc.line_to_char(end_line);

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

// 解析十六进制颜色
fn parse_hex(text: &str) -> Option<Color> {
    let text = text.trim_start_matches('#');
    if text.len() != 6 {
        return None;
    }

    let parse = |range: std::ops::Range<usize>| u8::from_str_radix(&text[range], 16).ok();

    Some(Color {
        red: parse(0..2)? as f32 / 255.0,
        green: parse(2..4)? as f32 / 255.0,
        blue: parse(4..6)? as f32 / 255.0,
        alpha: 1.0,
    })
}

/// 解析颜色分量（统一处理百分比和数值）
fn parse_components(parts: &[&str], count: usize, max_values: &[f32]) -> Option<Vec<f32>> {
    if parts.len() != count {
        return None;
    }

    parts
        .iter()
        .zip(max_values)
        .map(|(&part, &max)| parse_normalized_value(part, max))
        .collect()
}

/// 通用数值解析（支持百分比和标准化）
fn parse_normalized_value(s: &str, max: f32) -> Option<f32> {
    s.trim_end_matches('%')
        .parse::<f32>()
        .ok()
        .map(|v| if s.ends_with('%') { v / 100.0 } else { v / max })
        .and_then(|v| (0.0..=1.0).contains(&v).then_some(v))
}

/// bevy color SRGBA 解析
fn parse_srgba(text: &str) -> Option<Color> {
    parse_rgb_like(text, "srgba(", 4, &[1.0, 1.0, 1.0, 1.0])
}

// bevy color SRGB 解析
fn parse_srgb(text: &str) -> Option<Color> {
    parse_rgb_like(text, "srgb(", 3, &[1.0, 1.0, 1.0])
}

// bevy color RGBA 解析
fn parse_rgba(text: &str) -> Option<Color> {
    parse_rgb_like(text, "rgba(", 4, &[1.0, 1.0, 1.0, 1.0])
}

// 解析 RGB 颜色（支持小数和范围校验）
fn parse_rgb(text: &str) -> Option<Color> {
    parse_rgb_like(text, "rgb(", 3, &[255.0, 255.0, 255.0])
}

/// 解析 rgb like
fn parse_rgb_like(text: &str, prefix: &str, length: usize, max_values: &[f32]) -> Option<Color> {
    // 参数完整性校验
    let content = text.strip_prefix(prefix)?.strip_suffix(')')?;
    let parts: Vec<&str> = content.split(',').map(|s| s.trim()).collect();
    if parts.len() != length {
        return None;
    }
    let components = parse_components(&parts, length, max_values)?;
    Some(Color {
        red: components[0],
        green: components[1],
        blue: components[2],
        alpha: if length >= 4 { components[3] } else { 1.0 },
    })
}

fn parse_hsl_hsv_like(text: &str, prefix: &str, length: usize) -> Option<Vec<f32>> {
    let content = text.strip_prefix(prefix)?.strip_suffix(')')?;
    let parts: Vec<&str> = content.split(',').map(|s| s.trim()).collect();
    if parts.len() != length {
        return None;
    }

    let hue = parts[0].parse::<f32>().ok()?.rem_euclid(360.0);
    let saturation = parse_normalized_value(parts[1], 1.0)?;
    let lightness_or_value = parse_normalized_value(parts[2], 1.0)?;

    let alpha = if length == 4 {
        parse_normalized_value(parts[3], 1.0)?
    } else {
        1.0
    };

    Some(vec![hue, saturation, lightness_or_value, alpha])
}

// bevy hsla 支持
fn parse_hsla(text: &str) -> Option<Color> {
    let components = parse_hsl_hsv_like(text, "hsla(", 4)?;
    // 转换HSL到RGB
    let (red, green, blue) = hsl_to_rgb(components[0], components[1], components[2]);
    Some(Color {
        red,
        green,
        blue,
        alpha: components[3],
    })
}

// 新增HSL解析函数
fn parse_hsl(text: &str) -> Option<Color> {
    let components = parse_hsl_hsv_like(text, "hsl(", 3)?;
    // 转换HSL到RGB
    let (red, green, blue) = hsl_to_rgb(components[0], components[1], components[2]);
    Some(Color {
        red,
        green,
        blue,
        alpha: components[3],
    })
}

// HSL转RGB算法实现
fn hsl_to_rgb(h: f32, s: f32, l: f32) -> (f32, f32, f32) {
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = l - c / 2.0;

    let (r, g, b) = match (h / 60.0).floor() as i32 % 6 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };

    (r + m, g + m, b + m)
}

// bevy hsva 支持
fn parse_hsva(text: &str) -> Option<Color> {
    let components = parse_hsl_hsv_like(text, "hsva(", 4)?;
    // 转换HSV到RGB
    let (red, green, blue) = hsv_to_rgb(components[0], components[1], components[2]);
    Some(Color {
        red,
        green,
        blue,
        alpha: components[3],
    })
}

// 新增 HSV 解析函数
fn parse_hsv(text: &str) -> Option<Color> {
    let components = parse_hsl_hsv_like(text, "hsv(", 3)?;
    // 转换HSV到RGB
    let (red, green, blue) = hsv_to_rgb(components[0], components[1], components[2]);
    Some(Color {
        red,
        green,
        blue,
        alpha: components[3],
    })
}

// HSV 到 RGB 转换算法
fn hsv_to_rgb(h: f32, s: f32, v: f32) -> (f32, f32, f32) {
    let c = v * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = v - c;

    let (r, g, b) = match h {
        h if h < 60.0 => (c, x, 0.0),
        h if h < 120.0 => (x, c, 0.0),
        h if h < 180.0 => (0.0, c, x),
        h if h < 240.0 => (0.0, x, c),
        h if h < 300.0 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };

    (r + m, g + m, b + m)
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_lsp::lsp_types::Color;
    use ropey::Rope;

    // 测试辅助函数
    fn assert_color_eq(actual: &Color, expected: (f32, f32, f32, f32)) {
        assert!(
            (actual.red - expected.0).abs() < 0.001,
            "Red mismatch: expected {}, got {}",
            expected.0,
            actual.red
        );
        assert!(
            (actual.green - expected.1).abs() < 0.001,
            "Green mismatch: expected {}, got {}",
            expected.1,
            actual.green
        );
        assert!(
            (actual.blue - expected.2).abs() < 0.001,
            "Blue mismatch: expected {}, got {}",
            expected.2,
            actual.blue
        );
        assert!(
            (actual.alpha - expected.3).abs() < 0.001,
            "Alpha mismatch: expected {}, got {}",
            expected.3,
            actual.alpha
        );
    }

    fn test_parse_func(
        func: fn(&str) -> Option<Color>,
        cases: &[(&str, Option<(f32, f32, f32, f32)>)],
    ) {
        for (input, expected) in cases {
            let result = func(&input.to_lowercase());
            println!("--> {input} {result:?} {expected:?}");
            assert_eq!(
                result.is_some(),
                expected.is_some(),
                "Failed case: {}",
                input
            );
            if let Some(expected_values) = expected {
                assert_color_eq(&result.unwrap(), *expected_values);
            }
        }
    }

    // 测试十六进制颜色
    #[test]
    fn test_hex() {
        let cases = [
            ("#ff0000", Some((1.0, 0.0, 0.0, 1.0))),
            ("#00FF00", Some((0.0, 1.0, 0.0, 1.0))),
            ("#0000ff80", None), // 无效长度
            ("#gg0000", None),   // 非法字符
        ];
        test_parse_func(parse_hex, &cases);
    }

    // 测试SRGB/SRGBA
    #[test]
    fn test_srgb() {
        let cases = [
            ("SRGB(0.0, 1.0, 0.7)", Some((0.0, 1.0, 0.7, 1.0))),
            ("srgb(1.0, 0.5, 0.3)", Some((1.0, 0.5, 0.3, 1.0))),
            ("srgba(0.5, 0.5, 0.5, 0.5)", Some((0.5, 0.5, 0.5, 0.5))),
            ("srgba(2.0, 0.0, 0.0, 1.0)", None), // 超范围
        ];
        test_parse_func(parse_srgb, &cases[0..2]);
        test_parse_func(parse_srgba, &cases[2..]);
    }

    // 测试RGB/RGBA
    #[test]
    fn test_rgb() {
        let cases = [
            ("rgb(255, 0, 0)", Some((1.0, 0.0, 0.0, 1.0))),
            ("rgb(100%, 50%, 0%)", Some((1.0, 0.5, 0.0, 1.0))),
            ("rgba(1.0, 0.5, 0.25, 0.5)", Some((1.0, 0.5, 0.25, 0.5))),
            ("rgba(300, 0, 0, 1)", None), // 超范围
        ];
        test_parse_func(parse_rgb, &cases[0..2]);
        test_parse_func(parse_rgba, &cases[2..]);
    }

    // 测试HSL/HSV
    #[test]
    fn test_hsl_hsv() {
        let hsl_cases = [
            ("hsl(0, 100%, 50%)", Some((1.0, 0.0, 0.0, 1.0))), // 红色
            ("hsl(120, 100%, 25%)", Some((0.0, 0.5, 0.0, 1.0))), // 深绿色
            ("hsl(480, 0.5, 0.5)", Some((0.25, 0.75, 0.25, 1.0))), // 色相环绕
        ];

        let hsv_cases = [
            ("hsv(0, 100%, 100%)", Some((1.0, 0.0, 0.0, 1.0))),
            ("hsv(120, 1.0, 1.0)", Some((0.0, 1.0, 0.0, 1.0))),
            ("hsv(60, 0.5, 0.8)", Some((0.8, 0.8, 0.4, 1.0))),
        ];

        test_parse_func(parse_hsl, &hsl_cases);
        test_parse_func(parse_hsv, &hsv_cases);
    }

    // 测试位置计算
    #[test]
    fn test_position_calculation() {
        let text = r#"
            #ff0000
            srgb(0.2, 0.8, 0.4)
            rgba(1.0, 0, 0, 0.5)
        "#;

        let doc = Rope::from_str(text);
        let colors = extract_colors(&doc);

        // 验证检测到的颜色数量
        assert_eq!(colors.len(), 3);

        // 验证第一个颜色（#ff0000）
        let color1 = &colors[0];
        assert_eq!(color1.range.start.line, 1);
        assert_eq!(color1.range.start.character, 12);
        assert_eq!(color1.range.end.character, 19);

        // 验证第二个颜色（srgb）
        let color2 = &colors[1];
        assert_eq!(color2.range.start.line, 2);
        assert_eq!(color2.range.start.character, 12);
    }

    // 测试边界条件
    #[test]
    fn test_edge_cases() {
        // 最小/最大值测试
        let cases = [
            ("rgb(0, 0, 0)", Some((0.0, 0.0, 0.0, 1.0))),
            ("rgb(255, 255, 255)", Some((1.0, 1.0, 1.0, 1.0))),
            ("hsl(0, 0%, 0%)", Some((0.0, 0.0, 0.0, 1.0))),
            ("hsv(0, 0%, 0%)", Some((0.0, 0.0, 0.0, 1.0))),
        ];
        test_parse_func(parse_rgb, &cases[0..2]);
        test_parse_func(parse_hsl, &cases[2..3]);
        test_parse_func(parse_hsv, &cases[3..]);
    }

    // 测试无效输入
    #[test]
    fn test_invalid_inputs() {
        let cases = [
            "rgb(255.1, 0, 0)",    // 超范围
            "hsl(360, 101%, 50%)", // 饱和度超范围
            "hsv(0, -0.1, 1)",     // 负值
            "rgba(255, 0, 0)",     // 参数不足
            "srgb(invalid, 0, 0)", // 非法字符
        ];

        for input in cases {
            assert!(
                parse_color(&input).is_none(),
                "Should reject invalid input: {}",
                input
            );
        }
    }

    // 测试混合格式文档解析
    #[test]
    fn test_mixed_formats() {
        let text = r#"
            /* 颜色定义 */
            #ff0000                  // 十六进制
            srgb(0.2, 0.8, 0.4)      // sRGB
            rgba(1.0, 0, 0, 0.5)      // RGBA
            hsl(180, 50%, 50%)        // HSL
            hsv(300, 1.0, 1.0)        // HSV
            
            /* 无效颜色 */
            #gggggg
            rgb(256, 0, 0)
            hsl(360, 150%, 50%)
        "#;

        let doc = Rope::from_str(text);
        let colors = extract_colors(&doc);

        // 验证有效颜色数量
        assert_eq!(colors.len(), 5, "Should detect 5 valid colors");

        // 验证颜色类型分布
        let type_counts = colors.iter().fold([0; 5], |mut counts, ci| {
            match ci.color {
                c if c.alpha < 1.0 => counts[0] += 1,                  // RGBA
                c if c.red == 1.0 && c.green == 0.0 => counts[1] += 1, // 红色系
                c if c.blue > 0.5 => counts[2] += 1,                   // 蓝色系
                _ => counts[3] += 1,
            }
            counts
        });

        assert_eq!(type_counts[0], 1, "Should contain 1 RGBA color");
        assert_eq!(type_counts[1], 2, "Should contain 2 red colors");
    }
}
