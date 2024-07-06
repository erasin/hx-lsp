use std::{collections::HashMap, path::PathBuf, sync::Arc};

use lsp_server::Connection;
use lsp_types::{
    notification::{
        DidChangeTextDocument, DidCloseTextDocument, DidOpenTextDocument, DidSaveTextDocument,
        Exit, Notification,
    },
    request::{CodeActionRequest, CodeActionResolveRequest, Completion, Request},
    CodeAction, CodeActionOptions, CodeActionProviderCapability, CompletionOptions,
    InitializeParams, SaveOptions, ServerCapabilities, TextDocumentSyncCapability,
    TextDocumentSyncKind, TextDocumentSyncOptions, TextDocumentSyncSaveOptions,
};
use parking_lot::Mutex;
use ropey::Rope;

use crate::{
    action::{shell_exec, Actions},
    encoding::{get_current_word, is_field},
};
use crate::{clipboard::get_clipboard_provider, snippet::Snippets};
use crate::{encoding::get_range_content, errors::Error};
use crate::{
    encoding::{apply_content_change, OffsetEncoding},
    variables::VariableInit,
};

pub fn server_capabilities() -> ServerCapabilities {
    ServerCapabilities {
        completion_provider: Some(CompletionOptions::default()),
        code_action_provider: Some(CodeActionProviderCapability::Options(CodeActionOptions {
            resolve_provider: Some(true),
            ..Default::default()
        })),
        // definition_provider: Some(lsp_types::OneOf::Left(true)),
        text_document_sync: Some(TextDocumentSyncCapability::Options(
            TextDocumentSyncOptions {
                open_close: Some(true),
                // change: Some(TextDocumentSyncKind::FULL),
                change: Some(TextDocumentSyncKind::INCREMENTAL),
                will_save: Some(true),
                will_save_wait_until: Some(true),
                save: Some(TextDocumentSyncSaveOptions::SaveOptions(SaveOptions {
                    include_text: Some(true),
                })),
            },
        )),
        // workspace: Some(WorkspaceServerCapabilities { }),
        ..Default::default()
    }
}

pub struct Server {
    root: PathBuf,
    // config: Config
    lang_id: HashMap<String, String>,
    lang_doc: Arc<Mutex<HashMap<String, Rope>>>,
    // snippets
    #[allow(dead_code)]
    params: InitializeParams,
}

impl Server {
    pub fn new(params: &InitializeParams) -> Self {
        // FIX: 文件夹中存在多个 .helix 的目录问题
        let mut root = std::env::current_dir().ok().unwrap();
        if let Some(ws) = params.workspace_folders.clone() {
            if !ws.is_empty() {
                root = ws.first().unwrap().uri.to_file_path().unwrap();
            }
        };

        Server {
            root,
            lang_id: HashMap::new(),
            lang_doc: Arc::new(Mutex::new(HashMap::new())),
            params: params.clone(),
        }
    }

    pub fn listen(&mut self, connection: &Connection) -> Result<(), Error> {
        log::info!("starting example main loop");

        while let Ok(msg) = connection.receiver.recv() {
            match msg {
                lsp_server::Message::Request(request) => {
                    if connection.handle_shutdown(&request)? {
                        return Ok(());
                    }

                    let response = self.handle_request(request)?;

                    connection
                        .sender
                        .send(lsp_server::Message::Response(response))?;
                }
                lsp_server::Message::Response(resp) => {
                    log::info!("Get Response: {resp:?}");
                }
                lsp_server::Message::Notification(notification) => {
                    self.handle_notification(notification)?
                }
            }
        }

        Ok(())
    }

    /// lsp 请求处理
    fn handle_request(
        &mut self,
        request: lsp_server::Request,
    ) -> Result<lsp_server::Response, Error> {
        let id = request.id.clone();

        match request.method.as_str() {
            // 代码片段补全处理
            Completion::METHOD => {
                let params = cast_request::<Completion>(request).expect("cast Completion");
                let completions = self.completion(params);

                Ok(lsp_server::Response {
                    id,
                    error: None,
                    result: Some(serde_json::to_value(completions)?),
                })
            }

            // 获取可能存在的 脚本处理
            CodeActionRequest::METHOD => {
                let params =
                    cast_request::<CodeActionRequest>(request).expect("cast code action request");
                let actions = self.actions(params);

                Ok(lsp_server::Response {
                    id,
                    error: None,
                    result: Some(serde_json::to_value(actions)?),
                })
            }

            CodeActionResolveRequest::METHOD => {
                let params = cast_request::<CodeActionResolveRequest>(request)
                    .expect("cast code action request");
                let actions = self.action_resolve(&params)?;

                Ok(lsp_server::Response {
                    id,
                    error: None,
                    result: Some(serde_json::to_value(actions)?),
                })
            }

            // DocumentDiagnosticRequest::METHOD
            // WorkspaceDiagnosticRequest::METHOD
            unsupported => Err(Error::UnsupportedLspRequest {
                request: unsupported.to_string(),
            }),
        }

        // match cast_request::<Completion>(request) {
        //     Ok((id, params)) => Vec::new(),

        //     Err(err @ ExtractError::JsonError { .. }) => panic!("{err:?}"),
        //     Err(ExtractError::MethodMismatch(req)) => req,
        // };
    }

