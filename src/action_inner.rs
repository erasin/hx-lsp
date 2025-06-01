use std::collections::HashMap;

use async_lsp::lsp_types::{
    CodeAction, CodeActionKind, CodeActionOrCommand, CodeActionParams, TextEdit, WorkspaceEdit,
};
use convert_case::{Case, Casing};
use ropey::RopeSlice;

pub(super) fn case_actions(
    range_content: RopeSlice,
    params: &CodeActionParams,
) -> Vec<CodeActionOrCommand> {
    if params.range.start.line != params.range.end.line {
        return Vec::new();
    }

    let range_content = range_content.to_string();

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
