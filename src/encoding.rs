use ropey::{Rope, RopeSlice};

// helix-lsp/src/lib.rs

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum OffsetEncoding {
    /// UTF-8 code units aka bytes
    Utf8,
    /// UTF-32 code units aka chars
    Utf32,
    /// UTF-16 code units
    #[default]
    Utf16,
}

/// Converts [`lsp::Position`] to a position in the document.
///
/// Returns `None` if position.line is out of bounds or an overflow occurs
pub fn lsp_pos_to_pos(
    doc: &Rope,
    pos: lsp_types::Position,
    offset_encoding: OffsetEncoding,
) -> Option<usize> {
    let pos_line = pos.line as usize;
    if pos_line > doc.len_lines() - 1 {
        // If it extends past the end, truncate it to the end. This is because the
        // way the LSP describes the range including the last newline is by
        // specifying a line number after what we would call the last line.
        log::warn!("LSP position {pos:?} out of range assuming EOF");
        return Some(doc.len_chars());
    }

    // We need to be careful here to fully comply ith the LSP spec.
    // Two relevant quotes from the spec:
    //
    // https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#position
    // > If the character value is greater than the line length it defaults back
    // >  to the line length.
    //
    // https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#textDocuments
    // > To ensure that both client and server split the string into the same
    // > line representation the protocol specifies the following end-of-line sequences:
    // > ‘\n’, ‘\r\n’ and ‘\r’. Positions are line end character agnostic.
    // > So you can not specify a position that denotes \r|\n or \n| where | represents the character offset.
    //
    // This means that while the line must be in bounds the `character`
    // must be capped to the end of the line.
    // Note that the end of the line here is **before** the line terminator
    // so we must use `line_end_char_index` instead of `doc.line_to_char(pos_line + 1)`
    //
    // FIXME: Helix does not fully comply with the LSP spec for line terminators.
    // The LSP standard requires that line terminators are ['\n', '\r\n', '\r'].
    // Without the unicode-linebreak feature disabled, the `\r` terminator is not handled by helix.
    // With the unicode-linebreak feature, helix recognizes multiple extra line break chars
    // which means that positions will be decoded/encoded incorrectly in their presence

    let line = match offset_encoding {
        OffsetEncoding::Utf8 => {
            let line_start = doc.line_to_byte(pos_line);
            let line_end = line_end_byte_index(&doc.slice(..), pos_line);
            line_start..line_end
        }
        OffsetEncoding::Utf16 => {
            // TODO directly translate line index to char-idx
            // ropey can do this just as easily as utf-8 byte translation
            // but the functions are just missing.
            // Translate to char first and then utf-16 as a workaround
            let line_start = doc.line_to_char(pos_line);
            let line_end = line_end_char_index(&doc.slice(..), pos_line);
            doc.char_to_utf16_cu(line_start)..doc.char_to_utf16_cu(line_end)
        }
        OffsetEncoding::Utf32 => {
            let line_start = doc.line_to_char(pos_line);
            let line_end = line_end_char_index(&doc.slice(..), pos_line);
            line_start..line_end
        }
    };

    // The LSP spec demands that the offset is capped to the end of the line
    let pos = line
        .start
        .checked_add(pos.character as usize)
        .unwrap_or(line.end)
        .min(line.end);

    match offset_encoding {
        OffsetEncoding::Utf8 => doc.try_byte_to_char(pos).ok(),
        OffsetEncoding::Utf16 => doc.try_utf16_cu_to_char(pos).ok(),
        OffsetEncoding::Utf32 => Some(pos),
    }
}

/// helix-core/src/line_ending
#[cfg(target_os = "windows")]
pub const NATIVE_LINE_ENDING: LineEnding = LineEnding::Crlf;
#[cfg(not(target_os = "windows"))]
pub const NATIVE_LINE_ENDING: LineEnding = LineEnding::LF;

