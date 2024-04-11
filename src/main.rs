use std::{collections::HashMap, path::PathBuf};

use log::{info, trace};
use lsp_server::Connection;
use lsp_types::{
    notification::{
        DidChangeTextDocument, DidChangeWatchedFiles, DidCloseTextDocument, DidOpenTextDocument,
        DidSaveTextDocument, Notification, Progress, PublishDiagnostics, ShowMessage,
    },
    request::{
        CodeActionRequest, Completion, Formatting, GotoDefinition, HoverRequest, Request,
        WorkDoneProgressCreate,
    },
    CompletionItem, CompletionItemKind, DocumentFormattingParams, InitializeParams,
    ServerCapabilities, TextEdit,
};

mod errors;
mod fuzzy;

use errors::Error;

use flexi_logger::{FileSpec, Logger, WriteMode};

fn main() -> Result<(), Error> {
    let _logger = Logger::try_with_str("trace, my::critical::module=trace")?
        .log_to_file(FileSpec::default())
        .write_mode(WriteMode::BufferAndFlush)
        .start()?;

    run_lsp_server()
}

fn run_lsp_server() -> Result<(), Error> {
    info!("hx-lsp server up");

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

    // main_loop(connection, initialization_params)?;

    let mut server = Server::new(initialization_params);
    server.listen(connection)?;
    io_threads.join()?;

    // Shut down gracefully.
    info!("shutting down server");

    Ok(())
}

pub struct Server {
    root: PathBuf,
    // config: Config
    lang_states: HashMap<String, String>,
    // snippets
    params: InitializeParams,
}

impl Server {
    fn new(params: InitializeParams) -> Self {
        Server {
            root: PathBuf::new(),
            lang_states: HashMap::new(),
            params,
        }
    }

    fn listen(&mut self, connection: Connection) -> Result<(), Error> {
        info!("starting example main loop");

        while let Ok(msg) = connection.receiver.recv() {
            trace!("Message: {:#?}", msg);

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
                    info!("Get Response: {resp:?}");
                }
                lsp_server::Message::Notification(notification) => {
                    self.handle_notification(notification)?
                }
            }
        }

        Ok(())
    }

    fn handle_request(
        &mut self,
        request: lsp_server::Request,
    ) -> Result<lsp_server::Response, Error> {
        let id = request.id.clone();

        match request.method.as_str() {
            Completion::METHOD => {
                let params = cast_request::<Completion>(request).expect("cast Completion");

                trace!("Get Completion: {params:?}");

                let completions = self.completion(params);

                trace!("Res Completion: {completions:?}");

                Ok(lsp_server::Response {
                    id,
                    error: None,
                    result: Some(serde_json::to_value(completions)?),
                })
            }

            // CodeActionRequest::METHOD => {
            //     let params =
            //         cast_request::<CodeActionRequest>(request).expect("cast code action request");

            //     // TODO action for tmux open window ...
            //     let mut actions = Vec::new();

            //     Ok(lsp_server::Response {
            //         id,
            //         error: None,
            //         result: Some(serde_json::to_value(action)?),
            //     })
            // }
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

    fn handle_notification(&mut self, notification: lsp_server::Notification) -> Result<(), Error> {
        match notification.method.as_str() {
            DidOpenTextDocument::METHOD => {
                let params = cast_notification::<DidOpenTextDocument>(notification)?;

                self.lang_states.insert(
                    params.text_document.uri.path().to_owned(),
                    params.text_document.language_id,
                );

                Ok::<(), Error>(())
            }

            DidCloseTextDocument::METHOD => {
                let params = cast_notification::<DidCloseTextDocument>(notification)?;

                self.lang_states.remove(params.text_document.uri.path());

                Ok(())
            }

            unsupported => Err(Error::UnsupportedLspRequest {
                request: unsupported.to_string(),
            }),
        }
    }

    fn completion(
        &self,
        params: lsp_types::CompletionParams,
    ) -> Option<Vec<lsp_types::CompletionItem>> {
        // let completion_items = if let Some(lang) =
        // state.get(&params.text_document_position.text_document.uri)
        let mut items = Vec::new();

        items.push(CompletionItem {
            label: "text1".to_owned(),
            kind: Some(CompletionItemKind::SNIPPET),
            detail: Some("text1 detail".to_owned()),
            documentation: Some(lsp_types::Documentation::String(
                "document text1".to_owned(),
            )),
            insert_text: Some("test1 $1 - $2".to_owned()),
            ..Default::default()
        });

        Some(items)
    }
}

fn cast_request<R>(request: lsp_server::Request) -> Result<R::Params, Error>
where
    R: lsp_types::request::Request,
    R::Params: serde::de::DeserializeOwned,
{
    let (_, params) = request.extract(R::METHOD)?;
    Ok(params)
}

fn cast_notification<N>(notification: lsp_server::Notification) -> Result<N::Params, Error>
where
    N: lsp_types::notification::Notification,
    N::Params: serde::de::DeserializeOwned,
{
    let params = notification.extract::<N::Params>(N::METHOD)?;
    Ok(params)
}
