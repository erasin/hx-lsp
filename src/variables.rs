use aho_corasick::{AhoCorasick, PatternID};
use parking_lot::Mutex;
use rand::Rng;
use std::{collections::HashMap, path::PathBuf, sync::OnceLock};
use time::{
    OffsetDateTime, UtcOffset,
    format_description::{self, BorrowedFormatItem, OwnedFormatItem},
};
use uuid::Uuid;

use crate::encoding::char_is_word;

pub fn init() {
    init_time_offset();
    init_time_formats();
    init_variable_automaton();
}

#[derive(Debug, Default)]
pub struct VariableInit {
    pub file_path: PathBuf,
    pub work_path: PathBuf,
    pub line_text: String,
    pub current_word: String,
    pub selected_text: String,
    pub line_pos: usize,
    pub cursor_pos: usize,
    pub clipboard: Option<String>,
}

/// 兼容 [vscode snippet variables](https://code.visualstudio.com/docs/editor/userdefinedsnippets#_variables)
#[derive(Debug, Clone)]
pub enum Variables {
    TmSelectedText,
    TmCurrentLine,
    TmCurrentWord,
    TmLineIndex,
    TmLineNumber,
    TmFilename,
    TmFilenameBase,
    TmDirectory,
    TmFilepath,
    RelativeFilepath,
    Clipboard,
    WorkspaceName,
    WorkspaceFolder,
    CursorIndex,
    CursorNumber,

    CurrentYear,
    CurrentYearShort,
    CurrentMonth,
    CurrentMonthName,
    CurrentMonthNameShort,
    CurrentDate,
    CurrentDayName,
    CurrentDayNameShort,
    CurrentHour,
    CurrentMinute,
    CurrentSecond,
    CurrentSecondsUnix,
    CurrentTimezoneOffset,

    // 时间
    Random,
    RandomHex,
    Uuid,

    // 注释
    BlockCommentStart,
    BlockCommentEnd,
    LineComment,
}

impl std::fmt::Display for Variables {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Variables::TmSelectedText => "TM_SELECTED_TEXT",
                Variables::TmCurrentLine => "TM_CURRENT_LINE",
                Variables::TmCurrentWord => "TM_CURRENT_WORD",
                Variables::TmLineIndex => "TM_LINE_INDEX",
                Variables::TmLineNumber => "TM_LINE_NUMBER",
                Variables::TmFilename => "TM_FILENAME",
                Variables::TmFilenameBase => "TM_FILENAME_BASE",
                Variables::TmDirectory => "TM_DIRECTORY",
                Variables::TmFilepath => "TM_FILEPATH",
                Variables::RelativeFilepath => "RELATIVE_FILEPATH",
                Variables::Clipboard => "CLIPBOARD",
                Variables::WorkspaceName => "WORKSPACE_NAME",
                Variables::WorkspaceFolder => "WORKSPACE_FOLDER",
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
    /// 获取所有支持的变量类型
    fn all() -> impl Iterator<Item = Self> {
        use Variables::*;
        [
            // 基础变量
            TmSelectedText,
            TmCurrentLine,
            TmCurrentWord,
            TmLineIndex,
            TmLineNumber,
            TmFilename,
            TmFilenameBase,
            TmDirectory,
            TmFilepath,
            RelativeFilepath,
            Clipboard,
            WorkspaceName,
            WorkspaceFolder,
            CursorIndex,
            CursorNumber,
            // 时间相关
            CurrentYear,
            CurrentYearShort,
            CurrentMonth,
            CurrentMonthName,
            CurrentMonthNameShort,
            CurrentDate,
            CurrentDayName,
            CurrentDayNameShort,
            CurrentHour,
            CurrentMinute,
            CurrentSecond,
            CurrentSecondsUnix,
            CurrentTimezoneOffset,
            // 随机值
            Random,
            RandomHex,
            Uuid,
            // 注释
            BlockCommentStart,
            BlockCommentEnd,
            LineComment,
        ]
        .into_iter()
    }

