use std::{collections::HashMap, sync::OnceLock};

use async_lsp::lsp_types::{
    CodeAction, CodeActionKind, CodeActionOrCommand, CodeActionParams, TextEdit, WorkspaceEdit,
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
                    kind: Some(CodeActionKind::REFACTOR_REWRITE),
                    edit: Some(WorkspaceEdit::new(changes)),
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
    range_content: &String,
    params: &CodeActionParams,
) -> Vec<CodeActionOrCommand> {
    if lang_id != "markdown" {
        return Vec::new();
    }

    let mut items = Vec::new();

    // 表格必须为三行以上，第二行起存在表头为 `- :|`
    if params.range.end.line - params.range.start.line > 1 {
        let has_table = doc
            .lines_at(params.range.start.line as usize)
            .skip(1)
            .any(|line| md_table_line_rg().is_match(line.to_string().trim()));

        if has_table {
            let out = markdown_table_formatter::format_tables(range_content);
            items.push(("Table Format", out));
        }
    }

    if params.range.end.line != params.range.start.line {
        let range_content = get_range_content(doc, &params.range).unwrap_or("".into());

        if !range_content.char(0).is_numeric() && range_content.char(1) != '.' {
            let order_content: String = range_content
                .lines()
                .enumerate()
                .map(|(index, line)| {
                    let line = line.to_string();
                    if (params.range.end.line - params.range.start.line) as usize == index
                        && params.range.end.character == 0
                    {
                        return line;
                    }
                    format!("{}. {line}", (index + 1))
                })
                .collect();
            items.push(("Ordered List", order_content));
        }

        if range_content.char(0) != '-' && range_content.char(1) != ' ' {
            let unorder_content: String = range_content
                .lines()
                .enumerate()
                .map(|(index, line)| {
                    let line = line.to_string();
                    if (params.range.end.line - params.range.start.line) as usize == index
                        && params.range.end.character == 0
                    {
                        return line;
                    }
                    format!("- {line}")
                })
                .collect();

            let task_content: String = range_content
                .lines()
                .enumerate()
                .map(|(index, line)| {
                    let line = line.to_string();
                    if (params.range.end.line - params.range.start.line) as usize == index
                        && params.range.end.character == 0
                    {
                        return line;
                    }
                    format!("- [ ] {line}")
                })
                .collect();

            items.push(("Unordered List", unorder_content));
            items.push(("Task List", task_content));
        }
    }

    // 单行处理
    if params.range.start.line == params.range.end.line {
        items.push(("Bold", format!("**{range_content}**")));
        items.push(("Italic", format!("_{range_content}_")));
        items.push(("Strikethrough", format!("~~{range_content}~~")));
    }

    items
        .iter()
        .map(|&(item, ref out)| {
            let mut changes = HashMap::new();
            let edits = vec![TextEdit {
                range: params.range,
                new_text: out.clone(),
            }];
            changes.insert(params.text_document.uri.clone(), edits);

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
