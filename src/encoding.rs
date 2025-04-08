use async_lsp::lsp_types::{Position, Range, TextDocumentContentChangeEvent};
use ropey::{Rope, RopeSlice};
use tracing::warn;

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

/// Converts LSP Position to a position in the document.
///
/// Returns `None` if position.line is out of bounds or an overflow occurs
pub fn lsp_pos_to_pos(
    doc: &Rope,
    pos: Position,
    offset_encoding: OffsetEncoding,
) -> Result<usize, Error> {
    let pos_line = pos.line as usize;
    if pos_line > doc.len_lines() - 1 {
        warn!("LSP position {pos:?} out of range assuming EOF");
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
) -> Result<(), Error> {
    let offset_encoding = OffsetEncoding::Utf16;
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
        }
        None => {
            *doc = Rope::from_str(&change.text);
        }
    }
    Ok(())
}

//  If input as field or attribute return true.
pub fn is_field(line: &RopeSlice, line_character_pos: usize) -> bool {
    if line_character_pos == 0 || line_character_pos > line.len_chars() {
        return false;
    }

    let mut after_punctuation = false;
    let _offset = line
        .chars_at(line_character_pos)
        .reversed()
        .take_while(|&ch| {
            if char_is_punctuation(ch) {
                after_punctuation = true;
                return true;
            }
            char_is_word(ch)
        })
        .count();

    after_punctuation
}

pub fn get_current_word<'a>(line: &'a RopeSlice, line_character_pos: usize) -> Option<&'a str> {
    if line_character_pos == 0 || line_character_pos > line.len_chars() {
        return None;
    }

    let offset_sub = line
        .chars_at(line_character_pos)
        .reversed()
        .take_while(|&ch| char_is_word(ch))
        .count();

    let offset_add = line
        .chars_at(line_character_pos)
        .take_while(|&ch| char_is_word(ch))
        .count();

    if offset_sub == 0 && offset_add == 0 {
        return None;
    }

    line.slice(
        line_character_pos.saturating_sub(offset_sub)
            ..line_character_pos.saturating_add(offset_add),
    )
    .as_str()
}

/// 获取内容
pub fn get_range_content<'a>(doc: &'a Rope, range: &Range) -> Option<RopeSlice<'a>> {
    let offset_encoding = OffsetEncoding::Utf16;
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
    use unicode_general_category::{GeneralCategory, get_general_category};

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

    use async_lsp::lsp_types::{Position, Range};
    use ropey::Rope;

    use crate::encoding::{char_is_punctuation, get_range_content};

    use super::get_current_word;

    #[test]
    fn test_get_range_content() {
        let cases = [
            ("你好世界", (0, 0, 0, 2), "你好"),
            ("你好世界", (0, 2, 0, 4), "世界"),
        ];

        for (input, range, expected) in cases {
            let result = get_range_content(
                &Rope::from_str(input),
                &Range::new(
                    Position::new(range.0, range.1),
                    Position::new(range.2, range.3),
                ),
            )
            .map(|f| f.to_string())
            .unwrap_or_default();
            assert_eq!(result, expected, "{input}:\n {result} != {expected}")
        }
    }

    #[test]
    fn test_get_last() {
        let line = ropey::RopeSlice::from("abcd ef1h");
        let word = get_current_word(&line, 7);
        assert_eq!(Some("ef1h"), word);
    }

    #[test]
    fn test_pun() {
        assert!(char_is_punctuation(':'));
    }
}
