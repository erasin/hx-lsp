use std::{collections::HashMap, fs::File, io::BufReader, path::PathBuf};

use json_comments::StripComments;
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::errors::Error;

pub trait Parser {
    type Item: DeserializeOwned + Clone;
    fn set_name(&mut self, name: String);
    fn set_hasmap(&mut self, hs: HashMap<String, Self::Item>);
}

/// 解析 `code-snippets json` 文件
pub fn parse<T>(lang_file_path: &PathBuf, name: String) -> Result<T, Error>
where
    T: Parser + DeserializeOwned + Serialize + Clone + Default,
{
    let file = File::open(lang_file_path)?;
    let reader = BufReader::new(file);

    // 过滤注释内容
    let json_data = StripComments::new(reader);

    // 日志记录错误
    let hs = match serde_json::from_reader(json_data) {
        Ok(s) => s,
        Err(err) => {
            log::error!("parse fail: {err:?}");
            return Err(err.into());
        }
    };

    let mut p: T = Default::default();
    p.set_name(name);
    p.set_hasmap(hs);

    Ok(p)
}

/// `String` 或者 `Vec<String>`
#[derive(Deserialize, Serialize, Clone, Debug)]
#[serde(untagged)]
pub enum StrOrSeq {
    String(String),
    Array(Vec<String>),
}

impl ToString for StrOrSeq {
    /// `Vec<String>` 使用 `\n` 组合为 String
    fn to_string(&self) -> String {
        match self {
            StrOrSeq::String(s) => s.clone(),
            StrOrSeq::Array(v) => v.join("\n").clone(),
        }
    }
}

impl StrOrSeq {
    /// 获取第一个元素
    pub fn first(&self) -> Option<String> {
        match self {
            StrOrSeq::String(s) => Some(s.clone()),
            StrOrSeq::Array(v) => v.first().and_then(|s| Some(s.clone())),
        }
    }
}
