use aho_corasick::AhoCorasick;
use rand::distributions::{Distribution, Uniform};
use time::{format_description, OffsetDateTime};
use uuid::Uuid;

/// 兼容 [vscode snippet variables](https://code.visualstudio.com/docs/editor/userdefinedsnippets#_variables)
#[derive(Debug, Clone, Copy)]
pub enum Variables {
    // The following variables can be used:
    /// The currently selected text or the empty string
    TmSelectedText,
    /// The contents of the current line
    TmCurrentLine,
    /// The contents of the word under cursor or the empty string
    TmCurrentWord,
    /// The zero-index based line number
    TmLineIndex,
    /// The one-index based line number
    TmLineNumber,
    /// The filename of the current document
    TmFilename,
    /// The filename of the current document without its extensions
    TmFilenameBase,
    /// The directory of the current document
    TmDirectory,
    /// The full file path of the current document
    TmFilepath,
    /// The relative (to the opened workspace or folder) file path of the current document
    RelativeFilepath,
    /// The contents of your clipboard
    Clipboard,
    /// The name of the opened workspace or folder
    WorkspaceName,
    /// The path of the opened workspace or folder
    WorkspaceFolder,
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

impl ToString for Variables {
    fn to_string(&self) -> String {
        match self {
            Variables::TmSelectedText => "TM_SELECTED_TEXT".to_owned(),
            Variables::TmCurrentLine => "TM_CURRENT_LINE".to_owned(),
            Variables::TmCurrentWord => "TM_CURRENT_WORD".to_owned(),
            Variables::TmLineIndex => "TM_LINE_INDEX".to_owned(),
            Variables::TmLineNumber => "TM_LINE_NUMBER".to_owned(),
            Variables::TmFilename => "TM_FILENAME".to_owned(),
            Variables::TmFilenameBase => "TM_FILENAME_BASE".to_owned(),
            Variables::TmDirectory => "TM_DIRECTORY".to_owned(),
            Variables::TmFilepath => "TM_FILEPATH".to_owned(),
            Variables::RelativeFilepath => "RELATIVE_FILEPATH".to_owned(),
            Variables::Clipboard => "CLIPBOARD".to_owned(),
            Variables::WorkspaceName => "WORKSPACE_NAME".to_owned(),
            Variables::WorkspaceFolder => "WORKSPACE_FOLDER".to_owned(),
            Variables::CursorIndex => "CURSOR_INDEX".to_owned(),
            Variables::CursorNumber => "CURSOR_NUMBER".to_owned(),

            Variables::CurrentYear => "CURRENT_YEAR".to_owned(),
            Variables::CurrentYearShort => "CURRENT_YEAR_SHORT".to_owned(),
            Variables::CurrentMonth => "CURRENT_MONTH".to_owned(),
            Variables::CurrentMonthName => "CURRENT_MONTH_NAME".to_owned(),
            Variables::CurrentMonthNameShort => "CURRENT_MONTH_NAME_SHORT".to_owned(),
            Variables::CurrentDate => "CURRENT_DATE".to_owned(),
            Variables::CurrentDayName => "CURRENT_DAY_NAME".to_owned(),
            Variables::CurrentDayNameShort => "CURRENT_DAY_NAME_SHORT".to_owned(),
            Variables::CurrentHour => "CURRENT_HOUR".to_owned(),
            Variables::CurrentMinute => "CURRENT_MINUTE".to_owned(),
            Variables::CurrentSecond => "CURRENT_SECOND".to_owned(),
            Variables::CurrentSecondsUnix => "CURRENT_SECONDS_UNIX".to_owned(),
            Variables::CurrentTimezoneOffset => "CURRENT_TIMEZONE_OFFSET".to_owned(),

            Variables::Random => "RANDOM".to_owned(),
            Variables::RandomHex => "RANDOM_HEX".to_owned(),
            Variables::Uuid => "UUID".to_owned(),

            Variables::BlockCommentStart => "BLOCK_COMMENT_START".to_owned(),
            Variables::BlockCommentEnd => "BLOCK_COMMENT_END".to_owned(),
            Variables::LineComment => "LINE_COMMENT".to_owned(),
        }
    }
}

impl Variables {
    /// 转换字符串内的变量
    pub fn convert_all(text: &String) -> String {
        let mut text = text.clone();
        Variables::to_vec()
            .into_iter()
            .for_each(|f| text = f.convert(&text));

        text
    }

