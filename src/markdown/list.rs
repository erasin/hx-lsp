use std::collections::HashMap;

use async_lsp::lsp_types::{Position, Range, TextEdit};
use comrak::{Arena, ComrakOptions, ExtensionOptions, parse_document};
use ropey::{Rope, RopeSlice};

/// 列表转换类型枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum ListType {
    Ordered,         // 有序列表 (1., 2., 3.)
    Unordered,       // 无序列表 (-, *, +)
    TaskList,        // 任务列表 (- [ ])
    TaskListChecked, // 任务列表已选中 (- [x])
    UnorderedToTask, // 无序列表 → 任务列表 (未选中)
    OrderedToTask,   // 有序列表 → 任务列表
    TaskToUnordered, // 任务列表 → 无序列表
    ToggleTask,      // 切换任务列表 checkbox 状态
}

/// 检测列表类型的返回值
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DetectedListType {
    None,
    Ordered,
    Unordered,
    TaskUnchecked,
    TaskChecked,
}

fn get_comrak_options() -> ComrakOptions<'static> {
    ComrakOptions {
        extension: ExtensionOptions {
            tasklist: true,
            strikethrough: false,
            tagfilter: false,
            table: false,
            autolink: false,
            superscript: false,
            header_ids: None,
            footnotes: false,
            description_lists: false,
            front_matter_delimiter: None,
            multiline_block_quotes: false,
            alerts: false,
            math_dollars: false,
            math_code: false,
            wikilinks_title_after_pipe: false,
            wikilinks_title_before_pipe: false,
            underline: false,
            subscript: false,
            spoiler: false,
            greentext: false,
            image_url_rewriter: None,
            link_url_rewriter: None,
        },
        ..Default::default()
    }
}

/// 检测当前列表类型（基于行首标识字符串匹配）
/// 多行时验证所有非空行类型一致
pub fn detect_list_type(rope: RopeSlice) -> DetectedListType {
    let mut detected_type: Option<DetectedListType> = None;

    for line in rope.lines() {
        let line_str = line.to_string();
        let trimmed = line_str.trim();

        if trimmed.is_empty() {
            continue;
        }

        let indent = line.chars().take_while(|c| c.is_whitespace()).count();
        let content = &line_str[indent..];

        let current_type = detect_line_list_type(content);

        match (&detected_type, &current_type) {
            (None, _) => detected_type = Some(current_type),
            (Some(prev), curr) if prev == curr => continue,
            _ => return DetectedListType::None,
        }
    }

    detected_type.unwrap_or(DetectedListType::None)
}

/// 检测单行是否是列表类型
pub fn detect_line_list_type(content: &str) -> DetectedListType {
    if content.starts_with("- [ ] ") {
        DetectedListType::TaskUnchecked
    } else if content.starts_with("- [x] ") {
        DetectedListType::TaskChecked
    } else if content.starts_with("- ") || content.starts_with("* ") || content.starts_with("+ ") {
        DetectedListType::Unordered
    } else if let Some(c) = content.chars().next()
        && c.is_ascii_digit()
    {
        let chars: Vec<char> = content.chars().collect();
        if let Some(dot_pos) = chars.iter().rposition(|&ch| ch == '.')
            && dot_pos > 0
            && chars[dot_pos - 1].is_ascii_digit()
            && dot_pos + 1 < chars.len()
            && chars[dot_pos + 1] == ' '
        {
            DetectedListType::Ordered
        } else {
            DetectedListType::None
        }
    } else {
        DetectedListType::None
    }
}

/// 检测指定行是否是任务列表
pub fn is_task_line(doc: &Rope, line: u32) -> bool {
    if let Some(line_slice) = doc.get_line(line as usize) {
        let line_str = line_slice.to_string();
        let indent = line_slice.chars().take_while(|c| c.is_whitespace()).count();
        let content = &line_str[indent..];
        content.starts_with("- [ ] ") || content.starts_with("- [x] ")
    } else {
        false
    }
}

