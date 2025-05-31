use std::{collections::HashMap, sync::OnceLock};

use async_lsp::lsp_types::{
    CodeAction, CodeActionKind, CodeActionOrCommand, CodeActionParams, Position, Range, TextEdit,
    WorkspaceEdit,
};
use convert_case::{Case, Casing};
use regex::Regex;
use ropey::Rope;

use crate::encoding::get_range_content;

pub(super) fn case_actions(
    range_content: String,
    params: &CodeActionParams,
) -> Vec<CodeActionOrCommand> {
    if params.range.start.line != params.range.end.line {
        return Vec::new();
    }
    if range_content.len() == 1 || !range_content.is_ascii() {
        return Vec::new();
    }

    let items = [
        ("case_snake", Case::Snake),
        ("CasePascal", Case::Pascal),
        ("caseCamel", Case::Camel),
    ];

    items
        .iter()
        .filter_map(|&(item, case)| {
            let out = range_content.to_case(case);
            if out.eq(&range_content) {
                return None;
            }

            let mut changes = HashMap::new();
            let edits = vec![TextEdit {
                range: params.range,
                new_text: out,
            }];
            changes.insert(params.text_document.uri.clone(), edits);

            Some(
                CodeAction {
                    title: item.to_string(),
                    kind: Some(CodeActionKind::REFACTOR_INLINE),
                    edit: Some(WorkspaceEdit::new(changes)),
                    is_preferred: Some(true),
                    ..Default::default()
                }
                .into(),
            )
        })
        .collect()
}

static MD_TABLE_PATTERN: OnceLock<Regex> = OnceLock::new();

fn md_table_line_rg() -> &'static Regex {
    MD_TABLE_PATTERN.get_or_init(|| Regex::new(r"^[-:| ]+$").expect("Invalid regex pattern"))
}

pub(super) fn markdown_actions(
    lang_id: String,
    doc: &Rope,
    params: &CodeActionParams,
) -> Vec<CodeActionOrCommand> {
    if lang_id != "markdown" {
        return Vec::new();
    }

    let range_content = get_range_content(doc, &params.range).unwrap_or("".into());

    let mut items = Vec::new();

    // 表格必须为三行以上，第二行起存在表头为 `- :|`
    if params.range.end.line - params.range.start.line > 1 {
        let has_table = range_content
            .lines()
            .skip(1)
            .any(|line| md_table_line_rg().is_match(line.to_string().trim()));

        if has_table {
            let edits = vec![TextEdit {
                range: params.range,
                new_text: markdown_table_formatter::format_tables(range_content.to_string()),
            }];

            items.push(("Table Format", edits));
        }
    }

    if params.range.end.line != params.range.start.line {
        if !range_content.char(0).is_numeric() && range_content.char(1) != '.' {
            let content = range_content
                .lines()
                .enumerate()
                .filter_map(|(index_line, line)| {
                    // 最后一行为空
                    if (params.range.end.line - params.range.start.line) as usize == index_line
                        && params.range.end.character == 0
                    {
                        return None;
                    }

                    let (index_char, _) = line
                        .chars()
                        .enumerate()
                        .find(|(_index_char, c)| !c.is_ascii_whitespace())
                        .unwrap_or_default();

                    let line = index_line as u32 + params.range.start.line;
                    let character = index_char as u32;

                    Some(TextEdit {
                        range: Range::new(
                            Position::new(line, character),
                            Position::new(line, character),
                        ),
                        new_text: format!("{}. ", (index_line + 1)),
                    })
                })
                .collect();

            items.push(("Ordered List", content));
        }

        if range_content.char(0) != '-' && range_content.char(1) != ' ' {
            let unorder_content = range_content
                .lines()
                .enumerate()
                .filter_map(|(index_line, line)| {
                    if (params.range.end.line - params.range.start.line) as usize == index_line
                        && params.range.end.character == 0
                    {
                        return None;
                    }

                    let (index_char, _) = line
                        .chars()
                        .enumerate()
                        .find(|(_index_char, c)| !c.is_ascii_whitespace())?;

                    let line = index_line as u32 + params.range.start.line;
                    let character = index_char as u32;

                    Some(TextEdit {
                        range: Range::new(
                            Position::new(line, character),
                            Position::new(line, character),
                        ),
                        new_text: "- ".to_string(),
                    })
                })
                .collect();

            let task_content = range_content
                .lines()
                .enumerate()
                .filter_map(|(index_line, line)| {
                    if (params.range.end.line - params.range.start.line) as usize == index_line
                        && params.range.end.character == 0
                    {
                        return None;
                    }

                    let (index_char, _) = line
                        .chars()
                        .enumerate()
                        .find(|(_index_char, c)| !c.is_ascii_whitespace())?;

                    let line = index_line as u32 + params.range.start.line;
                    let character = index_char as u32;

                    Some(TextEdit {
                        range: Range::new(
                            Position::new(line, character),
                            Position::new(line, character),
                        ),
                        new_text: "- [ ] ".to_string(),
                    })
                })
                .collect();

            items.push(("Unordered List", unorder_content));
            items.push(("Task List", task_content));
        }
    }

    // 单行处理
    if params.range.start.line == params.range.end.line {
        items.push((
            "Bold",
            vec![TextEdit {
                range: params.range,
                new_text: format!("**{range_content}**"),
            }],
        ));
        items.push((
            "Italic",
            vec![TextEdit {
                range: params.range,
                new_text: format!("_{range_content}_"),
            }],
        ));
        items.push((
            "Strikethrough",
            vec![TextEdit {
                range: params.range,
                new_text: format!("~~{range_content}~~"),
            }],
        ));

        if params.range.start.character + 1 == params.range.end.character {
            // let line = doc.line(params.range.start.line as usize);
            // line.chars().find_map(f)
            // TODO TextEdit
        }
    }

    items
        .iter()
        .map(|(item, edits)| {
            let mut changes = HashMap::new();
            changes.insert(params.text_document.uri.clone(), edits.to_vec());

            CodeAction {
                title: item.to_string(),
                kind: Some(CodeActionKind::REFACTOR_REWRITE),
                edit: Some(WorkspaceEdit::new(changes)),
                ..Default::default()
            }
            .into()
        })
        .collect()
}
