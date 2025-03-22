use async_lsp::lsp_types::{ColorInformation, Position, TextDocumentContentChangeEvent, Url};
use ropey::Rope;
use std::{
    collections::HashMap,
    hash::{DefaultHasher, Hash, Hasher},
    path::PathBuf,
    sync::{Arc, RwLock},
};
use tracing::debug;

#[derive(Default, Clone)]
pub struct State {
    pub root: PathBuf,
    pub documents: Arc<RwLock<HashMap<Url, Rope>>>,
    pub language_ids: Arc<RwLock<HashMap<Url, String>>>,
    pub color_cache: Arc<RwLock<HashMap<Url, CachedColors>>>, // 新增颜色缓存
    pub client_info: ClientInfo,
}

#[derive(Default, Clone)]
pub struct ClientInfo {
    pub name: String,
    pub version: String,
}

// 新增缓存结构
#[derive(Debug, Clone)]
pub struct CachedColors {
    content_hash: u64,
    colors: Vec<ColorInformation>,
}

impl State {
    // 计算文档内容的哈希值
    pub fn calculate_content_hash(&self, uri: &Url) -> u64 {
        let documents = self.documents.read().expect("Failed to read documents");
        let mut hasher = DefaultHasher::new();

        if let Some(content) = documents.get(uri) {
            content.chunks().for_each(|chunk| chunk.hash(&mut hasher));
        }

        hasher.finish()
    }

    pub fn get_contents(&self, uri: &Url) -> Rope {
        self.documents
            .read()
            .expect("Get Content Fail")
            .get(uri)
            .map(|s| s.to_owned())
            .unwrap_or_default()
    }

    pub fn get_language_id(&self, uri: &Url) -> String {
        self.language_ids
            .read()
            .expect("Get Language Id Fail")
            .get(uri)
            .map(|s| s.to_owned())
            .unwrap_or_default()
    }

    /// 打开文件时候保存处理
    pub fn open_file(&mut self, uri: &Url, content: Rope, language_id: Option<String>) {
        debug!("upsert file: {}", uri);

        if let Some(language_id) = language_id {
            self.language_ids
                .write()
                .expect("Set Content Fail")
                .insert(uri.clone(), language_id);
        };

        {
            let mut docs = self.documents.write().expect("Failed to write documents");
            docs.insert(uri.clone(), content);
        }

        // 清理色彩
        self.clear_color_cache(uri);
    }

    /// 更新文件
    pub fn apply_content_change(&mut self, uri: &Url, content: Rope) {
        let changed = {
            let mut docs = self.documents.write().expect("Failed to write documents");
            if let Some(doc) = docs.get_mut(uri) {
                *doc = content;
                true
            } else {
                false
            }
        };
        if changed {
            // 内容变更时清除缓存
            self.clear_color_cache(uri);
        }
    }

    /// 变更内容
    pub fn change_file(&mut self, uri: &Url, contents: Vec<TextDocumentContentChangeEvent>) {
        if let Some(doc) = self
            .documents
            .write()
            .expect("Get Document Fail")
            .get_mut(&uri.clone())
        {
            for content in contents {
                if let Some(range) = content.range {
                    let start = position_to_char_index(doc, range.start);
                    let end = position_to_char_index(doc, range.end);

                    doc.remove(start..end);
                    doc.insert(start, &content.text);
                } else {
                    *doc = Rope::from_str(&content.text);
                }
            }
        }

        self.clear_color_cache(uri);
    }

    /// 清理关闭的文件
    pub fn clean_file(&self, uri: &Url) {
        self.documents
            .write()
            .expect("Failed to write documents")
            .remove(uri);
        self.language_ids
            .write()
            .expect("Failed to write language IDs")
            .remove(uri);
        self.color_cache
            .write()
            .expect("Failed to write color cache")
            .remove(uri); // 移除文件时清除缓存
    }

    /// 客户端信息
    pub fn update_client_info(&mut self, name: String, version: String) {
        self.client_info = ClientInfo { name, version };
    }

    /// 获取或更新颜色缓存
    pub fn get_cached_colors(&self, uri: &Url, content_hash: u64) -> Option<Vec<ColorInformation>> {
        let cache = self.color_cache.read().expect("Failed to read color cache");
        cache.get(uri).and_then(|cached| {
            if cached.content_hash == content_hash {
                Some(cached.colors.clone())
            } else {
                None
            }
        })
    }

    // 更新颜色缓存
    pub fn update_color_cache(
        &mut self,
        uri: &Url,
        content_hash: u64,
        colors: Vec<ColorInformation>,
    ) {
        let mut cache = self
            .color_cache
            .write()
            .expect("Failed to write color cache");
        cache.insert(
            uri.clone(),
            CachedColors {
                content_hash,
                colors,
            },
        );
    }

    // 清理颜色缓存
    pub fn clear_color_cache(&mut self, uri: &Url) {
        let mut cache = self
            .color_cache
            .write()
            .expect("Failed to write color cache");
        cache.remove(uri);
    }
}

// convert lsp position to Rope position
pub(crate) fn position_to_char_index(rope: &Rope, position: Position) -> usize {
    rope.line_to_char(position.line as usize) + (position.character as usize)
}
