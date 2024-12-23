use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::OnceLock,
};

use async_lsp::lsp_types::{self, CodeAction, Range};
use parking_lot::Mutex;
use regex::Regex;
use ropey::Rope;
use serde::{Deserialize, Serialize};
use tracing as log;

use crate::{
    loader::{config_dir, Dirs},
    parser::{parse, Parser, StrOrSeq},
    variables::{VariableInit, Variables},
    Result,
};

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct Action {
    /// 捕捉, 支持单行或者两行
    title: String,
    catch: String,
    shell: StrOrSeq, // string
    description: Option<String>,
}

impl Action {
    /// 转换 lsp 格式
    fn to_code_action_item(&self, variable_init: &VariableInit) -> Option<CodeAction> {
        let shell = self.shell.to_string();
        let shell = Variables::convert_all(&shell, variable_init);

        let command = lsp_types::Command {
            title: "Run Test".to_string(),
            command: shell,
            arguments: None,
        };

        let action = CodeAction {
            title: self.title.clone(),
            kind: Some(lsp_types::CodeActionKind::EMPTY),
            // kind: Some("command".into()),
            command: Some(command),
            // diagnostics: Some(vec![diagnostic.clone()]),
            is_preferred: Some(true),
            diagnostics: None,
            disabled: None,
            data: None,
            ..Default::default()
        };

        Some(action)
    }

    /// 获取 description, 兼容空对象
    #[allow(dead_code)]
    fn description(&self) -> String {
        match &self.description {
            Some(s) => s.clone(),
            None => String::new(),
        }
    }
}

fn actions_list() -> &'static Mutex<HashMap<String, Actions>> {
    static ACTIONS: OnceLock<Mutex<HashMap<String, Actions>>> = OnceLock::new();
    ACTIONS.get_or_init(|| Mutex::new(HashMap::new()))
}

/// 语言包
#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct Actions {
    name: String,
    actions: HashMap<String, Action>,
}

impl Default for Actions {
    fn default() -> Self {
        Actions::new("default".to_owned(), HashMap::new())
    }
}

impl Parser for Actions {
    type Item = Action;

    fn set_name(&mut self, name: String) {
        self.name = name;
    }

    fn set_hasmap(&mut self, hs: HashMap<String, Self::Item>) {
        self.actions = hs;
    }
}

impl Actions {
    pub fn new(name: String, actions: HashMap<String, Action>) -> Actions {
        Actions { name, actions }
    }

    pub fn get_lang(lang_name: String, doc: &Rope, range: &Range, project_root: &Path) -> Actions {
        let mut actions_list = actions_list().lock();

        let mut actions = match actions_list.get(&lang_name) {
            Some(has) => has.clone(),
            None => {
                let file_name = format!("{}.json", lang_name.clone().to_lowercase());
                let lang_actions = from_files(
                    lang_name.clone(),
                    [
                        project_root
                            .join(".helix")
                            .join(Dirs::Actions.to_string())
                            .join(&file_name),
                        config_dir(Dirs::Actions).join(&file_name),
                    ]
                    .to_vec(),
                );

                actions_list.insert(lang_name, lang_actions.clone());
                lang_actions
            }
        };

        actions.filter(doc, range);
        actions
    }

    /// merge actions
    pub fn extend(&mut self, other: Actions) {
        self.actions.extend(other.actions);
    }

    pub fn to_code_action_items(&self, variable_init: &VariableInit) -> Vec<CodeAction> {
        self.actions
            .iter()
            .filter_map(|(_name, action)| action.to_code_action_item(variable_init))
            .collect()
    }

    pub fn filter(&mut self, doc: &Rope, range: &Range) {
        let line = doc.line(range.start.line as usize);

        let actions = self
            .actions
            .clone()
            .into_iter()
            .filter_map(|(name, action)| {
                if action.catch.is_empty() {
                    return Some((name, action));
                }

                log::trace!("{action:?}");

                let re = Regex::new(&action.catch);
                if let Ok(re) = re {
                    if re.is_match(&line.to_string()) {
                        return Some((name, action));
                    }

                    // TODO: 捕捉内容提供给脚本
                    // if let (captures) = re.captures(&line.to_string()){
                    //    if let Some(capture) = captures.get(1) {
                    //     let mut a = action.clone();
                    //     a
                    //     Some(a)
                    // };
                }
                None
            })
            .collect();

        self.actions = actions;
    }
}

fn from_files(name: String, files: Vec<PathBuf>) -> Actions {
    files
        .into_iter()
        .rev()
        .filter(|p| p.exists())
        .filter_map(|p| parse::<Actions>(&p, name.to_owned()).ok())
        .fold(
            Actions::new(name.to_owned(), HashMap::new()),
            |mut acc, map| {
                acc.extend(map);
                acc
            },
        )
}

/// 执行
pub fn shell_exec(cmd: &str) -> Result<()> {
    let shell = if cfg!(windows) {
        vec!["cmd".to_owned(), "/C".to_owned()]
    } else {
        vec!["sh".to_owned(), "-c".to_owned()]
    };

    let mut process = Command::new(&shell[0]);

    process
        .args(&shell[1..])
        .arg(cmd)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped());

    let _process = match process.spawn() {
        Ok(process) => process,
        Err(e) => {
            log::error!("Failed to start shell: {}", e);
            return Err(e.into());
        }
    };

    Ok(())
}

#[cfg(test)]
mod test {
    use super::shell_exec;

    #[test]
    fn test_shell_exec() {
        let re = shell_exec("tmux split-window -h\n tmux send hx Enter");
        eprintln!("{re:?}");
    }
}