    /// 解析变量值
    pub(crate) fn resolve(&self, init: &VariableInit) -> String {
        match self {
            // 基础变量
            Self::TmSelectedText => init.selected_text.clone(),
            Self::TmCurrentLine => init.line_text.clone(),
            Self::TmCurrentWord => init.current_word.clone(),
            Self::TmLineIndex => init.line_pos.to_string(),
            Self::TmLineNumber => (init.line_pos + 1).to_string(),
            Self::TmFilename => file_name(&init.file_path),
            Self::TmFilenameBase => file_name_base(&init.file_path),
            Self::TmDirectory => file_directory(&init.file_path),
            Self::TmFilepath => path_to_str(&init.file_path),
            Self::RelativeFilepath => path_to_str(&init.file_path), // TODO: 实现相对路径
            Self::Clipboard => init.clipboard.clone().unwrap_or_default(),
            Self::WorkspaceName => file_name(&init.work_path),
            Self::WorkspaceFolder => path_to_str(&init.work_path),
            Self::CursorIndex => init.cursor_pos.to_string(),
            Self::CursorNumber => (init.cursor_pos + 1).to_string(),

            // 时间相关
            Self::CurrentYear => time_format(&self.to_string()),
            Self::CurrentYearShort => time_format(&self.to_string()),
            Self::CurrentMonth => time_format(&self.to_string()),
            Self::CurrentMonthName => time_format(&self.to_string()),
            Self::CurrentMonthNameShort => time_format(&self.to_string()),
            Self::CurrentDate => time_format(&self.to_string()),
            Self::CurrentDayName => time_format(&self.to_string()),
            Self::CurrentDayNameShort => time_format(&self.to_string()),
            Self::CurrentHour => time_format(&self.to_string()),
            Self::CurrentMinute => time_format(&self.to_string()),
            Self::CurrentSecond => time_format(&self.to_string()),
            Self::CurrentSecondsUnix => time_format(&self.to_string()),
            Self::CurrentTimezoneOffset => current_timezone_offset(),

            // 随机值
            Self::Random => random_base10(6),
            Self::RandomHex => random_hex(6),
            Self::Uuid => Uuid::new_v4().to_string(),

            // 注释（需要语言上下文）
            Self::BlockCommentStart => self.to_string(), // 示例值，需根据语言调整
            Self::BlockCommentEnd => self.to_string(),
            Self::LineComment => self.to_string(),
        }
    }

    /// 批量替换文本中的变量
    pub fn replace_all(text: &str, init: &VariableInit) -> String {
        let automaton = init_variable_automaton();
        let mut replacements = Vec::new();

        for mat in automaton.find_iter(text) {
            let var = match Self::from_pattern_id(mat.pattern()) {
                Some(v) => v,
                None => continue,
            };
            let replacement = var.resolve(init);
            replacements.push((mat.range(), replacement));
        }

        build_replaced_string(text, replacements)
    }

    /// 从模式ID解析变量类型
    fn from_pattern_id(id: PatternID) -> Option<Self> {
        let index = id.as_usize() / 2; // 每个变量有2个模式
        Self::all().nth(index)
    }
}

/// 构建替换后的字符串（无锁操作）
fn build_replaced_string(
    text: &str,
    replacements: Vec<(std::ops::Range<usize>, String)>,
) -> String {
    let mut result = String::with_capacity(text.len());
    let mut last_end = 0;

    for (range, replacement) in replacements {
        result.push_str(&text[last_end..range.start]);
        result.push_str(&replacement);
        last_end = range.end;
    }

    result.push_str(&text[last_end..]);
    result
}

/// 初始化时区偏移
fn init_time_offset() -> &'static UtcOffset {
    // 时区偏移缓存
    // time local offset not support multi-thread
    static TIME_OFFSET: OnceLock<UtcOffset> = OnceLock::new();

    TIME_OFFSET.get_or_init(|| {
        OffsetDateTime::now_local()
            .unwrap_or_else(|_| OffsetDateTime::now_utc())
            .offset()
    })
}