/// Represents one of the valid Unicode line endings.
#[derive(PartialEq, Eq, Copy, Clone, Debug)]
pub enum LineEnding {
    Crlf, // CarriageReturn followed by LineFeed
    LF,   // U+000A -- LineFeed
    #[cfg(feature = "unicode-lines")]
    VT, // U+000B -- VerticalTab
    #[cfg(feature = "unicode-lines")]
    FF, // U+000C -- FormFeed
    #[cfg(feature = "unicode-lines")]
    CR, // U+000D -- CarriageReturn
    #[cfg(feature = "unicode-lines")]
    Nel, // U+0085 -- NextLine
    #[cfg(feature = "unicode-lines")]
    LS, // U+2028 -- Line Separator
    #[cfg(feature = "unicode-lines")]
    PS, // U+2029 -- ParagraphSeparator
}

impl LineEnding {
    #[inline]
    pub const fn len_chars(&self) -> usize {
        match self {
            Self::Crlf => 2,
            _ => 1,
        }
    }

    #[inline]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Crlf => "\u{000D}\u{000A}",
            Self::LF => "\u{000A}",
            #[cfg(feature = "unicode-lines")]
            Self::VT => "\u{000B}",
            #[cfg(feature = "unicode-lines")]
            Self::FF => "\u{000C}",
            #[cfg(feature = "unicode-lines")]
            Self::CR => "\u{000D}",
            #[cfg(feature = "unicode-lines")]
            Self::Nel => "\u{0085}",
            #[cfg(feature = "unicode-lines")]
            Self::LS => "\u{2028}",
            #[cfg(feature = "unicode-lines")]
            Self::PS => "\u{2029}",
        }
    }

    #[inline]
    pub const fn from_char(ch: char) -> Option<LineEnding> {
        match ch {
            '\u{000A}' => Some(LineEnding::LF),
            #[cfg(feature = "unicode-lines")]
            '\u{000B}' => Some(LineEnding::VT),
            #[cfg(feature = "unicode-lines")]
            '\u{000C}' => Some(LineEnding::FF),
            #[cfg(feature = "unicode-lines")]
            '\u{000D}' => Some(LineEnding::CR),
            #[cfg(feature = "unicode-lines")]
            '\u{0085}' => Some(LineEnding::Nel),
            #[cfg(feature = "unicode-lines")]
            '\u{2028}' => Some(LineEnding::LS),
            #[cfg(feature = "unicode-lines")]
            '\u{2029}' => Some(LineEnding::PS),
            // Not a line ending
            _ => None,
        }
    }

    // Normally we'd want to implement the FromStr trait, but in this case
    // that would force us into a different return type than from_char or
    // or from_rope_slice, which would be weird.
    #[allow(clippy::should_implement_trait)]
    #[inline]
    pub fn from_str(g: &str) -> Option<LineEnding> {
        match g {
            "\u{000D}\u{000A}" => Some(LineEnding::Crlf),
            "\u{000A}" => Some(LineEnding::LF),
            #[cfg(feature = "unicode-lines")]
            "\u{000B}" => Some(LineEnding::VT),
            #[cfg(feature = "unicode-lines")]
            "\u{000C}" => Some(LineEnding::FF),
            #[cfg(feature = "unicode-lines")]
            "\u{000D}" => Some(LineEnding::CR),
            #[cfg(feature = "unicode-lines")]
            "\u{0085}" => Some(LineEnding::Nel),
            #[cfg(feature = "unicode-lines")]
            "\u{2028}" => Some(LineEnding::LS),
            #[cfg(feature = "unicode-lines")]
            "\u{2029}" => Some(LineEnding::PS),
            // Not a line ending
            _ => None,
        }
    }

    #[inline]
    pub fn from_rope_slice(g: &RopeSlice) -> Option<LineEnding> {
        if let Some(text) = g.as_str() {
            LineEnding::from_str(text)
        } else {
            // Non-contiguous, so it can't be a line ending.
            // Specifically, Ropey guarantees that CRLF is always
            // contiguous.  And the remaining line endings are all
            // single `char`s, and therefore trivially contiguous.
            None
        }
    }
}

