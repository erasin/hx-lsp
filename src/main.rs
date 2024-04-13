// dev
#![allow(dead_code, unused_imports)]

use std::collections::HashMap;

use flexi_logger::{FileSpec, Logger, WriteMode};
use lsp_server::Connection;
use lsp_types::{
    notification::{DidCloseTextDocument, DidOpenTextDocument, Notification},
    request::{CodeActionRequest, Completion, Request},
    CodeAction, InitializeParams, ServerCapabilities,
};

mod action;
mod errors;
mod fuzzy;
mod loader;
mod snippet;
mod variables;

use errors::Error;
use snippet::Lang;

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
        completion_provider: Some(lsp_types::CompletionOptions::default()),
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
    // root: PathBuf,
    // config: Config
    lang_states: HashMap<String, String>,
    // snippets
    params: InitializeParams,
}

impl Server {
    fn new(params: &InitializeParams) -> Self {
        // get workpath
        // let project_path = params.root_uri.unwrap().clone();

        // params.initialization_options.unwrap().get("path")

        Server {
            // root: PathBuf::from(project_path.path()),
            lang_states: HashMap::new(),
            params: params.clone(),
        }
    }

    fn listen(&mut self, connection: &Connection) -> Result<(), Error> {
        log::info!("starting example main loop");

        while let Ok(msg) = connection.receiver.recv() {
            log::trace!("Message: {:#?}", msg);

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
                log::trace!("Get Completion: {params:?}");

                let completions = self.completion(params);
                log::trace!("Res Completion: {completions:?}");

                Ok(lsp_server::Response {
                    id,
                    error: None,
                    result: Some(serde_json::to_value(completions)?),
                })
            }

            // 获取可能存在的 脚本处理
            CodeActionRequest::METHOD => {
                // let params = cast_request::<CodeActionRequest>(request).expect("cast code action request");

                // TODO action for tmux open window ...
                let actions: Vec<CodeAction> = Vec::new();

                Ok(lsp_server::Response {
                    id,
                    error: None,
                    result: Some(serde_json::to_value(actions)?),
                })
            }

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

                // 记录打开文件所对应的文件语言ID
                self.lang_states.insert(
                    params.text_document.uri.path().to_owned(),
                    params.text_document.language_id,
                );

                Ok::<(), Error>(())
            }

            // 文件关闭
            DidCloseTextDocument::METHOD => {
                let params = cast_notification::<DidCloseTextDocument>(notification)?;

                // 移除记录的文件状态
                self.lang_states.remove(params.text_document.uri.path());

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
        let lang_name = self
            .lang_states
            .get(params.text_document_position.text_document.uri.path())?;

        // let mut lang = Lang::get_lang(lang_name.to_owned()).ok()?;
        // if let Some(global) = Lang::get_global().ok() {
        //     lang.extend(global);
        // }

        let lang = [Lang::get_lang(lang_name.to_owned()), Lang::get_global()]
            .into_iter()
            .filter(|r| r.is_ok())
            .map(move |r| r.unwrap())
            .fold(Lang::default(), |mut lang, other| {
                lang.extend(other);
                lang
            });

        let items = lang.get_completion_items();
        Some(items)
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
