use std::{collections::HashMap, path::PathBuf};

use lsp_types::{CompletionItem, CompletionItemKind};
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

use crate::{
    errors::Error,
    loader::{config_dir, Dirs},
    parser::{parse, Parser, StrOrSeq},
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

// TODO: watch file or restart lsp
static SNIPPETS: Lazy<Mutex<HashMap<String, Snippets>>> = Lazy::new(|| Mutex::new(HashMap::new()));

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
    pub fn get_global(project_root: &PathBuf) -> Snippets {
        let name = "global";
        // check have
        let mut snippets = SNIPPETS.lock();
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
    pub fn get_lang(lang_name: String, project_root: &PathBuf) -> Snippets {
        let mut snippets_list = SNIPPETS.lock();
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
    pub fn to_completion_items(&self) -> Vec<CompletionItem> {
        self.snippets
            .iter()
            .filter_map(|(_name, snippet)| snippet.to_completion_item())
            .collect()
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
        .fold(
            Snippets::new(name.to_owned(), HashMap::new()),
            |mut acc, map| {
                acc.extend(map);
                acc
            },
        )
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

#[cfg(test)]
mod test {

    use super::Snippets;

    #[test]
    fn test_get_lang() {
        let root = std::env::current_dir().ok().unwrap();
        let lang = Snippets::get_lang("markdown".to_owned(), &root);

        assert_eq!(lang.name, "markdown".to_owned(),);
        assert!(lang.snippets.get("time").is_some());
    }
}
