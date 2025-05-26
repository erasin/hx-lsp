use std::{collections::HashMap, sync::OnceLock};

use async_lsp::lsp_types::{
    CodeAction, CodeActionKind, CodeActionOrCommand, CodeActionParams, TextEdit, WorkspaceEdit,
};
use convert_case::{Case, Casing};
use regex::Regex;
use ropey::Rope;

pub(super) fn case_actions(
    range_content: String,
    params: &CodeActionParams,
) -> Vec<CodeActionOrCommand> {
    if params.range.start.line != params.range.end.line {
        return Vec::new();
    }
    if !range_content.is_ascii() {
        return Vec::new();
    }

    let items = [
        ("case_snake", Case::Snake),
        ("CasePascal", Case::Pascal),
        ("caseCamel", Case::Camel),
    ];

    items
        .iter()
        .map(|&(item, case)| {
            let out = range_content.to_case(case);

            let mut changes = HashMap::new();
            let edits = vec![TextEdit {
                range: params.range,
                new_text: out,
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

    // 多行
    if params.range.end.line - params.range.start.line > 1 {
        // 表格必须为三行以上，第二行以 `--` 开头
        let line_2 = doc.get_line(params.range.start.line as usize + 1).unwrap();
        if md_table_line_rg().is_match(line_2.to_string().trim()) {
            let out = markdown_table_formatter::format_tables(range_content);
            items.push(("TableFormat", out));
        }
    }

    // 单行处理
    if params.range.start.line == params.range.end.line {
        items.push(("Bold", format!("**{range_content}**")));
        items.push(("Italic", format!("_{range_content}_")));
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