    /// lsp 提示处理
    fn handle_notification(&mut self, notification: lsp_server::Notification) -> Result<(), Error> {
        match notification.method.as_str() {
            Exit::METHOD => {
                // exit
                Ok(())
            }

            // 打开文件
            DidOpenTextDocument::METHOD => {
                let params = cast_notification::<DidOpenTextDocument>(notification)?;
                log::debug!("OpenFile: {params:?}");
                let uri = params.text_document.uri.path();

                // ropey
                let doc = Rope::from(params.text_document.text);

                // 记录打开文件所对应的文件语言ID, 内容
                self.lang_id
                    .insert(uri.to_string(), params.text_document.language_id);
                self.lang_doc.lock().insert(uri.to_string(), doc);

                Ok::<(), Error>(())
            }

            // 文件关闭
            DidCloseTextDocument::METHOD => {
                let params = cast_notification::<DidCloseTextDocument>(notification)?;
                log::debug!("CloseFile: {params:?}");
                let uri = params.text_document.uri.path();

                // 移除记录的文件状态
                self.lang_id.remove(uri);
                self.lang_doc.lock().remove(uri);

                Ok(())
            }

            // didchange
            DidChangeTextDocument::METHOD => {
                let params = cast_notification::<DidChangeTextDocument>(notification)?;
                log::debug!("ChangeText: {params:?}");
                let uri = params.text_document.uri.path();

                let mut doc_lock = self.lang_doc.lock();
                if let Some(doc) = doc_lock.get_mut(uri) {
                    // 增量更新
                    for change in params.content_changes {
                        // OffseetEncoding get from document
                        apply_content_change(doc, &change, OffsetEncoding::Utf8)?;
                    }
                };

                // Option: change: Some(TextDocumentSyncKind::FULL),
                // *doc = Rope::from(params.content_changes.last().unwrap().text.clone());

                Ok(())
            }

            DidSaveTextDocument::METHOD => {
                let params = cast_notification::<DidSaveTextDocument>(notification)?;
                log::debug!("SaveFile: {params:?}");
                let uri = params.text_document.uri.path();
                let save = Rope::from(params.text.unwrap());
                let mut doc_lock = self.lang_doc.lock();
                if let Some(doc) = doc_lock.get_mut(uri) {
                    *doc = save;
                } else {
                    doc_lock.insert(uri.to_string(), save);
                }
                Ok(())
            }

            unsupported => Err(Error::UnsupportedLspRequest {
                request: unsupported.to_string(),
            }),
        }
    }

    /// 获取补全列表
    fn completion(
        &self,
        params: lsp_types::CompletionParams,
    ) -> Option<Vec<lsp_types::CompletionItem>> {
        let uri = params.text_document_position.text_document.uri.path();
        let pos = params.text_document_position.position;
        let lang_id = self.lang_id.get(uri)?;

        let snippets = [
            Snippets::get_lang(lang_id.clone(), &self.root),
            Snippets::get_global(&self.root),
        ]
        .into_iter()
        .fold(Snippets::default(), |mut lang, other| {
            lang.extend(other);
            lang
        });

        let doc_lock = self.lang_doc.lock();
        let doc = doc_lock
            .get(uri)
            .unwrap_or_else(|| panic!("unknown cachefile of {uri}"));
        let line = doc.get_line(pos.line as usize)?;

        if is_field(&line, pos.character as usize) {
            return None;
        };

        let mut cursor_word = String::new();

        let snippets = match get_current_word(&line, pos.character as usize) {
            Some(word) => {
                cursor_word = word.to_string();
                snippets.filter(word).ok()?
            }
            None => snippets,
        };

        let variable_init = VariableInit {
            file_path: params
                .text_document_position
                .text_document
                .uri
                .to_file_path()
                .unwrap(),
            work_path: self.root.clone(),
            line_pos: params.text_document_position.position.line as usize,
            line_text: line.to_string(),
            current_word: cursor_word,
            selected_text: Default::default(),
            clipboard: get_clipboard_provider().get_contents().ok(),
        };

        Some(snippets.to_completion_items(&variable_init))
    }

    fn actions(&self, params: lsp_types::CodeActionParams) -> Option<Vec<lsp_types::CodeAction>> {
        let uri = params.text_document.uri.path();
        let lang_id = self.lang_id.get(uri)?;
        let doc_lock = self.lang_doc.lock();
        let doc = doc_lock.get(uri)?;

        let line = doc.get_line(params.range.end.line as usize)?;
        let cursor_word =
            get_current_word(&line, params.range.end.character as usize).unwrap_or_default();

        let actions = Actions::get_lang(lang_id.clone(), doc, &params.range, &self.root);

        let range_content =
            get_range_content(doc, &params.range, OffsetEncoding::Utf8).unwrap_or("".into());

        let variable_init = VariableInit {
            file_path: params.text_document.uri.to_file_path().unwrap(),
            work_path: self.root.clone(),
            line_pos: params.range.start.line as usize,
            line_text: line.to_string(),
            current_word: cursor_word.to_string(),
            selected_text: range_content.to_string(),
            clipboard: get_clipboard_provider().get_contents().ok(),
        };

        Some(actions.to_code_action_items(&variable_init))
    }

    fn action_resolve(&self, action: &CodeAction) -> Result<CodeAction, Error> {
        let cmd = action.clone().command.expect("unknow cmd");

        shell_exec(cmd.command.as_str())?;

        // 设置 title 和 tooltip
        let mut resolved_action = action.clone();
        resolved_action.kind = Some(lsp_types::CodeActionKind::EMPTY);
        resolved_action.command = None;

        Ok(resolved_action)
    }
}

/// 获取 request 参数
fn cast_request<R>(request: lsp_server::Request) -> Result<R::Params, Error>
where
    R: lsp_types::request::Request,
    R::Params: serde::de::DeserializeOwned,
{
    let (_, params) = request.extract(R::METHOD)?;
    Ok(params)
}

/// 获取 notification 参数
fn cast_notification<N>(notification: lsp_server::Notification) -> Result<N::Params, Error>
where
    N: lsp_types::notification::Notification,
    N::Params: serde::de::DeserializeOwned,
{
    let params = notification.extract::<N::Params>(N::METHOD)?;
    Ok(params)
}