/// 将选中的 Markdown 文本转换为指定类型的列表
pub fn convert_to_list(
    rope: RopeSlice,
    range: Range,
    conversion_type: ListType,
) -> Option<Vec<TextEdit>> {
    let arena = Arena::new();
    let _root = parse_document(&arena, &rope.to_string(), &get_comrak_options());

    let detected = detect_list_type(rope);

    match conversion_type {
        ListType::Ordered
        | ListType::Unordered
        | ListType::TaskList
        | ListType::TaskListChecked => {
            if detected != DetectedListType::None {
                return None;
            }
            convert_non_list_to_list(rope, range, conversion_type)
        }
        ListType::UnorderedToTask => {
            if detected != DetectedListType::Unordered {
                return None;
            }
            convert_unordered_to_task(rope, range, false)
        }
        ListType::OrderedToTask => {
            if detected != DetectedListType::Ordered {
                return None;
            }
            convert_ordered_to_task(rope, range)
        }
        ListType::TaskToUnordered => {
            if detected != DetectedListType::TaskUnchecked
                && detected != DetectedListType::TaskChecked
            {
                return None;
            }
            convert_task_to_unordered(rope, range)
        }
        ListType::ToggleTask => {
            if detected != DetectedListType::TaskUnchecked
                && detected != DetectedListType::TaskChecked
            {
                return None;
            }
            toggle_task_state(rope, range)
        }
    }
}

/// 非列表文本转换为列表
fn convert_non_list_to_list(
    rope: RopeSlice,
    range: Range,
    conversion_type: ListType,
) -> Option<Vec<TextEdit>> {
    let mut counters: HashMap<usize, u32> = HashMap::new();
    let mut current_levels = Vec::new();

    let edits: Vec<TextEdit> = rope
        .lines()
        .enumerate()
        .filter_map(|(index, line)| {
            let indent = line.chars().take_while(|c| c.is_whitespace()).count();

            if line.to_string().trim().is_empty() {
                current_levels.clear();
                return None;
            }

            while !current_levels.is_empty() && *current_levels.last().unwrap() > indent {
                current_levels.pop();
            }

            if current_levels.last() != Some(&indent) {
                current_levels.push(indent);
            }

            let prefix = match conversion_type {
                ListType::Ordered => {
                    let level = current_levels.len().saturating_sub(1);
                    let counter = counters.entry(level).or_insert(0);
                    *counter += 1;

                    for l in (level + 1).. {
                        if counters.remove(&l).is_none() {
                            break;
                        }
                    }

                    let prefix = (0..=level)
                        .map(|l| counters.get(&l).unwrap_or(&1).to_string())
                        .collect::<Vec<_>>()
                        .join(".");

                    format!("{prefix}. ")
                }
                ListType::Unordered => "- ".to_string(),
                ListType::TaskList => "- [ ] ".to_string(),
                ListType::TaskListChecked => "- [x] ".to_string(),
                _ => return None,
            };

            let insert_pos = Position {
                line: range.start.line + index as u32,
                character: indent as u32,
            };

            Some(TextEdit {
                range: Range::new(insert_pos, insert_pos),
                new_text: prefix,
            })
        })
        .collect();

    (!edits.is_empty()).then_some(edits)
}

/// 无序列表转换为任务列表
fn convert_unordered_to_task(
    rope: RopeSlice,
    range: Range,
    checked: bool,
) -> Option<Vec<TextEdit>> {
    let prefix = if checked { "- [x] " } else { "- [ ] " };

    let edits: Vec<TextEdit> = rope
        .lines()
        .enumerate()
        .filter_map(|(index, line)| {
            let line_str = line.to_string();
            let trimmed = line_str.trim();

            if trimmed.is_empty() {
                return None;
            }

            let indent = line.chars().take_while(|c| c.is_whitespace()).count();
            let content_after_indent = &line_str[indent..];

            let (prefix_len, new_prefix) = if !content_after_indent.starts_with("- [ ] ")
                && !content_after_indent.starts_with("- [x] ")
                && (content_after_indent.starts_with("- ")
                    || content_after_indent.starts_with("* ")
                    || content_after_indent.starts_with("+ "))
            {
                (2, prefix.to_string())
            } else {
                return None;
            };

            let start_pos = Position {
                line: range.start.line + index as u32,
                character: indent as u32,
            };
            let end_pos = Position {
                line: range.start.line + index as u32,
                character: (indent + prefix_len) as u32,
            };

            Some(TextEdit {
                range: Range::new(start_pos, end_pos),
                new_text: new_prefix,
            })
        })
        .collect();

    (!edits.is_empty()).then_some(edits)
}

/// 任务列表转换为无序列表
fn convert_task_to_unordered(rope: RopeSlice, range: Range) -> Option<Vec<TextEdit>> {
    let edits: Vec<TextEdit> = rope
        .lines()
        .enumerate()
        .filter_map(|(index, line)| {
            let line_str = line.to_string();
            let trimmed = line_str.trim();

            if trimmed.is_empty() {
                return None;
            }

            let indent = line.chars().take_while(|c| c.is_whitespace()).count();
            let content_after_indent = &line_str[indent..];

            let prefix_len = if content_after_indent.starts_with("- [ ] ")
                || content_after_indent.starts_with("- [x] ")
            {
                6
            } else {
                return None;
            };

            let start_pos = Position {
                line: range.start.line + index as u32,
                character: indent as u32,
            };
            let end_pos = Position {
                line: range.start.line + index as u32,
                character: (indent + prefix_len) as u32,
            };

            Some(TextEdit {
                range: Range::new(start_pos, end_pos),
                new_text: "- ".to_string(),
            })
        })
        .collect();

    (!edits.is_empty()).then_some(edits)
}

