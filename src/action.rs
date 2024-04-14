use std::{
    collections::HashMap,
    path::PathBuf,
    process::{Command, Stdio},
};

use lsp_types::{CodeAction, WorkspaceEdit};
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
    catch: StrOrSeq,
    shell: StrOrSeq, // string
    description: Option<String>,
}

impl Action {
    /// 转换 lsp 格式
    fn to_code_action_item(&self) -> Option<CodeAction> {
        if self.catch.first().is_none() {
            return None;
        }

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
        project_root: &PathBuf,
    ) -> Result<Actions, Error> {
        let file_name = format!("{}.json", lang_name.clone().to_lowercase());
        let actions_file_path = config_dir(Dirs::Actions).join(file_name);
        let actions = parse::<Actions>(&actions_file_path, lang_name)?;

        // TODO: project

        Ok(actions)
    }

    pub fn to_code_action_items(&self) -> Vec<CodeAction> {
        self.actions
            .iter()
            .map(|(_name, action)| action.to_code_action_item())
            .filter(|s| s.is_some())
            .map(|s| s.unwrap())
            .collect()
    }
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
