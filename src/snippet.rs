use std::{collections::HashMap, fs::File, io::BufReader, path::PathBuf};

use json_comments::StripComments;
use lsp_types::{CompletionItem, CompletionItemKind};
use serde::{de::DeserializeOwned, Deserialize, Deserializer, Serialize, Serializer};

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
    pub fn get_global(project_root: &PathBuf) -> Result<Snippets, Error> {
        let global_all: HashMap<String, Snippet> = [
            read_names(&project_root.join(".helix").join(Dirs::Snippets.to_string())),
            read_names(&config_dir(Dirs::Snippets)),
        ]
        .concat()
        .into_iter()
        .rev()
        .map(|p| parse::<Snippets>(&p, p.file_stem().unwrap().to_string_lossy().into_owned()).ok())
        .filter(|l| l.is_some())
        .map(|l| l.unwrap().snippets)
        .fold(HashMap::new(), |mut acc, map| {
            acc.extend(map);
            acc
        });

        if global_all.is_empty() {
            Err(Error::NotFound("Global Snippets".to_owned()))
        } else {
            Ok(Snippets {
                name: "global".to_owned(),
                snippets: global_all,
            })
        }
    }

    /// 获取 XDG_CONFIG_HOME 下的 `langid.json` 语言文件
    pub fn get_lang(lang_name: String, project_root: &PathBuf) -> Result<Snippets, Error> {
        let file_name = format!("{}.json", lang_name.clone().to_lowercase());
        let snippets = [
            project_root
                .join(".helix")
                .join(Dirs::Snippets.to_string())
                .join(&file_name),
            config_dir(Dirs::Snippets).join(&file_name),
        ]
        .into_iter()
        .rev()
        .filter(|p| p.exists())
        .map(|p| parse::<Snippets>(&p, lang_name.to_owned()))
        .filter(|l| l.is_ok())
        .map(|l| l.unwrap())
        .fold(
            Snippets::new(lang_name.to_owned(), HashMap::new()),
            |mut acc, map| {
                acc.extend(map);
                acc
            },
        );

        Ok(snippets)
    }

    /// 合并 snippets
    pub fn extend(&mut self, other: Snippets) {
        self.snippets.extend(other.snippets);
    }

    /// 转换 snippets 为 lsp 的提示类型
    pub fn to_completion_items(&self) -> Vec<CompletionItem> {
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

#[cfg(test)]
mod test {

    use super::Snippets;

    #[test]
    fn test_get_lang() {
        let root = std::env::current_dir().ok().unwrap();
        let lang = Snippets::get_lang("markdown".to_owned(), &root);

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
