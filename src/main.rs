// dev
// #![allow(dead_code, unused_imports)]

use std::{collections::HashMap, path::PathBuf, str::FromStr, sync::Arc};

use action::Actions;
use flexi_logger::{FileSpec, Logger, WriteMode};
use lsp_server::Connection;
use lsp_types::{
    notification::{
        DidChangeTextDocument, DidCloseTextDocument, DidOpenTextDocument, DidSaveTextDocument,
        Notification,
    },
    request::{CodeActionRequest, CodeActionResolveRequest, Completion, Request},
    CodeAction, CodeActionOptions, CodeActionProviderCapability, CompletionOptions,
    InitializeParams, SaveOptions, ServerCapabilities, TextDocumentSyncCapability,
    TextDocumentSyncKind, TextDocumentSyncOptions, TextDocumentSyncSaveOptions,
};
use parking_lot::Mutex;

mod action;
mod encoding;
mod errors;
mod loader;
mod parser;
mod snippet;
mod variables;

use errors::Error;
use ropey::Rope;
use snippet::Snippets;

use crate::encoding::{apply_content_change, OffsetEncoding};

fn main() -> Result<(), Error> {
    if let Some(arg) = std::env::args().nth(1) {
        if arg.eq("--version") {
            let version = env!("CARGO_PKG_VERSION");
            eprintln!("version: {version}");
            return Ok(());
        }

        if arg.eq("--log") {
            let _logger = Logger::try_with_str("trace, my::critical::module=trace")?
                .log_to_file(FileSpec::default())
                .write_mode(WriteMode::BufferAndFlush)
                .start()?;
        }
    }

    run_lsp_server()
}

fn run_lsp_server() -> Result<(), Error> {
    log::info!("hx-lsp server up");

    let (connection, io_threads) = Connection::stdio();

    let server_capabilities = serde_json::to_value(&ServerCapabilities {
        completion_provider: Some(CompletionOptions::default()),
        code_action_provider: Some(CodeActionProviderCapability::Options(CodeActionOptions {
            resolve_provider: Some(true),
            ..Default::default()
        })),
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
    })
    .expect("Must be serializable");

    let initialization_params = match connection.initialize(server_capabilities) {
        Ok(it) => it,
        Err(e) => {
            if e.channel_is_disconnected() {
                io_threads.join()?;
            }
            return Err(e.into());
        }
    };
    let initialization_params: InitializeParams = serde_json::from_value(initialization_params)?;

    let mut server = Server::new(&initialization_params);
    server.listen(&connection)?;
    io_threads.join()?;

    // Shut down gracefully.
    log::info!("shutting down server");

    Ok(())
}

pub struct Server {
    root: PathBuf,
    // config: Config
    lang_id: HashMap<String, String>,
    lang_doc: Arc<Mutex<HashMap<String, Rope>>>,
    // snippets
    params: InitializeParams,
}

impl Server {
    fn new(params: &InitializeParams) -> Self {
        let root = if let Some(ws) = params.workspace_folders.clone() {
            let p = ws.first().unwrap().uri.path();
            PathBuf::from_str(p).unwrap()
        } else {
            std::env::current_dir().ok().unwrap()
        };

        Server {
            root,
            lang_id: HashMap::new(),
            lang_doc: Arc::new(Mutex::new(HashMap::new())),
            params: params.clone(),
        }
    }

    fn listen(&mut self, connection: &Connection) -> Result<(), Error> {
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
            // 打开文件
            DidOpenTextDocument::METHOD => {
                let params = cast_notification::<DidOpenTextDocument>(notification)?;
                log::debug!("OpenFile: {params:?}");
                let uri = params.text_document.uri.path().to_string();

                // ropey
                let doc = Rope::from(params.text_document.text);

                // 记录打开文件所对应的文件语言ID, 内容
                self.lang_id
                    .insert(uri.clone(), params.text_document.language_id);
                self.lang_doc.lock().insert(uri.clone(), doc);

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
                let mut doc = doc_lock.get_mut(uri).expect("undefind file path.");

                // Option: change: Some(TextDocumentSyncKind::FULL),
                // *doc = Rope::from(params.content_changes.last().unwrap().text.clone());

                // 增量更新
                for change in params.content_changes {
                    // TODO: OffseetEncoding get from document
                    apply_content_change(&mut doc, &change, OffsetEncoding::Utf8)?;
                }
                Ok(())
            }

            // DidChangeWatchedFiles::METHOD => {
            //     let params = cast_notification::<DidChangeWatchedFiles>(notification)?;
            //     log::debug!("WatchFile: {params:?}");
            //     Ok(())
            // }
            DidSaveTextDocument::METHOD => {
                let params = cast_notification::<DidSaveTextDocument>(notification)?;
                log::debug!("SaveFile: {params:?}");
                let uri = params.text_document.uri.path();

                let mut doc_lock = self.lang_doc.lock();
                let doc = doc_lock.get_mut(uri).expect("undefind file path.");

                *doc = Rope::from(params.text.unwrap());

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

        let lang_id = self.lang_id.get(uri)?;

        let mut snippets = Snippets::get_lang(lang_id.clone(), &self.root);
        snippets.extend(Snippets::get_global(&self.root));

        // let snippets = [
        //     Snippets::get_lang(lang_id.clone(), &self.root),
        //     Snippets::get_global(&self.root),
        // ]
        // .into_iter()
        // .filter_map(|r| r.ok())
        // .fold(Snippets::default(), |mut lang, other| {
        //     lang.extend(other);
        //     lang
        // });

        let snippets = snippets.to_completion_items();
        Some(snippets)
    }

    fn actions(&self, params: lsp_types::CodeActionParams) -> Option<Vec<lsp_types::CodeAction>> {
        let uri = params.text_document.uri.path();
        let lang_id = self.lang_id.get(uri)?;
        let doc_lock = self.lang_doc.lock();
        let doc = doc_lock.get(uri)?;

        let actions = Actions::get_lang(lang_id.clone(), doc, &params.range, &self.root);

        Some(actions.to_code_action_items())
    }

    fn action_resolve(&self, action: &CodeAction) -> Result<CodeAction, Error> {
        let cmd = action.clone().command.expect("unknow cmd");

        action::shell_exec(cmd.command.as_str())?;

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