    /// 获可支持的字段
    fn to_vec() -> Vec<Variables> {
        [
            Variables::TmSelectedText,
            Variables::TmCurrentLine,
            Variables::TmCurrentWord,
            Variables::TmLineIndex,
            Variables::TmLineNumber,
            Variables::TmFilename,
            Variables::TmFilenameBase,
            Variables::TmDirectory,
            Variables::TmFilepath,
            Variables::RelativeFilepath,
            // Variables::Clipboard,
            Variables::WorkspaceName,
            Variables::WorkspaceFolder,
            // Variables::CursorIndex,
            // Variables::CursorNumber,
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
            Variables::TmSelectedText => self.to_string(),
            Variables::TmCurrentLine => self.to_string(),
            Variables::TmCurrentWord => self.to_string(),
            Variables::TmLineIndex => self.to_string(),
            Variables::TmLineNumber => self.to_string(),
            Variables::TmFilename => self.to_string(),
            Variables::TmFilenameBase => self.to_string(),
            Variables::TmDirectory => self.to_string(),
            Variables::TmFilepath => self.to_string(),
            Variables::RelativeFilepath => self.to_string(),
            Variables::Clipboard => self.to_string(),
            Variables::WorkspaceName => self.to_string(),
            Variables::WorkspaceFolder => self.to_string(),
            Variables::CursorIndex => self.to_string(),
            Variables::CursorNumber => self.to_string(),

            Variables::CurrentYear => OffsetDateTime::now_utc().year().to_string(),
            Variables::CurrentYearShort => year_short(),
            Variables::CurrentMonth => month(),
            Variables::CurrentMonthName => month_name(),
            Variables::CurrentMonthNameShort => month_name_short(),
            Variables::CurrentDate => OffsetDateTime::now_utc().day().to_string(),
            Variables::CurrentDayName => day_name(),
            Variables::CurrentDayNameShort => day_name_short(),
            Variables::CurrentHour => OffsetDateTime::now_utc().hour().to_string(),
            Variables::CurrentMinute => OffsetDateTime::now_utc().minute().to_string(),
            Variables::CurrentSecond => OffsetDateTime::now_utc().second().to_string(),
            Variables::CurrentSecondsUnix => OffsetDateTime::now_utc().unix_timestamp().to_string(),
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

fn year_short() -> String {
    let format = format_description::parse("[year repr:last_two]").unwrap();
    OffsetDateTime::now_utc().format(&format).unwrap()
}

fn month() -> String {
    let format = format_description::parse("[month]").unwrap();
    OffsetDateTime::now_utc().format(&format).unwrap()
}

fn month_name() -> String {
    let format = format_description::parse("[month repr:long]").unwrap();
    OffsetDateTime::now_utc().format(&format).unwrap()
}

fn month_name_short() -> String {
    let format = format_description::parse("[month repr:short]").unwrap();
    OffsetDateTime::now_utc().format(&format).unwrap()
}

fn day_name() -> String {
    let format = format_description::parse("[weekday repr:long]").unwrap();
    OffsetDateTime::now_utc().format(&format).unwrap()
}

fn day_name_short() -> String {
    let format = format_description::parse("[weekday repr:short] ").unwrap();
    OffsetDateTime::now_utc().format(&format).unwrap()
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
    use super::Variables;

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
        let re = Variables::convert_all(&text);

        assert_eq!(re.len(), 10);
    }
}