/// 有序列表转换为任务列表
fn convert_ordered_to_task(rope: RopeSlice, range: Range) -> Option<Vec<TextEdit>> {
    let edits: Vec<TextEdit> = rope
        .lines()
        .enumerate()
        .filter_map(|(index, line)| {
            let line_str = line.to_string();
            let trimmed = line_str.trim();

            if trimmed.is_empty() {
                return None;
            }

            let indent = line.chars().take_while(|c| c.is_whitespace()).count();
            let content_after_indent = &line_str[indent..];

            let chars: Vec<char> = content_after_indent.chars().collect();
            if chars.is_empty() || !chars[0].is_ascii_digit() {
                return None;
            }

            if content_after_indent.starts_with("- [ ] ")
                || content_after_indent.starts_with("- [x] ")
            {
                return None;
            }

            if let Some(dot_pos) = chars.iter().rposition(|&ch| ch == '.')
                && dot_pos > 0
                && chars[dot_pos - 1].is_ascii_digit()
            {
                let after_dot_pos = dot_pos + 1;
                if after_dot_pos < chars.len() && chars[after_dot_pos] == ' ' {
                    let prefix_len = after_dot_pos + 1;
                    let start_pos = Position {
                        line: range.start.line + index as u32,
                        character: indent as u32,
                    };
                    let end_pos = Position {
                        line: range.start.line + index as u32,
                        character: (indent + prefix_len) as u32,
                    };

                    return Some(TextEdit {
                        range: Range::new(start_pos, end_pos),
                        new_text: "- [ ] ".to_string(),
                    });
                }
            }

            None
        })
        .collect();

    (!edits.is_empty()).then_some(edits)
}

