use lsp_types::{Range, TextDocumentContentChangeEvent};
use ropey::{Rope, RopeSlice};

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
/// 转换 [`lsp::Position`] 为文本位置。
///
/// Returns `None` if position.line is out of bounds or an overflow occurs
pub fn lsp_pos_to_pos(
    doc: &Rope,
    pos: lsp_types::Position,
    offset_encoding: OffsetEncoding,
) -> Result<usize, Error> {
    let pos_line = pos.line as usize;
    if pos_line > doc.len_lines() - 1 {
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
    .map(|p| p + doc.line_to_char(pos.line as usize))
    .map_err(|_| Error::PositionOutOfBounds(pos.line, pos.character))
}

/// 增量变更文本
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

            // 获取 line 中的索引
            let change_start_doc_char_idx =
                lsp_pos_to_pos(doc, range.start, offset_encoding).unwrap();
            let change_end_doc_char_idx = match range.start == range.end {
                true => change_start_doc_char_idx,
                false => lsp_pos_to_pos(doc, range.end, offset_encoding).unwrap(),
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

pub fn get_last_word_at_pos<'a>(line: &'a RopeSlice, line_character_pos: usize) -> Option<&'a str> {
    if line_character_pos == 0 || line_character_pos > line.len_chars() {
        return None;
    }

    let offset = line
        .chars_at(line_character_pos)
        .reversed()
        .take_while(|&ch| char_is_punctuation(ch) || char_is_word(ch))
        .count();

    if offset == 0 {
        return None;
    }

    line.slice(line_character_pos.saturating_sub(offset)..line_character_pos)
        .as_str()
}

/// 获取内容
fn get_range_content<'a>(
    doc: &'a Rope,
    range: &Range,
    offset_encoding: OffsetEncoding,
) -> Option<RopeSlice<'a>> {
    if range.start > range.end {
        return None;
    }

    let start_idx = lsp_pos_to_pos(doc, range.start, offset_encoding).unwrap();
    let end_idx = match range.start == range.end {
        true => start_idx,
        false => lsp_pos_to_pos(doc, range.end, offset_encoding).unwrap(),
    };
    let s = doc.slice(start_idx..end_idx);
    Some(s)
}

#[inline]
pub fn char_is_punctuation(ch: char) -> bool {
    use unicode_general_category::{get_general_category, GeneralCategory};

    matches!(
        get_general_category(ch),
        GeneralCategory::OtherPunctuation
            | GeneralCategory::OpenPunctuation
            | GeneralCategory::ClosePunctuation
            | GeneralCategory::InitialPunctuation
            | GeneralCategory::FinalPunctuation
            | GeneralCategory::ConnectorPunctuation
            | GeneralCategory::DashPunctuation
            | GeneralCategory::MathSymbol
            | GeneralCategory::CurrencySymbol
            | GeneralCategory::ModifierSymbol
    )
}

#[inline]
pub fn char_is_word(ch: char) -> bool {
    ch.is_alphanumeric() || ch == '_'
}

#[cfg(test)]
mod test {

    use crate::encoding::char_is_punctuation;

    use super::get_last_word_at_pos;

    #[test]
    fn test_get_last() {
        let line = ropey::RopeSlice::from("abcd ef1h");
        let word = get_last_word_at_pos(&line, 7);
        assert_eq!(Some("ef"), word);
    }

    #[test]
    fn test_pun() {
        assert!(char_is_punctuation(':'));
    }
}
