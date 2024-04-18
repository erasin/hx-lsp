use std::{
    collections::HashMap,
    path::PathBuf,
    process::{Command, Stdio},
};

use lsp_types::{CodeAction, Range};
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use regex::Regex;
use ropey::Rope;
use serde::{Deserialize, Serialize};

use crate::{
    errors::Error,
    loader::{config_dir, Dirs},
    parser::{parse, Parser, StrOrSeq},
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
    fn to_code_action_item(&self) -> Option<CodeAction> {
        // if self.catch.is_empty() {
        //     return None;
        // }

        let command = lsp_types::Command {
            title: "Run Test".to_string(),
            command: self.shell.to_string(),
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
    fn description(&self) -> String {
        match &self.description {
            Some(s) => s.clone(),
            None => String::new(),
        }
    }
}

static ACTIONS: Lazy<Mutex<HashMap<String, Actions>>> = Lazy::new(|| Mutex::new(HashMap::new()));

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

    /// .
    ///
    /// # Errors
    ///
    /// This function will return an error if .
    pub fn get_lang(
        lang_name: String,
        file_content: &Rope,
        file_range: &Range,
        project_root: &PathBuf,
    ) -> Actions {
        let mut actions_list = ACTIONS.lock();

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

        actions.fliter(file_content, file_range);
        actions
    }

    /// 合并 actions
    pub fn extend(&mut self, other: Actions) {
        self.actions.extend(other.actions);
    }

    pub fn to_code_action_items(&self) -> Vec<CodeAction> {
        self.actions
            .iter()
            .filter_map(|(_name, action)| action.to_code_action_item())
            .collect()
    }

    pub fn fliter(&mut self, file_content: &Rope, file_range: &Range) {
        let line = file_content.line(file_range.start.line as usize);

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
                if re.is_ok() {
                    let re = re.unwrap();
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
pub fn shell_exec(cmd: &str) -> Result<(), Error> {
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