/// 切换任务列表的 checkbox 状态
pub fn toggle_task_state(rope: RopeSlice, range: Range) -> Option<Vec<TextEdit>> {
    let edits: Vec<TextEdit> = rope
        .lines()
        .enumerate()
        .filter_map(|(index, line)| {
            let line_str = line.to_string();
            let trimmed = line_str.trim();

            if trimmed.is_empty() {
                return None;
            }

            let indent = line.chars().take_while(|c| c.is_whitespace()).count();
            let content_after_indent = &line_str[indent..];

            let (prefix_len, new_text) = if content_after_indent.starts_with("- [ ] ") {
                (6, "- [x] ".to_string())
            } else if content_after_indent.starts_with("- [x] ") {
                (6, "- [ ] ".to_string())
            } else {
                return None;
            };

            let start_pos = Position {
                line: range.start.line + index as u32,
                character: indent as u32,
            };
            let end_pos = Position {
                line: range.start.line + index as u32,
                character: (indent + prefix_len) as u32,
            };

            Some(TextEdit {
                range: Range::new(start_pos, end_pos),
                new_text,
            })
        })
        .collect();

    (!edits.is_empty()).then_some(edits)
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_lsp::lsp_types::Range;
    use ropey::Rope;

    fn rope_from_str(s: &str) -> Rope {
        Rope::from_str(s)
    }

    fn range(line_start: u32, char_start: u32, line_end: u32, char_end: u32) -> Range {
        Range::new(
            async_lsp::lsp_types::Position::new(line_start, char_start),
            async_lsp::lsp_types::Position::new(line_end, char_end),
        )
    }

    #[test]
    fn test_detect_list_type() {
        assert_eq!(
            detect_list_type(rope_from_str("- item").slice(..)),
            DetectedListType::Unordered
        );
        assert_eq!(
            detect_list_type(rope_from_str("- [ ] item").slice(..)),
            DetectedListType::TaskUnchecked
        );
        assert_eq!(
            detect_list_type(rope_from_str("- [x] item").slice(..)),
            DetectedListType::TaskChecked
        );
        assert_eq!(
            detect_list_type(rope_from_str("1. item").slice(..)),
            DetectedListType::Ordered
        );
        assert_eq!(
            detect_list_type(rope_from_str("plain text").slice(..)),
            DetectedListType::None
        );
    }

    #[test]
    fn test_detect_list_type_nested() {
        let doc = rope_from_str("- a\n  - a1\n- b\n- c");
        assert_eq!(detect_list_type(doc.slice(..)), DetectedListType::Unordered);

        let doc = rope_from_str("- [ ] a\n  - [ ] a1\n- [ ] b\n- [ ] c");
        assert_eq!(
            detect_list_type(doc.slice(..)),
            DetectedListType::TaskUnchecked
        );
    }

    #[test]
    fn test_convert_none_to_ordered() {
        let doc = rope_from_str("a\n  a1\nb\nc");
        let edits = convert_to_list(doc.slice(..), range(0, 0, 3, 0), ListType::Ordered);
        assert!(edits.is_some());
        let edits = edits.unwrap();
        assert_eq!(edits.len(), 4);
        assert_eq!(edits[0].new_text, "1. ");
        assert_eq!(edits[1].new_text, "1.1. ");
        assert_eq!(edits[2].new_text, "2. ");
        assert_eq!(edits[3].new_text, "3. ");
    }

    #[test]
    fn test_convert_none_to_unordered() {
        let doc = rope_from_str("a\n  a1\nb\nc");
        let edits = convert_to_list(doc.slice(..), range(0, 0, 3, 0), ListType::Unordered);
        assert!(edits.is_some());
        let edits = edits.unwrap();
        assert_eq!(edits.len(), 4);
        assert_eq!(edits[0].new_text, "- ");
        assert_eq!(edits[1].new_text, "- ");
        assert_eq!(edits[2].new_text, "- ");
        assert_eq!(edits[3].new_text, "- ");
    }

    #[test]
    fn test_convert_none_to_task() {
        let doc = rope_from_str("a\n  a1\nb\nc");
        let edits = convert_to_list(doc.slice(..), range(0, 0, 3, 0), ListType::TaskList);
        assert!(edits.is_some());
        let edits = edits.unwrap();
        assert_eq!(edits.len(), 4);
        assert_eq!(edits[0].new_text, "- [ ] ");
    }

    #[test]
    fn test_convert_unordered_to_task() {
        let doc = rope_from_str("- a\n  - a1\n- b\n- c");
        let edits = convert_to_list(doc.slice(..), range(0, 0, 3, 0), ListType::UnorderedToTask);
        assert!(edits.is_some());
        let edits = edits.unwrap();
        assert_eq!(edits.len(), 4);
        assert_eq!(edits[0].new_text, "- [ ] ");
        assert_eq!(edits[0].range.start.character, 0);
        assert_eq!(edits[0].range.end.character, 2);
    }

    #[test]
    fn test_convert_ordered_to_task() {
        let doc = rope_from_str("1. a\n  1.1. a1\n2. b\n3. c");
        let edits = convert_to_list(doc.slice(..), range(0, 0, 3, 0), ListType::OrderedToTask);
        assert!(edits.is_some());
        let edits = edits.unwrap();
        assert_eq!(edits.len(), 4);
        assert_eq!(edits[0].new_text, "- [ ] ");
        assert_eq!(edits[0].range.end.character, 3);
    }

    #[test]
    fn test_convert_task_to_unordered() {
        let doc = rope_from_str("- [ ] a\n  - [ ] a1\n- [ ] b\n- [ ] c");
        let edits = convert_to_list(doc.slice(..), range(0, 0, 3, 0), ListType::TaskToUnordered);
        assert!(edits.is_some());
        let edits = edits.unwrap();
        assert_eq!(edits.len(), 4);
        assert_eq!(edits[0].new_text, "- ");
        assert_eq!(edits[0].range.end.character, 6);
    }

    #[test]
    fn test_toggle_task_state() {
        let doc = rope_from_str("- [ ] a\n- [ ] b\n- [ ] c");
        let edits = convert_to_list(doc.slice(..), range(0, 0, 2, 0), ListType::ToggleTask);
        assert!(edits.is_some());
        let edits = edits.unwrap();
        assert_eq!(edits.len(), 3);
        assert_eq!(edits[0].new_text, "- [x] ");
        assert_eq!(edits[1].new_text, "- [x] ");
        assert_eq!(edits[2].new_text, "- [x] ");
    }

    #[test]
    fn test_toggle_task_state_checked_to_unchecked() {
        let doc = rope_from_str("- [x] a\n- [x] b");
        let edits = convert_to_list(doc.slice(..), range(0, 0, 1, 0), ListType::ToggleTask);
        assert!(edits.is_some());
        let edits = edits.unwrap();
        assert_eq!(edits.len(), 2);
        assert_eq!(edits[0].new_text, "- [ ] ");
        assert_eq!(edits[1].new_text, "- [ ] ");
    }
}
