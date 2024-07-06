use std::{path::PathBuf, sync::OnceLock};

use aho_corasick::AhoCorasick;
use rand::distributions::{Distribution, Uniform};
use time::{format_description, OffsetDateTime, UtcOffset};
use uuid::Uuid;

use crate::encoding::char_is_word;

#[derive(Debug, Default)]
pub struct VariableInit {
    pub file_path: PathBuf,
    pub work_path: PathBuf,
    pub line_text: String,
    pub current_word: String,
    pub selected_text: String,
    pub line_pos: usize,
    pub clipboard: Option<String>,
}

/// 兼容 [vscode snippet variables](https://code.visualstudio.com/docs/editor/userdefinedsnippets#_variables)
#[derive(Debug, Clone)]
pub enum Variables {
    // The following variables can be used:
    /// The currently selected text or the empty string
    TmSelectedText(String),
    /// The contents of the current line
    TmCurrentLine(String),
    /// The contents of the word under cursor or the empty string
    TmCurrentWord(String),
    /// The zero-index based line number
    TmLineIndex(usize),
    /// The one-index based line number
    TmLineNumber(usize),
    /// The filename of the current document
    TmFilename(PathBuf),
    /// The filename of the current document without its extensions
    TmFilenameBase(PathBuf),
    /// The directory of the current document
    TmDirectory(PathBuf),
    /// The full file path of the current document
    TmFilepath(PathBuf),
    /// The relative (to the opened workspace or folder) file path of the current document
    RelativeFilepath(PathBuf),
    /// The contents of your clipboard
    Clipboard(String),
    /// The name of the opened workspace or folder
    WorkspaceName(PathBuf),
    /// The path of the opened workspace or folder
    WorkspaceFolder(PathBuf),
    /// The zero-index based cursor number
    CursorIndex,
    /// The one-index based cursor number
    CursorNumber,

    // For inserting the current date and time:
    /// The current year
    CurrentYear,
    /// The current year's last two digits
    CurrentYearShort,
    /// The month as two digits (example '02')
    CurrentMonth,
    /// The full name of the month (example 'July')
    CurrentMonthName,
    /// The short name of the month (example 'Jul')
    CurrentMonthNameShort,
    /// The day of the month as two digits (example '08')
    CurrentDate,
    /// The name of day (example 'Monday')
    CurrentDayName,
    /// The short name of the day (example 'Mon')
    CurrentDayNameShort,
    /// The current hour in 24-hour clock format
    CurrentHour,
    /// The current minute as two digits
    CurrentMinute,
    /// The current second as two digits
    CurrentSecond,
    /// The number of seconds since the Unix epoch
    CurrentSecondsUnix,
    /// The current UTC time zone offset as +HH:MM or -HH:MM (example -07:00).
    CurrentTimezoneOffset,

    // For inserting random values:
    /// 6 random Base-10 digits
    Random,
    /// 6 random Base-16 digits
    RandomHex,
    /// A Version 4 UUID
    Uuid,

    // For inserting line or block comments, honoring the current language:
    /// Example output: in PHP /* or in HTML <!--
    BlockCommentStart,
    /// Example output: in PHP */ or in HTML -->
    BlockCommentEnd,
    /// Example output: in PHP `//`
    LineComment,
}

impl std::fmt::Display for Variables {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Variables::TmSelectedText(_) => "TM_SELECTED_TEXT",
                Variables::TmCurrentLine(_) => "TM_CURRENT_LINE",
                Variables::TmCurrentWord(_) => "TM_CURRENT_WORD",
                Variables::TmLineIndex(_) => "TM_LINE_INDEX",
                Variables::TmLineNumber(_) => "TM_LINE_NUMBER",
                Variables::TmFilename(_) => "TM_FILENAME",
                Variables::TmFilenameBase(_) => "TM_FILENAME_BASE",
                Variables::TmDirectory(_) => "TM_DIRECTORY",
                Variables::TmFilepath(_) => "TM_FILEPATH",
                Variables::RelativeFilepath(_) => "RELATIVE_FILEPATH",
                Variables::Clipboard(_) => "CLIPBOARD",
                Variables::WorkspaceName(_) => "WORKSPACE_NAME",
                Variables::WorkspaceFolder(_) => "WORKSPACE_FOLDER",
                Variables::CursorIndex => "CURSOR_INDEX",
                Variables::CursorNumber => "CURSOR_NUMBER",

