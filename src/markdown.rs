use std::collections::HashMap;

use async_lsp::lsp_types::{
    CodeAction, CodeActionKind, CodeActionOrCommand, CodeActionParams, Position, Range, TextEdit,
    WorkspaceEdit,
};
use list::{ListType, convert_to_list, detect_list_type, is_task_line, toggle_task_state};
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

    let range_content = match get_range_content(doc, &params.range) {
        Some(content) => content,
        None => return Vec::new(),
    };

    let mut items = Vec::new();

    let detected_type = detect_list_type(range_content);

    if params.range.end.line != params.range.start.line {
        if params.range.end.line - params.range.start.line > 1 {
            items.push(("Table Format", table::format(range_content, params.range)));
        }

        match detected_type {
            list::DetectedListType::None => {
                if let Some(edits) = convert_to_list(range_content, params.range, ListType::Ordered)
                {
                    items.push(("Order List", edits));
                }
                if let Some(edits) =
                    convert_to_list(range_content, params.range, ListType::Unordered)
                {
                    items.push(("Unorder List", edits));
                }
                if let Some(edits) =
                    convert_to_list(range_content, params.range, ListType::TaskList)
                {
                    items.push(("Task List", edits));
                }
            }
            list::DetectedListType::Ordered => {
                if let Some(edits) =
                    convert_to_list(range_content, params.range, ListType::OrderedToTask)
                {
                    items.push(("To Task List", edits));
                }
                if let Some(edits) =
                    convert_to_list(range_content, params.range, ListType::TaskToUnordered)
                {
                    items.push(("To Unordered List", edits));
                }
            }
            list::DetectedListType::Unordered => {
                if let Some(edits) =
                    convert_to_list(range_content, params.range, ListType::UnorderedToTask)
                {
                    items.push(("To Task List", edits));
                }
                if let Some(edits) = convert_to_list(range_content, params.range, ListType::Ordered)
                {
                    items.push(("To Ordered List", edits));
                }
            }
            list::DetectedListType::TaskUnchecked | list::DetectedListType::TaskChecked => {
                if let Some(edits) = toggle_task_state(range_content, params.range) {
                    items.push(("Toggle Checkbox", edits));
                }
                if let Some(edits) =
                    convert_to_list(range_content, params.range, ListType::TaskToUnordered)
                {
                    items.push(("To Unordered List", edits));
                }
                if let Some(edits) =
                    convert_to_list(range_content, params.range, ListType::OrderedToTask)
                {
                    items.push(("To Ordered List", edits));
                }
            }
        }
    } else {
        let line = params.range.start.line;
        let is_task = is_task_line(doc, line);

        if is_task && let Some(line_slice) = doc.get_line(line as usize) {
            let line_range = Range {
                start: Position { line, character: 0 },
                end: params.range.end,
            };
            if let Some(edits) = toggle_task_state(line_slice, line_range) {
                items.push(("Toggle Checkbox", edits));
            }
        }

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
