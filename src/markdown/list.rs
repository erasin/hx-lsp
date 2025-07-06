use std::collections::HashMap;

use async_lsp::lsp_types::{Position, Range, TextEdit};
use comrak::{
    Arena, ComrakOptions, ExtensionOptions,
    nodes::{AstNode, NodeValue},
    parse_document,
};
use ropey::RopeSlice;

/// 列表转换类型枚举
pub enum ListType {
    Ordered,   // 有序列表 (1., 2., 3.)
    Unordered, // 无序列表 (-, *, +)
    TaskList,  // 任务列表 (- [ ])
}

/// 将选中的非列表 Markdown 文本转换为指定类型的列表
pub fn convert_to_list(
    rope: RopeSlice,
    range: Range,
    conversion_type: ListType,
) -> Option<Vec<TextEdit>> {
    // 使用 Comrak 解析 Markdown
    let arena = Arena::new();
    let options = ComrakOptions {
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
    };

    let root = parse_document(&arena, &rope.to_string(), &options);

    // 检查是否包含列表
    if contains_list(root) {
        return None;
    }

    // 为需要添加序号的行生成文本编辑操作
    let mut counters: HashMap<usize, u32> = HashMap::new(); // 缩进级别 -> 计数器
    let mut current_levels = Vec::new(); // 当前缩进层级栈
    // let mut prev_indent: Option<usize> = None; // 上一行的缩进级别

    let edits: Vec<TextEdit> = rope
        .lines()
        .enumerate()
        .filter_map(|(index, line)| {
            // 计算缩进级别（前导空格/制表符数量）
            let indent = line.chars().take_while(|c| c.is_whitespace()).count();

            // 跳过空行
            if line.to_string().trim().is_empty() {
                current_levels.clear();
                return None;
            }

            // 更新缩进层级栈
            while !current_levels.is_empty() && *current_levels.last().unwrap() > indent {
                current_levels.pop();
            }

            if current_levels.last() != Some(&indent) {
                current_levels.push(indent);
            }

            // 根据转换类型生成列表前缀
            let prefix = match conversion_type {
                ListType::Ordered => {
                    // 获取当前层级的计数器
                    let level = current_levels.len().saturating_sub(1);
                    let counter = counters.entry(level).or_insert(0);
                    *counter += 1;

                    // 重置更深层级的计数器
                    for l in (level + 1).. {
                        if counters.remove(&l).is_none() {
                            break;
                        }
                    }

                    // // 生成有序列表前缀
                    let prefix = (0..=level)
                        .map(|l| counters.get(&l).unwrap_or(&1).to_string())
                        .collect::<Vec<_>>()
                        .join(".");

                    // 生成有序列表前缀
                    format!("{prefix}. ")
                }
                ListType::Unordered => {
                    // 无序列表使用相同的标记
                    "- ".to_string()
                }
                ListType::TaskList => {
                    // 任务列表使用未选中标记
                    "- [ ] ".to_string()
                }
            };

            // 计算插入位置（在缩进之后）
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

/// 检查 AST 是否包含列表
fn contains_list<'a>(root: &'a AstNode<'a>) -> bool {
    let mut stack = vec![root];
    while let Some(node) = stack.pop() {
        match &node.data.borrow().value {
            NodeValue::List(_) | NodeValue::Item(_) | NodeValue::TaskItem(_) => {
                return true;
            }
            _ => {}
        }
        stack.extend(node.children());
    }
    false
}
