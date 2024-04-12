use std::{
    collections::HashMap,
    fs::File,
    io::BufReader,
    path::{Path, PathBuf},
};

use lsp_types::{CompletionItem, CompletionItemKind, Documentation};
use serde::{Deserialize, Serialize};
use serde_json::{from_value, Value};

use crate::{
    errors::Error,
    loader::{config_dir, Dirs},
};

// https://code.visualstudio.com/docs/editor/userdefinedsnippets
// Example:
// "Print to console": {
// 	"prefix": "log",
// 	"body": [
// 		"console.log('$1');",
// 		"$2"
// 	],
// 	"description": "Log output to console"
// }
#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct Snippet {
    prefix: StrOrSeq, // string
    body: StrOrSeq,   // string
    description: Option<String>,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
#[serde(untagged)]
pub enum StrOrSeq {
    String(String),
    Array(Vec<String>),
}

impl StrOrSeq {
    fn to_string(&self) -> String {
        match self {
            StrOrSeq::String(s) => s.clone(),
            StrOrSeq::Array(v) => v.join("\n").clone(),
        }
    }
    fn first(&self) -> Option<String> {
        match self {
            StrOrSeq::String(s) => Some(s.clone()),
            StrOrSeq::Array(v) => v.first().and_then(|s| Some(s.clone())),
        }
    }
}

impl Snippet {
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

    fn description(&self) -> String {
        match &self.description {
            Some(s) => s.clone(),
            None => String::new(),
        }
    }
}

#[derive(Deserialize, Debug)]
pub struct Lang {
    name: String,
    snippets: HashMap<String, Snippet>,
}

impl Lang {
    pub fn get_global() -> Option<Lang> {
        let global_all: HashMap<String, Snippet> = read_names(&config_dir(Dirs::Snippets))
            .iter()
            .map(|p| parse(p, p.file_stem().unwrap().to_string_lossy().into_owned()).ok())
            .filter(|l| l.is_some())
            .map(|l| l.unwrap().snippets)
            // .flat_map(|m| m.iter())
            .fold(HashMap::new(), |mut acc, map| {
                acc.extend(map);
                acc
            });

        // TODO: project

        if global_all.is_empty() {
            None
        } else {
            Some(Lang {
                name: "global".to_owned(),
                snippets: global_all,
            })
        }
    }

    pub fn get_lang(lang_name: String) -> Result<Lang, Error> {
        let file_name = format!("{}.json", lang_name.clone().to_lowercase());
        let lang_file_path = config_dir(Dirs::Snippets).join(file_name);
        let lang = parse(&lang_file_path, lang_name)?;

        // TODO: project

        Ok(lang)
    }

    pub fn extend(&mut self, other: Lang) {
        self.snippets.extend(other.snippets);
    }

    pub fn get_completion_items(&self) -> Vec<CompletionItem> {
        self.snippets
            .iter()
            .map(|(_name, snippet)| snippet.to_completion_item())
            .filter(|s| s.is_some())
            .map(|s| s.unwrap())
            .collect()
    }
}

fn read_names(path: &PathBuf) -> Vec<PathBuf> {
    std::fs::read_dir(path)
        .map(|entries| {
            entries
                .filter_map(|entry| {
                    let entry = entry.ok()?;
                    let path = entry.path();
                    (path.extension()? == "code-snippet").then(|| path)
                })
                .collect()
        })
        .unwrap_or_default()
}

fn parse(lang_file_path: &PathBuf, name: String) -> Result<Lang, Error> {
    let file = File::open(lang_file_path)?;
    let rdr = BufReader::new(file);
    let lang = Lang {
        name,
        snippets: serde_json::from_reader(rdr)?,
    };
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
                assert!(lang.snippets.get("markdown b").is_some());
            }
            Err(err) => {
                eprintln!("{err}")
            }
        }
    }
}