/// 初始化时间格式缓存
fn init_time_formats() -> &'static Mutex<HashMap<&'static str, Vec<OwnedFormatItem>>> {
    // 时间格式
    static TIME_FORMAT_CACHE: OnceLock<Mutex<HashMap<&'static str, Vec<OwnedFormatItem>>>> =
        OnceLock::new();
    TIME_FORMAT_CACHE.get_or_init(|| {
        let mut map = HashMap::new();
        let formats = [
            ("CURRENT_YEAR", "[year]"),
            ("CURRENT_YEAR_SHORT", "[year repr:last_two]"),
            ("CURRENT_MONTH", "[month]"),
            ("CURRENT_MONTH_NAME", "[month repr:long]"),
            ("CURRENT_MONTH_NAME_SHORT", "[month repr:short]"),
            ("CURRENT_DATE", "[day]"),
            ("CURRENT_DAY_NAME", "[weekday repr:long]"),
            ("CURRENT_DAY_NAME_SHORT", "[weekday repr:short]"),
            ("CURRENT_HOUR", "[hour repr:24]"),
            ("CURRENT_MINUTE", "[minute]"),
            ("CURRENT_SECOND", "[second]"),
            (
                "CURRENT_SECONDS_UNIX",
                "[unix_timestamp precision:nanosecond]",
            ),
        ];

        for (key, fmt) in formats {
            if let Ok(parsed) = format_description::parse(fmt) {
                // 转换为拥有所有权的格式项
                let v = convert_to_owned(parsed);
                map.insert(key, v);
            }
        }
        Mutex::new(map)
    })
}

/// 将 BorrowedFormatItem 转换为 OwnedFormatItem
fn convert_to_owned<'a>(items: Vec<BorrowedFormatItem<'a>>) -> Vec<OwnedFormatItem> {
    items.iter().map(|item| item.into()).collect()
}

/// 获取当前时间（带缓存时区）
fn current_time() -> OffsetDateTime {
    OffsetDateTime::now_utc().to_offset(*init_time_offset())
}

fn current_timezone_offset() -> String {
    current_time().offset().to_string()
}

fn time_format(fmt: &str) -> String {
    let cache = init_time_formats();
    let lock = cache.lock();

    if let Some(format) = lock.get(fmt) {
        // 使用 OwnedFormatItem 进行格式化
        current_time()
            .format(&format)
            .unwrap_or_else(|_| String::from(fmt))
    } else {
        fmt.to_owned()
    }
}

/// 初始化变量自动机
fn init_variable_automaton() -> &'static AhoCorasick {
    // 变量自动机缓存
    static VARIABLE_AUTOMATON: OnceLock<AhoCorasick> = OnceLock::new();

    VARIABLE_AUTOMATON.get_or_init(|| {
        let patterns: Vec<String> = Variables::all()
            .flat_map(|var| [format!("${var}"), format!("${{{var}}}")])
            .collect();

        AhoCorasick::builder()
            .ascii_case_insensitive(true)
            .match_kind(aho_corasick::MatchKind::LeftmostLongest)
            .build(patterns)
            .expect("Failed to build Aho-Corasick automaton")
    })
}

/// 生成指定位数的随机数
fn random_base10(len: usize) -> String {
    let mut rng = rand::rng();
    (0..len)
        .map(|_| rng.random_range(0..=9).to_string())
        .collect()
}

/// 生成指定位数的十六进制随机数
fn random_hex(len: usize) -> String {
    let mut rng = rand::rng();
    (0..len)
        .map(|_| format!("{:x}", rng.random_range(0..16)))
        .collect()
}

/// 安全获取路径文件名
fn file_name(path: &PathBuf) -> String {
    path.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or_default()
        .to_string()
}

/// 获取无扩展名的文件名
fn file_name_base(path: &PathBuf) -> String {
    let name = file_name(path);
    name.chars()
        .take_while(|&c| char_is_word(c) && c != '.')
        .collect()
}

fn file_directory(path: &PathBuf) -> String {
    path.parent()
        .and_then(|p| p.to_str())
        .unwrap_or("")
        .to_owned()
}

/// 路径转字符串
fn path_to_str(path: &PathBuf) -> String {
    path.to_str().unwrap_or_default().to_string()
}

#[cfg(test)]
mod test {
    use copypasta::{ClipboardContext, ClipboardProvider};

    use super::init_variable_automaton;

    #[test]
    fn test_var() {
        init_variable_automaton();
    }

    #[test]
    fn test_clipboard() {
        let mut ctx = ClipboardContext::new().unwrap();
        let msg = "Hello!";
        ctx.set_contents(msg.to_owned()).unwrap();
        let content = ctx.get_contents().unwrap();
        assert_eq!(msg, content, "{msg},{content}");
    }
}
