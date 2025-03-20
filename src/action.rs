use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::OnceLock,
    time::Duration,
};

use async_lsp::lsp_types::{self, CodeAction, CodeActionParams, Range, TextDocumentIdentifier};
use parking_lot::Mutex;
use regex::Regex;
use ropey::Rope;
use serde::{Deserialize, Serialize};
use tracing as log;
use url::Url;

use crate::{
    loader::{Dirs, config_dir},
    parser::{Parser, StrOrSeq, parse},
    variables::{VariableInit, Variables},
};

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct Action {
    /// 捕捉, 支持单行或者两行
    title: String,
    /// 返回: shell bool
    filter: StrOrSeq,
    /// shell 执行 返回 string
    shell: StrOrSeq, // string
    /// 简介
    description: Option<String>,
}

impl Action {
    /// 转换 lsp 格式
    fn to_code_action_item(
        &self,
        variable_init: &VariableInit,
        data: &ActionData,
    ) -> Option<CodeAction> {
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
            data: Some(serde_json::to_value(data.clone()).unwrap()),
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

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct ActionData {
    pub text_document: TextDocumentIdentifier,
    pub range: Range,
}

impl From<CodeActionParams> for ActionData {
    fn from(value: CodeActionParams) -> Self {
        ActionData {
            text_document: value.text_document.clone(),
            range: value.range,
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

    pub fn to_code_action_items(
        &self,
        variable_init: &VariableInit,
        data: &ActionData,
    ) -> Vec<CodeAction> {
        self.actions
            .iter()
            .filter_map(|(_name, action)| action.to_code_action_item(variable_init, data))
            .collect()
    }

    pub fn filter(&mut self, doc: &Rope, range: &Range) {
        let line = doc.line(range.start.line as usize);

        let actions = self
            .actions
            .clone()
            .into_iter()
            .filter_map(|(name, action)| {
                if action.filter.to_string().is_empty() {
                    return Some((name, action));
                }

                log::trace!("{action:?}");

                let re = Regex::new(&action.filter.to_string());
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
// pub fn shell_exec(cmd: &str) -> Result<()> {
//     let shell = if cfg!(windows) {
//         vec!["cmd".to_owned(), "/C".to_owned()]
//     } else {
//         vec!["sh".to_owned(), "-c".to_owned()]
//     };

//     let output = std::thread::spawn(move || {
//         // ...原执行逻辑

//         let mut process = Command::new(&shell[0]);

//         process
//             .args(&shell[1..])
//             .arg(cmd)
//             .stdin(Stdio::piped())
//             .stdout(Stdio::piped());

//         let _process = match process.spawn() {
//             Ok(process) => process,
//             Err(e) => {
//                 log::error!("Failed to start shell: {}", e);
//                 return Err(e.into());
//             }
//         };
//     })
//     .join_timeout(Duration::from_secs(5))?;

//     Ok(())
// }

#[cfg(test)]
mod test {
    use super::shell_impl;
    use anyhow::Result;
    use ropey::Rope;

    // #[test]
    // fn test_shell_impl() {
    //     let re = shell_impl("echo test", &Some(Rope::from_str("text")));
    //     eprintln!("{re:?}");
    // }

    #[test]
    fn test_basic_command() -> Result<()> {
        // 测试基础命令执行
        #[cfg(unix)]
        let (cmd, input, expected) = ("echo -n hello", &Some(Rope::from_str("text")), "hello");
        #[cfg(windows)]
        let (cmd, input, expected) = ("echo hello", &Some(Rope::from_str("text")), "hello");

        let output = shell_impl(cmd, input)?;
        assert_eq!(output.trim_end(), expected.trim_end());
        Ok(())
    }
}

use anyhow::{Context, Result};
use tokio::{runtime::Handle, task::block_in_place};

/// 同步接口实现（核心封装）
pub fn shell_impl(cmd: &str, input: &Option<Rope>) -> Result<String> {
    block_in_place(|| Handle::current().block_on(async { shell_impl_async(cmd, input).await }))
}

/// 异步核心实现（保持原有逻辑）
async fn shell_impl_async(cmd: &str, input: &Option<Rope>) -> Result<String> {
    let shell = get_shell();
    let mut process = tokio::process::Command::new(&shell[0]);
    process
        .args(&shell[1..])
        .arg(cmd)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());

    if input.is_some() || cfg!(windows) {
        process.stdin(std::process::Stdio::piped());
    } else {
        process.stdin(std::process::Stdio::null());
    }

    let mut process = process.spawn().context("Failed to spawn child process")?;

    if let Some(input) = input {
        // 异步写入输入
        let mut stdin = process
            .stdin
            .take()
            .ok_or_else(|| anyhow::anyhow!("Failed to open stdin"))?;

        tokio::io::AsyncWriteExt::write_all(&mut stdin, input.to_string().as_bytes())
            .await
            .context("Failed to write to stdin")?;

        drop(stdin);
    }

    let timeout_sec = 5;
    // 带超时等待
    let output =
        match tokio::time::timeout(Duration::from_secs(timeout_sec), process.wait_with_output())
            .await
        {
            Ok(Ok(output)) => output,
            Ok(Err(e)) => return Err(e).context("Child process error"),
            Err(_) => anyhow::bail!("Command timed out after {}s", timeout_sec),
        };

    // 错误状态处理
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow::anyhow!(
            "Command failed ({}): {}",
            output.status,
            stderr.trim_end()
        ));
    }

    // 输出处理
    String::from_utf8(output.stdout)
        .map(|s| s.trim_end().to_owned())
        .or_else(|e| Ok(String::from_utf8_lossy(&e.as_bytes()).into_owned()))
}

// 跨平台配置（保持与之前相同）
#[cfg(unix)]
fn get_shell() -> Vec<String> {
    vec!["sh".to_owned(), "-c".to_owned()]
}

#[cfg(windows)]
fn get_shell() -> &Vec<String> {
    vec!["cmd".to_owned(), "/C".to_owned()]
}
