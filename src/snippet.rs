use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use anyhow::Result;
use async_lsp::lsp_types::{CompletionItem, CompletionItemKind};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

use crate::{
    fuzzy::fuzzy_match,
    loader::{Dirs, config_dir},
    parser::{Parser, StrOrSeq, parse},
    variables::{VariableInit, Variables},
};

/// 代码片段
/// 兼容 <https://code.visualstudio.com/docs/editor/userdefinedsnippets>
///
/// Example:
/// ```json
/// {
/// "Print to console": {
///    "prefix": "log",
///    "body": [
///       "console.log('$1');",
///       "$2"
///    ],
///    "description": "Log output to console"
/// }
/// }
/// ```
#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct Snippet {
    prefix: StrOrSeq, // string
    body: StrOrSeq,   // string
    description: Option<String>,
}

fn to_completion_item(prefix: String, body: String, detail: String) -> CompletionItem {
    let mut c = CompletionItem::new_simple(prefix, detail);
    c.kind = Some(CompletionItemKind::SNIPPET);
    c.insert_text = Some(body);
    c
}

impl Snippet {
    /// 转换为 lsp 类型 CompletionItem
    fn to_completion_item(&self, variable_init: &VariableInit) -> Vec<CompletionItem> {
        let body = self.body.to_string();
        let body = Variables::replace_all(&body, variable_init);

        match &self.prefix {
            StrOrSeq::String(s) => {
                [to_completion_item(s.to_owned(), body, self.description())].to_vec()
            }
            StrOrSeq::Array(arr) => arr
                .iter()
                .map(|s| to_completion_item(s.to_owned(), body.to_owned(), self.description()))
                .collect(),
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

// TODO: watch file or restart lsp
fn snippets_list() -> &'static Mutex<HashMap<String, Snippets>> {
    static SNIPPETS: OnceLock<Mutex<HashMap<String, Snippets>>> = OnceLock::new();
    SNIPPETS.get_or_init(|| Mutex::new(HashMap::new()))
}

/// 语言包
#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct Snippets {
    name: String,
    snippets: HashMap<String, Snippet>,
}

impl Default for Snippets {
    fn default() -> Self {
        Snippets::new("default".to_owned(), HashMap::new())
    }
}

impl Parser for Snippets {
    type Item = Snippet;

    fn set_name(&mut self, name: String) {
        self.name = name;
    }

    fn set_hasmap(&mut self, hs: HashMap<String, Self::Item>) {
        self.snippets = hs;
    }
}

impl Snippets {
    pub fn new(name: String, snippets: HashMap<String, Snippet>) -> Snippets {
        Snippets { name, snippets }
    }

    /// 获取 XDG_CONFIG_HOME 下的 `code-snippets` 全局片段文件
    /// 获取 workspace 项目目录下的 `code-snippets` 文件
    ///
    /// # Panics
    ///
    /// Panics if .
    ///
    /// # Errors
    ///
    /// This function will return an error if .
    pub fn get_global(project_root: &Path) -> Snippets {
        let name = "global";
        // check have
        let mut snippets = snippets_list().lock();
        match snippets.get(name) {
            Some(has) => has.clone(),
            None => {
                let global_snippets = from_files(
                    name.to_owned(),
                    [
                        read_names(&project_root.join(".helix").join(Dirs::Snippets.to_string())),
                        read_names(&config_dir(Dirs::Snippets)),
                    ]
                    .concat(),
                );

                snippets.insert(name.to_owned(), global_snippets.clone());
                global_snippets
            }
        }
    }

    /// 获取 XDG_CONFIG_HOME 下的 `langid.json` 语言文件
    pub fn get_lang(lang_name: String, project_root: &Path) -> Snippets {
        let mut snippets_list = snippets_list().lock();
        match snippets_list.get(&lang_name) {
            Some(has) => has.clone(),
            None => {
                let file_name = format!("{}.json", lang_name.clone().to_lowercase());
                let lang_snippets = from_files(
                    lang_name.clone(),
                    [
                        project_root
                            .join(".helix")
                            .join(Dirs::Snippets.to_string())
                            .join(&file_name),
                        config_dir(Dirs::Snippets).join(&file_name),
                    ]
                    .to_vec(),
                );

                snippets_list.insert(lang_name, lang_snippets.clone());
                lang_snippets
            }
        }
    }

    /// 合并 snippets
    pub fn extend(&mut self, other: Snippets) {
        self.snippets.extend(other.snippets);
    }

    /// 转换 snippets 为 lsp 的提示类型
    pub fn to_completion_items(&self, variable_init: &VariableInit) -> Vec<CompletionItem> {
        self.snippets
            .values()
            .map(|snippet| snippet.to_completion_item(variable_init))
            .fold(Vec::<CompletionItem>::new(), |mut a, b| {
                a.extend(b);
                a
            })
    }

    pub fn filter(&self, word: &str) -> Result<Snippets> {
        let names: HashMap<String, String> = self
            .clone()
            .snippets
            .into_iter()
            .map(|(title, snippet)| (snippet.prefix.to_string(), title))
            .collect();

        let re = fuzzy_match(word, names.clone().into_keys(), false)
            .into_iter()
            .filter_map(|(name, _)| names.get(&name))
            .filter_map(|f| self.snippets.get_key_value(f))
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();

        Ok(Snippets::new(self.name.clone(), re))
    }
}

fn from_files(name: String, files: Vec<PathBuf>) -> Snippets {
    files
        .into_iter()
        .rev()
        .filter(|p| p.exists())
        .filter_map(|p| {
            parse::<Snippets>(&p, p.file_stem().unwrap().to_string_lossy().into_owned()).ok()
        })
        .fold(Snippets::new(name, HashMap::new()), |mut acc, map| {
            acc.extend(map);
            acc
        })
}

/// 读取文件夹内容，获取全局 `*.code-snippets` 文件路径
fn read_names(path: &PathBuf) -> Vec<PathBuf> {
    std::fs::read_dir(path)
        .map(|entries| {
            entries
                .filter_map(|entry| {
                    let entry = entry.ok()?;
                    let path = entry.path();
                    (path.extension()? == "code-snippets").then_some(path)
                })
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod test {

    use super::Snippets;

    #[test]
    fn test_get_lang() {
        let root = std::env::current_dir().ok().unwrap();
        let lang = Snippets::get_lang("markdown".to_owned(), &root);

        println!("{:?}", lang);
        assert_eq!(lang.name, "markdown".to_owned(),);
        assert!(lang.snippets.contains_key("markdown a"));
    }
}
