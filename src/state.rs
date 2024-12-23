use async_lsp::lsp_types::{Position, TextDocumentContentChangeEvent, Url};
use ropey::Rope;
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, RwLock},
};
use tracing::info;

#[derive(Default, Clone)]
pub struct State {
    pub root: PathBuf,
    pub documents: Arc<RwLock<HashMap<Url, Rope>>>,
    pub language_ids: Arc<RwLock<HashMap<Url, String>>>,
    pub client_info: ClientInfo,
}

#[derive(Default, Clone)]
pub struct ClientInfo {
    pub name: String,
    pub version: String,
}

impl State {
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

    pub fn upsert_file(&mut self, uri: &Url, content: Rope, language_id: Option<String>) {
        info!("upserting file: {}", uri);
        if let Some(language_id) = language_id {
            self.language_ids
                .write()
                .expect("Set Content Fail")
                .insert(uri.clone(), language_id);
        };

        let mut docs = self.documents.write().expect("Get Document Fail");
        docs.insert(uri.clone(), content);
    }

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
    }

    pub fn remove_file(&self, uri: &Url) {
        self.documents
            .write()
            .expect("Get Document Fail")
            .remove(&uri.clone());
        self.language_ids
            .write()
            .expect("Get Language Id Fail")
            .remove(&uri.clone());
    }

    pub fn update_client_info(&mut self, name: String, version: String) {
        self.client_info = ClientInfo { name, version };
    }
}

// convert lsp position to Rope position
pub(crate) fn position_to_char_index(rope: &Rope, position: Position) -> usize {
    rope.line_to_char(position.line as usize) + (position.character as usize)
}
