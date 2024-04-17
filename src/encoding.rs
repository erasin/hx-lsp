use lsp_types::TextDocumentContentChangeEvent;
use ropey::Rope;

use crate::errors::Error;

// 参考
// helix-lsp/src/lib.rs
// https://gist.github.com/rojas-diego/04d9c4e3fff5f8374f29b9b738d541ef

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum OffsetEncoding {
    /// UTF-8 code units aka bytes
    Utf8,
    /// UTF-16 code units
    #[default]
    Utf16,
    /// UTF-32 code units aka chars
    Utf32,
}

/// Converts [`lsp::Position`] to a position in the document.
///
/// Returns `None` if position.line is out of bounds or an overflow occurs
pub fn lsp_pos_to_pos(
    doc: &Rope,
    pos: lsp_types::Position,
    offset_encoding: OffsetEncoding,
) -> Result<usize, Error> {
    let pos_line = pos.line as usize;
    if pos_line > doc.len_lines() - 1 {
        // If it extends past the end, truncate it to the end. This is because the
        // way the LSP describes the range including the last newline is by
        // specifying a line number after what we would call the last line.
        log::warn!("LSP position {pos:?} out of range assuming EOF");
        return Err(Error::PositionOutOfBounds(pos.line, pos.character));
    }

    let slice = match doc.get_line(pos.line as usize) {
        Some(line) => line,
        None => return Err(Error::PositionOutOfBounds(pos.line, pos.character)),
    };

    match offset_encoding {
        OffsetEncoding::Utf8 => slice.try_byte_to_char(pos.character as usize),
        OffsetEncoding::Utf16 => slice.try_utf16_cu_to_char(pos.character as usize),
        OffsetEncoding::Utf32 => Ok(pos.character as usize),
    }
    .map_err(|_| Error::PositionOutOfBounds(pos.line, pos.character))
}

// 增量变更文本
pub fn apply_content_change(
    doc: &mut Rope,
    change: &TextDocumentContentChangeEvent,
    offset_encoding: OffsetEncoding,
) -> Result<(), Error> {
    match change.range {
        Some(range) => {
            assert!(
                range.start.line < range.end.line
                    || (range.start.line == range.end.line
                        && range.start.character <= range.end.character)
            );

            let same_line = range.start.line == range.end.line;
            let same_character = range.start.character == range.end.character;

            let change_start_line_idx = range.start.line as usize;
            let change_end_line_idx = range.end.line as usize;

            // 获取 line 中的索引
            let change_start_line_char_idx =
                lsp_pos_to_pos(doc, range.start, offset_encoding).unwrap();
            let change_end_line_char_idx = match same_line && same_character {
                true => change_start_line_char_idx,
                false => lsp_pos_to_pos(doc, range.end, offset_encoding).unwrap(),
            };

            // 转化为 doc 索引
            let change_start_doc_char_idx =
                doc.line_to_char(change_start_line_idx) + change_start_line_char_idx;
            let change_end_doc_char_idx = match same_line && same_character {
                true => change_start_doc_char_idx,
                false => doc.line_to_char(change_end_line_idx) + change_end_line_char_idx,
            };

            // 移除区域并插入新的文本
            doc.remove(change_start_doc_char_idx..change_end_doc_char_idx);
            doc.insert(change_start_doc_char_idx, &change.text);

            return Ok(());
        }
        None => {
            *doc = Rope::from_str(&change.text);

            return Ok(());
        }
    }
}

// 获取变更时候最后的
// fn input_last_word() {}