                Variables::CurrentYear => "CURRENT_YEAR",
                Variables::CurrentYearShort => "CURRENT_YEAR_SHORT",
                Variables::CurrentMonth => "CURRENT_MONTH",
                Variables::CurrentMonthName => "CURRENT_MONTH_NAME",
                Variables::CurrentMonthNameShort => "CURRENT_MONTH_NAME_SHORT",
                Variables::CurrentDate => "CURRENT_DATE",
                Variables::CurrentDayName => "CURRENT_DAY_NAME",
                Variables::CurrentDayNameShort => "CURRENT_DAY_NAME_SHORT",
                Variables::CurrentHour => "CURRENT_HOUR",
                Variables::CurrentMinute => "CURRENT_MINUTE",
                Variables::CurrentSecond => "CURRENT_SECOND",
                Variables::CurrentSecondsUnix => "CURRENT_SECONDS_UNIX",
                Variables::CurrentTimezoneOffset => "CURRENT_TIMEZONE_OFFSET",

                Variables::Random => "RANDOM",
                Variables::RandomHex => "RANDOM_HEX",
                Variables::Uuid => "UUID",

                Variables::BlockCommentStart => "BLOCK_COMMENT_START",
                Variables::BlockCommentEnd => "BLOCK_COMMENT_END",
                Variables::LineComment => "LINE_COMMENT",
            }
        )
    }
}

impl Variables {
    /// 转换字符串内的变量
    pub fn convert_all(text: &String, init: &VariableInit) -> String {
        let mut text = text.clone();
        Variables::to_vec(init)
            .into_iter()
            .for_each(|f| text = f.convert(&text));

        text
    }

    /// 获可支持的字段
    fn to_vec(init: &VariableInit) -> Vec<Variables> {
        [
            Variables::TmSelectedText(init.selected_text.clone()),
            Variables::TmCurrentLine(init.line_text.clone()),
            Variables::TmCurrentWord(init.current_word.clone()),
            Variables::TmLineIndex(init.line_pos),
            Variables::TmLineNumber(init.line_pos + 1),
            Variables::TmFilenameBase(init.file_path.clone()),
            Variables::TmFilename(init.file_path.clone()),
            Variables::TmDirectory(init.file_path.clone()),
            Variables::TmFilepath(init.file_path.clone()),
            Variables::RelativeFilepath(init.file_path.clone()),
            Variables::Clipboard(init.clipboard.clone().unwrap_or(Default::default())),
            Variables::WorkspaceName(init.work_path.clone()),
            Variables::WorkspaceFolder(init.work_path.clone()),
            Variables::CursorIndex,
            Variables::CursorNumber,
            Variables::CurrentYearShort,
            Variables::CurrentYear,
            Variables::CurrentMonthNameShort,
            Variables::CurrentMonthName,
            Variables::CurrentMonth,
            Variables::CurrentDate,
            Variables::CurrentDayNameShort,
            Variables::CurrentDayName,
            Variables::CurrentHour,
            Variables::CurrentMinute,
            Variables::CurrentSecond,
            Variables::CurrentSecondsUnix,
            Variables::CurrentTimezoneOffset,
            Variables::Random,
            Variables::RandomHex,
            Variables::Uuid,
            // Variables::BlockCommentStart,
            // Variables::BlockCommentEnd,
            // Variables::LineComment,
        ]
        .to_vec()
    }

