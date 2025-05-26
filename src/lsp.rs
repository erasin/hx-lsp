use std::{collections::HashMap, ops::ControlFlow, time::Duration};

use async_lsp::{
    ClientSocket, LanguageServer, ResponseError,
    client_monitor::ClientProcessMonitorLayer,
    concurrency::ConcurrencyLayer,
    lsp_types::{
        CodeAction, CodeActionKind, CodeActionOptions, CodeActionParams,
        CodeActionProviderCapability, CodeActionResponse, ColorInformation,
        ColorProviderCapability, CompletionOptions, CompletionParams, CompletionResponse,
        DidChangeConfigurationParams, DidChangeTextDocumentParams, DidCloseTextDocumentParams,
        DidOpenTextDocumentParams, DidSaveTextDocumentParams, DocumentColorParams,
        InitializeParams, InitializeResult, PositionEncodingKind, SaveOptions, ServerCapabilities,
        TextDocumentSyncCapability, TextDocumentSyncKind, TextDocumentSyncOptions,
        TextDocumentSyncSaveOptions, TextEdit, WorkspaceEdit,
    },
    panic::CatchUnwindLayer,
    router::Router,
    server::LifecycleLayer,
    tracing::TracingLayer,
};
use copypasta::{ClipboardContext, ClipboardProvider};
use futures::future::BoxFuture;
use ropey::Rope;
use tower::ServiceBuilder;
use tracing::{Level, info};

use crate::{
    action::{Actions, shell},
    action_inner::{case_actions, markdown_actions},
    colors::extract_colors,
    encoding::{get_current_word, get_range_content, is_field},
    snippet::Snippets,
    state::State,
    variables::VariableInit,
};

/// LSP 服务器
pub struct Server {
    #[allow(unused)]
    pub client: ClientSocket,
    pub state: State,
}

pub struct TickEvent;

impl Server {
    pub fn router(client: ClientSocket) -> Router<Self> {
        let mut router = Router::from_language_server(Self {
            client,
            state: State::default(),
        });
        router.event(Self::on_tick);
        router
    }

    fn on_tick(&mut self, _: TickEvent) -> ControlFlow<async_lsp::Result<()>> {
        ControlFlow::Continue(())
    }

    pub async fn run() {
        let (server, _) = async_lsp::MainLoop::new_server(|client| -> _ {
            tokio::spawn({
                let client = client.clone();
                async move {
                    let mut interval = tokio::time::interval(Duration::from_secs(1));
                    loop {
                        interval.tick().await;
                        if client.emit(TickEvent).is_err() {
                            break;
                        }
                    }
                }
            });

            ServiceBuilder::new()
                .layer(TracingLayer::default())
                .layer(LifecycleLayer::default())
                .layer(CatchUnwindLayer::default())
                .layer(ConcurrencyLayer::default())
                .layer(ClientProcessMonitorLayer::new(client.clone()))
                .service(Server::router(client))
        });

        tracing_subscriber::fmt()
            .with_max_level(Level::INFO)
            .without_time()
            .with_ansi(false)
            .with_writer(std::io::stderr)
            .init();

        // Prefer truly asynchronous piped stdin/stdout without blocking tasks.
        #[cfg(unix)]
        let (stdin, stdout) = (
            async_lsp::stdio::PipeStdin::lock_tokio().unwrap(),
            async_lsp::stdio::PipeStdout::lock_tokio().unwrap(),
        );
        // Fallback to spawn blocking read/write otherwise.
        #[cfg(not(unix))]
        let (stdin, stdout) = (
            tokio_util::compat::TokioAsyncReadCompatExt::compat(tokio::io::stdin()),
            tokio_util::compat::TokioAsyncWriteCompatExt::compat_write(tokio::io::stdout()),
        );

        server.run_buffered(stdin, stdout).await.unwrap();
    }
}

impl LanguageServer for Server {
    type Error = ResponseError;
    type NotifyResult = ControlFlow<async_lsp::Result<()>>;

    fn initialize(
        &mut self,
        params: InitializeParams,
    ) -> BoxFuture<'static, Result<InitializeResult, Self::Error>> {
        //  文件夹中存在多个 .helix 的目录问题
        if let Some(ws) = params.workspace_folders.clone() {
            if !ws.is_empty() {
                self.state.root = ws.first().unwrap().uri.to_file_path().unwrap();
            }
        };