#[inline]
pub fn str_is_line_ending(s: &str) -> bool {
    LineEnding::from_str(s).is_some()
}

#[inline]
pub fn rope_is_line_ending(r: RopeSlice) -> bool {
    r.chunks().all(str_is_line_ending)
}

/// Attempts to detect what line ending the passed document uses.
pub fn auto_detect_line_ending(doc: &Rope) -> Option<LineEnding> {
    // Return first matched line ending. Not all possible line endings
    // are being matched, as they might be special-use only
    for line in doc.lines().take(100) {
        match get_line_ending(&line) {
            None => {}
            #[cfg(feature = "unicode-lines")]
            Some(LineEnding::VT) | Some(LineEnding::FF) | Some(LineEnding::PS) => {}
            ending => return ending,
        }
    }
    None
}

/// Returns the passed line's line ending, if any.
pub fn get_line_ending(line: &RopeSlice) -> Option<LineEnding> {
    // Last character as str.
    let g1 = line
        .slice(line.len_chars().saturating_sub(1)..)
        .as_str()
        .unwrap();

    // Last two characters as str, or empty str if they're not contiguous.
    // It's fine to punt on the non-contiguous case, because Ropey guarantees
    // that CRLF is always contiguous.
    let g2 = line
        .slice(line.len_chars().saturating_sub(2)..)
        .as_str()
        .unwrap_or("");

    // First check the two-character case for CRLF, then check the single-character case.
    LineEnding::from_str(g2).or_else(|| LineEnding::from_str(g1))
}

#[cfg(not(feature = "unicode-lines"))]
/// Returns the passed line's line ending, if any.
pub fn get_line_ending_of_str(line: &str) -> Option<LineEnding> {
    if line.ends_with("\u{000D}\u{000A}") {
        Some(LineEnding::Crlf)
    } else if line.ends_with('\u{000A}') {
        Some(LineEnding::LF)
    } else {
        None
    }
}

#[cfg(feature = "unicode-lines")]
/// Returns the passed line's line ending, if any.
pub fn get_line_ending_of_str(line: &str) -> Option<LineEnding> {
    if line.ends_with("\u{000D}\u{000A}") {
        Some(LineEnding::Crlf)
    } else if line.ends_with('\u{000A}') {
        Some(LineEnding::LF)
    } else if line.ends_with('\u{000B}') {
        Some(LineEnding::VT)
    } else if line.ends_with('\u{000C}') {
        Some(LineEnding::FF)
    } else if line.ends_with('\u{000D}') {
        Some(LineEnding::CR)
    } else if line.ends_with('\u{0085}') {
        Some(LineEnding::Nel)
    } else if line.ends_with('\u{2028}') {
        Some(LineEnding::LS)
    } else if line.ends_with('\u{2029}') {
        Some(LineEnding::PS)
    } else {
        None
    }
}

/// Returns the char index of the end of the given line, not including its line ending.
pub fn line_end_char_index(slice: &RopeSlice, line: usize) -> usize {
    slice.line_to_char(line + 1)
        - get_line_ending(&slice.line(line))
            .map(|le| le.len_chars())
            .unwrap_or(0)
}

pub fn line_end_byte_index(slice: &RopeSlice, line: usize) -> usize {
    slice.line_to_byte(line + 1)
        - get_line_ending(&slice.line(line))
            .map(|le| le.as_str().len())
            .unwrap_or(0)
}

/// Fetches line `line_idx` from the passed rope slice, sans any line ending.
pub fn line_without_line_ending<'a>(slice: &'a RopeSlice, line_idx: usize) -> RopeSlice<'a> {
    let start = slice.line_to_char(line_idx);
    let end = line_end_char_index(slice, line_idx);
    slice.slice(start..end)
}

/// Returns the char index of the end of the given RopeSlice, not including
/// any final line ending.
pub fn rope_end_without_line_ending(slice: &RopeSlice) -> usize {
    slice.len_chars() - get_line_ending(slice).map(|le| le.len_chars()).unwrap_or(0)
}
