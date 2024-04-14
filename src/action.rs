use std::{
    collections::HashMap,
    process::{Command, Stdio},
};

use lsp_types::CodeAction;
use serde::{Deserialize, Serialize};

use crate::{
    errors::Error,
    loader::config_dir,
    parser::{Parser, StrOrSeq},
};

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct Action {
    /// 捕捉
    catch: String,
    prefix: StrOrSeq, // string
    body: StrOrSeq,   // string
    description: Option<String>,
}

impl Action {
    /// 转换 lsp 格式
    fn to_code_action_item(&self) -> Option<CodeAction> {
        todo!()
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

    pub fn get_lang(lang_name: String) -> Result<Actions, Error> {
        // let file_name = format!("{}.json", lang_name.clone().to_lowercase());
        // let lang_file_path = config_dir(Dirs::Snippets).join(file_name);
        // let lang = parse(&lang_file_path, lang_name)?;

        // // TODO: project

        // Ok(lang)
        todo!()
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
        let re = shell_exec("tmux split-window -h; tmux send hx Enter");
        eprintln!("{re:?}");
    }
}