        let unknown = "unknown".to_owned();
        if let Some(client_info) = params.client_info {
            let client_version = client_info.version.unwrap_or(unknown);
            self.state
                .update_client_info(client_info.name, client_version);
        } else {
            self.state.update_client_info("web".to_owned(), unknown);
        };
        Box::pin(async move {
            Ok(InitializeResult {
                capabilities: ServerCapabilities {
                    position_encoding: Some(PositionEncodingKind::UTF16),
                    code_action_provider: Some(CodeActionProviderCapability::Options(
                        CodeActionOptions {
                            code_action_kinds: Some(vec![
                                CodeActionKind::EMPTY,
                                CodeActionKind::REFACTOR_REWRITE,
                            ]),
                            resolve_provider: Some(true),
                            ..Default::default()
                        },
                    )),
                    completion_provider: Some(CompletionOptions {
                        resolve_provider: Some(false),
                        ..Default::default()
                    }),
                    color_provider: Some(ColorProviderCapability::Simple(true)),
                    text_document_sync: Some(TextDocumentSyncCapability::Options(
                        TextDocumentSyncOptions {
                            open_close: Some(true),
                            change: Some(TextDocumentSyncKind::INCREMENTAL),
                            will_save: Some(true),
                            will_save_wait_until: Some(true),
                            save: Some(TextDocumentSyncSaveOptions::SaveOptions(SaveOptions {
                                include_text: Some(true),
                            })),
                        },
                    )),
                    ..Default::default()
                },
                server_info: None,
            })
        })
    }

    fn did_change_configuration(
        &mut self,
        _: DidChangeConfigurationParams,
    ) -> ControlFlow<async_lsp::Result<()>> {
        ControlFlow::Continue(())
    }

    fn did_open(&mut self, params: DidOpenTextDocumentParams) -> Self::NotifyResult {
        let uri = params.text_document.uri;
        let content = Rope::from(params.text_document.text);
        let language_id = params.text_document.language_id;

        self.state.open_file(&uri, content, Some(language_id));

        ControlFlow::Continue(())
    }

    fn did_change(&mut self, params: DidChangeTextDocumentParams) -> Self::NotifyResult {
        let uri = params.text_document.uri;

        if !params.content_changes.is_empty() {
            self.state.change_file(&uri, params.content_changes);
        }
        ControlFlow::Continue(())
    }

    fn did_save(&mut self, params: DidSaveTextDocumentParams) -> Self::NotifyResult {
        let uri = params.text_document.uri;
        let content = Rope::from(params.text.unwrap());
        self.state.apply_content_change(&uri, content);
        ControlFlow::Continue(())
    }

    fn did_close(&mut self, params: DidCloseTextDocumentParams) -> Self::NotifyResult {
        let uri = params.text_document.uri;
        self.state.clean_file(&uri);
        ControlFlow::Continue(())
    }

    fn completion(
        &mut self,
        params: CompletionParams,
    ) -> BoxFuture<'static, Result<Option<CompletionResponse>, ResponseError>> {
        let uri = params.text_document_position.text_document.uri;
        let pos = params.text_document_position.position;
        let doc = self.state.get_contents(&uri);
        let lang_id = self.state.get_language_id(&uri);
        let root = self.state.root.clone();
        let snippets = [
            Snippets::get_lang(lang_id.clone(), &root),
            Snippets::get_global(&root),
        ]
        .into_iter()
        .fold(Snippets::default(), |mut lang, other| {
            lang.extend(other);
            lang
        });

        let line = doc.get_line(pos.line as usize).unwrap();

        if is_field(&line, pos.character as usize) {
            return Box::pin(async move { Ok(None) });
        }

        let mut cursor_word = String::new();

        let snippets = match get_current_word(&line, pos.character as usize) {
            Some(word) => {
                cursor_word = word.to_string();
                snippets.filter(word).ok().unwrap()
            }
            None => snippets,
        };

        let mut clipboard_ctx = ClipboardContext::new().unwrap();

        let variable_init = VariableInit {
            file_path: uri.to_file_path().unwrap(),
            work_path: root.clone(),
            line_pos: params.text_document_position.position.line as usize,
            cursor_pos: pos.character as usize,
            line_text: line.to_string(),
            current_word: cursor_word,
            selected_text: Default::default(),
            clipboard: clipboard_ctx.get_contents().ok(),
        };

        let items = snippets.to_completion_items(&variable_init);

        Box::pin(async move { Ok(Some(CompletionResponse::Array(items))) })
    }

    fn code_action(
        &mut self,
        params: CodeActionParams,
    ) -> BoxFuture<'static, Result<Option<CodeActionResponse>, ResponseError>> {
        self.state.action_cache_clear();

        let uri = params.text_document.uri.clone();
        let state = self.state.clone();

        let doc = state.get_contents(&uri);
        let lang_id = state.get_language_id(&uri);
        let root = state.root.clone();

        // 当前行
        let line = doc.get_line(params.range.end.line as usize).unwrap();
        // 当前 word
        let cursor_word =
            get_current_word(&line, params.range.end.character as usize).unwrap_or_default();
        // 当前 选择区域
        let range_content = get_range_content(&doc, &params.range).unwrap_or("".into());

        let mut clipboard_ctx = ClipboardContext::new().unwrap();

        let variable_init = VariableInit {
            file_path: uri.to_file_path().unwrap(),
            work_path: root.clone(),
            line_pos: params.range.start.line as usize,
            cursor_pos: params.range.end.character as usize,
            line_text: line.to_string(),
            current_word: cursor_word.to_string(),
            selected_text: range_content.to_string(),
            clipboard: clipboard_ctx.get_contents().ok(),
        };

        let actions = Actions::get_lang(lang_id.clone(), &variable_init);

        let actions = actions
            .to_code_action_items(&variable_init, &params.clone().into())
            .iter()
            .map(|(action, data)| {
                self.state
                    .action_cache_set(action.title.clone(), data.clone());
                action.clone().into()
            })
            .chain(case_actions(range_content.to_string(), &params))
            .chain(markdown_actions(
                lang_id,
                &doc,
                &range_content.to_string(),
                &params,
            ))
            .collect();

        Box::pin(async move { Ok(Some(actions)) })
    }

    fn code_action_resolve(
        &mut self,
        params: CodeAction,
    ) -> BoxFuture<'static, Result<CodeAction, ResponseError>> {
        // let data: ActionData = serde_json::from_value(params.data.clone().unwrap()).unwrap();
        let data = self
            .state
            .action_cache_get(params.title.clone())
            .expect("Unkown Action");

        let uri = data.params.text_document.uri;

        let range = data.params.range;
        let selected = if range.start != range.end {
            let state = self.state.clone();
            let doc = state.get_contents(&uri);
            // let lang_id = state.get_language_id(&uri);
            // let root = state.root.clone();
            let range_content = get_range_content(&doc, &range).unwrap_or("".into()).into();
            Some(range_content)
        } else {
            None
        };

        // 设置 title 和 tooltip
        let mut resolved_action = params.clone();

        if let Some(output) = data
            .command
            .and_then(|cmd| shell(&cmd, &selected).ok())
            .filter(|o| !o.is_empty())
        {
            // resolved_action.data = Some(serde_json::to_value(output.clone()).unwrap());
            resolved_action.data = None;
            let mut changes = HashMap::new();
            let edits = vec![TextEdit {
                range: data.params.range,
                new_text: output,
            }];
            changes.insert(uri.clone(), edits);
            resolved_action.edit = Some(WorkspaceEdit::new(changes));
            resolved_action.kind = Some(CodeActionKind::REFACTOR_REWRITE);
        }

        Box::pin(async move { Ok(resolved_action) })
    }

    fn document_color(
        &mut self,
        params: DocumentColorParams,
    ) -> BoxFuture<'static, Result<Vec<ColorInformation>, ResponseError>> {
        let uri = params.text_document.uri;
        let doc = self.state.get_contents(&uri);

        let current_hash = self.state.calculate_content_hash(&uri);

        // 尝试获取缓存
        let colors = if let Some(cached_colors) = self.state.cached_colors_get(&uri, current_hash) {
            cached_colors
        } else {
            // 缓存处理
            let colors = extract_colors(&doc);
            self.state
                .color_cache_set(&uri, current_hash, colors.clone());
            colors
        };

        Box::pin(async move { Ok(colors) })
    }

    fn shutdown(&mut self, _: ()) -> BoxFuture<'static, Result<(), ResponseError>> {
        info!("shutdown...");
        Box::pin(async move { Ok(()) })
    }
}
