use std::collections::HashMap;

use async_lsp::lsp_types::{
    CodeAction, CodeActionKind, CodeActionOrCommand, CodeActionParams, TextEdit, WorkspaceEdit,
};
use list::{ListType, convert_to_list};
use ropey::Rope;

use crate::encoding::get_range_content;

mod list;
mod table;

pub(super) fn actions(
    lang_id: String,
    doc: &Rope,
    params: &CodeActionParams,
) -> Vec<CodeActionOrCommand> {
    if lang_id != "markdown" {
        return Vec::new();
    }

    let range_content = get_range_content(doc, &params.range).unwrap_or("".into());
    let mut items = Vec::new();

    if params.range.end.line - params.range.start.line > 1 {
        items.push(("Table Format", table::format(range_content, params.range)));
    }

    if params.range.end.line != params.range.start.line {
        if let Some(edits) = convert_to_list(range_content, params.range, ListType::Ordered) {
            items.push(("Order List", edits));
        };
        if let Some(edits) = convert_to_list(range_content, params.range, ListType::Unordered) {
            items.push(("Unorder List", edits));
        };
        if let Some(edits) = convert_to_list(range_content, params.range, ListType::TaskList) {
            items.push(("Task List", edits));
        };
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
    }

    items
        .iter()
        .filter_map(|(item, edits)| {
            if edits.is_empty() {
                return None;
            }

            let mut changes = HashMap::new();
            changes.insert(params.text_document.uri.clone(), edits.to_vec());

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
