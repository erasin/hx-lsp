use std::{collections::HashMap, fs::File, io::BufReader, path::PathBuf};

use json_comments::StripComments;
use lsp_types::{CompletionItem, CompletionItemKind};
use serde::{Deserialize, Serialize};

use crate::{
    errors::Error,
    loader::{config_dir, Dirs},
};

/// 代码片段
/// 兼容 <https://code.visualstudio.com/docs/editor/userdefinedsnippets>
///
/// Example:
/// ```json
/// {
/// "Print to console": {
/// 	"prefix": "log",
/// 	"body": [
/// 		"console.log('$1');",
/// 		"$2"
/// 	],
/// 	"description": "Log output to console"
/// }
/// }
/// ```
#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct Snippet {
    prefix: StrOrSeq, // string
    body: StrOrSeq,   // string
    description: Option<String>,
}

/// `String` 或者 `Vec<String>`
#[derive(Deserialize, Serialize, Clone, Debug)]
#[serde(untagged)]
pub enum StrOrSeq {
    String(String),
    Array(Vec<String>),
}

impl ToString for StrOrSeq {
    /// `Vec<String>` 使用 `\n` 组合为 String
    fn to_string(&self) -> String {
        match self {
            StrOrSeq::String(s) => s.clone(),
            StrOrSeq::Array(v) => v.join("\n").clone(),
        }
    }
}

impl StrOrSeq {
    /// 获取第一个元素
    fn first(&self) -> Option<String> {
        match self {
            StrOrSeq::String(s) => Some(s.clone()),
            StrOrSeq::Array(v) => v.first().and_then(|s| Some(s.clone())),
        }
    }
}

impl Snippet {
    /// 转换为 lsp 类型 CompletionItem
    fn to_completion_item(&self) -> Option<CompletionItem> {
        if let Some(prefix) = self.prefix.first() {
            let mut c = CompletionItem::new_simple(prefix, self.description());
            c.kind = Some(CompletionItemKind::SNIPPET);
            c.insert_text = Some(self.body.to_string());
            Some(c)
        } else {
            None
        }
    }

    /// 获取 description, 兼容空对象
    fn description(&self) -> String {
        match &self.description {
            Some(s) => s.clone(),
            None => String::new(),
        }
    }
}

/// 语言包
#[derive(Deserialize, Debug)]
pub struct Lang {
    name: String,
    snippets: HashMap<String, Snippet>,
}

impl Default for Lang {
    fn default() -> Self {
        Lang::new("default".to_owned(), HashMap::new())
    }
}

impl Lang {
    pub fn new(name: String, snippets: HashMap<String, Snippet>) -> Lang {
        Lang { name, snippets }
    }

    /// 获取 XDG_CONFIG_HOME 下的 `code-snippets` 全局片段文件
    /// 获取 workspace 项目目录下的 `code-snippets` 文件
    pub fn get_global() -> Result<Lang, Error> {
        let global_all: HashMap<String, Snippet> = read_names(&config_dir(Dirs::Snippets))
            .into_iter()
            .map(|p| parse(&p, p.file_stem().unwrap().to_string_lossy().into_owned()).ok())
            .filter(|l| l.is_some())
            .map(|l| l.unwrap().snippets)
            .fold(HashMap::new(), |mut acc, map| {
                acc.extend(map);
                acc
            });

        // TODO: project

        if global_all.is_empty() {
            Err(Error::NotFound("Global Snippets".to_owned()))
        } else {
            Ok(Lang {
                name: "global".to_owned(),
                snippets: global_all,
            })
        }
    }

    /// 获取 XDG_CONFIG_HOME 下的 `langid.json` 语言文件
    pub fn get_lang(lang_name: String) -> Result<Lang, Error> {
        let file_name = format!("{}.json", lang_name.clone().to_lowercase());
        let lang_file_path = config_dir(Dirs::Snippets).join(file_name);
        let lang = parse(&lang_file_path, lang_name)?;

        // TODO: project

        Ok(lang)
    }

    /// 合并 snippets
    pub fn extend(&mut self, other: Lang) {
        self.snippets.extend(other.snippets);
    }

    /// 转换 snippets 为 lsp 的提示类型
    pub fn get_completion_items(&self) -> Vec<CompletionItem> {
        self.snippets
            .iter()
            .map(|(_name, snippet)| snippet.to_completion_item())
            .filter(|s| s.is_some())
            .map(|s| s.unwrap())
            .collect()
    }
}

/// 读取文件夹内容，获取全局 `*.code-snippets` 文件路径
fn read_names(path: &PathBuf) -> Vec<PathBuf> {
    std::fs::read_dir(path)
        .map(|entries| {
            entries
                .filter_map(|entry| {
                    let entry = entry.ok()?;
                    let path = entry.path();
                    (path.extension()? == "code-snippets").then(|| path)
                })
                .collect()
        })
        .unwrap_or_default()
}

/// 解析 `code-snippets json` 文件
fn parse(lang_file_path: &PathBuf, name: String) -> Result<Lang, Error> {
    let file = File::open(lang_file_path)?;
    let reader = BufReader::new(file);

    // 过滤注释内容
    let json_data = StripComments::new(reader);

    // 日志记录错误
    let snippets = match serde_json::from_reader(json_data) {
        Ok(s) => s,
        Err(err) => {
            log::error!("parse fail: {err:?}");
            return Err(err.into());
        }
    };

    let lang = Lang { name, snippets };
    Ok(lang)
}

#[cfg(test)]
mod test {

    use super::Lang;

    #[test]
    fn test_get_lang() {
        let lang = Lang::get_lang("markdown".to_owned());

        // eprintln!("{lang:?}");
        match lang {
            Ok(lang) => {
                assert_eq!(lang.name, "markdown".to_owned(),);
                assert!(lang.snippets.get("time").is_some());
            }
            Err(err) => {
                eprintln!("{err}")
            }
        }
    }
}
