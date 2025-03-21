use std::{
    collections::HashMap,
    io::Write,
    path::PathBuf,
    process::{Command, Stdio},
    sync::{OnceLock, mpsc},
    thread,
    time::{Duration, Instant},
};

use anyhow::{Context, Result};
use async_lsp::lsp_types::{
    CodeAction, CodeActionKind, CodeActionParams, Range, TextDocumentIdentifier,
};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

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

        let action = CodeAction {
            title: self.title.clone(),
            kind: Some(CodeActionKind::EMPTY),
            is_preferred: Some(true),
            diagnostics: None,
            disabled: None,
            data: Some(serde_json::to_value(data.with_command(shell).clone()).unwrap()),
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
    pub command: Option<String>,
}

impl ActionData {
    pub fn with_command(&self, command: String) -> Self {
        ActionData {
            command: Some(command),
            ..self.clone()
        }
    }
}

impl From<CodeActionParams> for ActionData {
    fn from(value: CodeActionParams) -> Self {
        ActionData {
            text_document: value.text_document.clone(),
            range: value.range,
            command: None,
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

    pub fn get_lang(lang_name: String, init: &VariableInit) -> Actions {
        let mut actions_list = actions_list().lock();

        let mut actions = match actions_list.get(&lang_name) {
            Some(has) => has.clone(),
            None => {
                let file_name = format!("{}.json", lang_name.clone().to_lowercase());
                let lang_actions = from_files(
                    lang_name.clone(),
                    [
                        init.work_path
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

        actions.filter(init);
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

    fn filter(&mut self, init: &VariableInit) {
        let actions = self
            .actions
            .clone()
            .into_iter()
            .filter_map(|(name, action)| {
                if action.filter.to_string().is_empty() {
                    return Some((name, action));
                }

                let shell_script = action.filter.to_string();
                let shell_script = Variables::convert_all(&shell_script, init);

                let filter = match shell(&shell_script, &Some(init.selected_text.clone())) {
                    Ok(s) => matches!(s.to_lowercase().as_str(), "true" | "1"),
                    Err(_) => false,
                };
                match filter {
                    true => Some((name, action)),
                    false => None,
                }
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

/// 异步核心实现（保持原有逻辑）
pub fn shell(cmd: &str, input: &Option<String>) -> Result<String> {
    let shell = get_shell();
    let mut process = Command::new(&shell[0]);
    process
        .args(&shell[1..])
        .arg(cmd)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    if input.is_some() || cfg!(windows) {
        process.stdin(Stdio::piped());
    } else {
        process.stdin(Stdio::null());
    }

    let mut process = process.spawn().context("Failed to spawn child process")?;

    if let Some(input) = input {
        // 异步写入输入
        let mut stdin = process
            .stdin
            .take()
            .ok_or_else(|| anyhow::anyhow!("Failed to open stdin"))?;

        stdin
            .write_all(input.to_string().as_bytes())
            .context("Failed to write to stdin")?;

        drop(stdin);
    }

    let timeout = Duration::from_secs(5);

    // 使用通道进行超时控制
    let (tx, rx) = mpsc::channel();
    let start_time = Instant::now();

    // 启动监控线程
    thread::spawn(move || {
        let output = process.wait_with_output();
        let _ = tx.send(output);
    });

    // 带超时等待
    let output = match rx.recv_timeout(timeout) {
        Ok(Ok(output)) => output,
        Ok(Err(e)) => return Err(e).context("Child process error"),
        Err(_) => {
            let elapsed = start_time.elapsed().as_secs();
            anyhow::bail!(
                "Command timed out after {}s (max {}s)",
                elapsed,
                timeout.as_secs()
            )
        }
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
        .or_else(|e| Ok(String::from_utf8_lossy(e.as_bytes()).into_owned()))
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

#[cfg(test)]
mod test {
    use super::shell;
    use anyhow::Result;

    #[test]
    fn test_basic_command() -> Result<()> {
        // 测试基础命令执行
        #[cfg(unix)]
        let (cmd, input, expected) = ("echo -n hello", &Some(String::from("text")), "hello");
        #[cfg(windows)]
        let (cmd, input, expected) = ("echo hello", &Some(Rope::from_str("text")), "hello");

        let output = shell(cmd, input)?;
        assert_eq!(output.trim_end(), expected.trim_end());
        Ok(())
    }
}