    /// 转化的内容
    fn to_value(&self) -> String {
        match self {
            Variables::TmSelectedText(str) => str.to_string(),
            Variables::TmCurrentLine(str) => str.to_string(),
            Variables::TmCurrentWord(str) => str.to_string(),
            Variables::TmLineIndex(line_pos) => line_pos.to_string(),
            Variables::TmLineNumber(line_pos) => line_pos.to_string(),
            Variables::TmFilename(file_path) => {
                file_path.file_name().unwrap().to_str().unwrap().to_string()
            }
            Variables::TmFilenameBase(file_path) => file_path
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .to_string()
                .chars()
                .take_while(|&ch| char_is_word(ch))
                .collect(),
            Variables::TmDirectory(file_path) => file_path
                .parent()
                .unwrap()
                .to_str()
                .unwrap_or("")
                .to_owned(),
            Variables::TmFilepath(file_path) => file_path.to_str().unwrap_or("").to_owned(),
            Variables::RelativeFilepath(file_path) => file_path.to_str().unwrap_or("").to_string(),
            Variables::Clipboard(s) => s.to_string(),
            Variables::WorkspaceName(work_path) => {
                work_path.file_name().unwrap().to_str().unwrap().to_string()
            }
            Variables::WorkspaceFolder(work_path) => work_path.to_str().unwrap_or("").to_string(),
            Variables::CursorIndex => self.to_string(),
            Variables::CursorNumber => self.to_string(),

            Variables::CurrentYear => OffsetDateTime::now_utc().year().to_string(),
            Variables::CurrentYearShort => time_format("[year repr:last_two]"),
            Variables::CurrentMonth => time_format("[month]"),
            Variables::CurrentMonthName => time_format("[month repr:long]"),
            Variables::CurrentMonthNameShort => time_format("[month repr:short]"),
            Variables::CurrentDate => time_format("[day]"),
            Variables::CurrentDayName => time_format("[weekday repr:long]"),
            Variables::CurrentDayNameShort => time_format("[weekday repr:short]"),
            Variables::CurrentHour => time_format("[hour repr:24]"),
            Variables::CurrentMinute => time_format("[minute]"),
            Variables::CurrentSecond => time_format("[second]"),
            Variables::CurrentSecondsUnix => time_format("[unix_timestamp precision:nanosecond]"),
            Variables::CurrentTimezoneOffset => OffsetDateTime::now_utc().offset().to_string(),

            Variables::Random => random(),
            Variables::RandomHex => random_hex(),
            Variables::Uuid => Uuid::new_v4().to_string(),

            Variables::BlockCommentStart => self.to_string(),
            Variables::BlockCommentEnd => self.to_string(),
            Variables::LineComment => self.to_string(),
        }
    }

    /// 替换处理
    pub fn convert(&self, text: &String) -> String {
        let str = self.to_string();
        let str_replace = self.to_value();
        if str_replace.is_empty() {
            return text.clone();
        }

        let patterns = &[format!("${str}"), format!("${{{str}}}")];
        let replace_with = &[str_replace.to_owned(), str_replace];

        let ac = AhoCorasick::builder()
            .ascii_case_insensitive(true)
            .build(patterns.into_iter())
            .unwrap();

        let re = ac
            .try_replace_all(text, replace_with)
            .unwrap_or(text.to_owned());
        re
    }
}

pub fn get_time_offset() -> &'static UtcOffset {
    // time local offset not support multi-thread
    static TIME_OFFSET: OnceLock<UtcOffset> = OnceLock::new();

    TIME_OFFSET.get_or_init(|| {
        OffsetDateTime::now_local()
            .unwrap_or(OffsetDateTime::now_utc())
            .offset()
    })
}

pub fn time_format(s: &str) -> String {
    match time_format_parse(s) {
        Ok(s) => s,
        Err(_e) => s.to_owned(),
    }
}

pub fn time_format_parse(s: &str) -> Result<String, Box<dyn std::error::Error>> {
    let format = format_description::parse(s)?;

    let s = OffsetDateTime::now_utc()
        .to_offset(*get_time_offset())
        .format(&format)?;

    Ok(s)
}

fn random() -> String {
    let step = Uniform::new(0, 9);
    let mut rng = rand::thread_rng();
    step.sample_iter(&mut rng)
        .take(6)
        .map(|f| f.to_string())
        .collect()
}

fn random_hex() -> String {
    const DIGITS: &[u8] = b"0123456789abcdef";
    let step = Uniform::new(0, DIGITS.len());
    let mut rng = rand::thread_rng();
    step.sample_iter(&mut rng)
        .take(6)
        .map(|x| DIGITS[x] as char)
        .collect()
}

#[cfg(test)]
mod test {

    use super::{VariableInit, Variables};

    #[test]
    fn test_convert() {
        let text = String::from("${CURRENT_YEAR_SHORT} or $CURRENT_YEAR_SHORT");

        let v = Variables::CurrentYearShort;
        let re = v.convert(&text);

        assert_eq!(re.len(), 8);
    }

    #[test]
    fn test_convert_all() {
        let text = String::from("${CURRENT_YEAR} or $CURRENT_YEAR_SHORT");

        let mut vars = VariableInit::default();
        let root = std::env::current_dir().ok().unwrap();
        vars.file_path = root.clone();
        vars.work_path = root.clone();

        let re = Variables::convert_all(&text, &vars);

        assert_eq!(re.len(), 10);
    }
}
