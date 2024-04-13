use std::io;

use crossbeam_channel::SendError;
use lsp_server::{ExtractError, Message};
use thiserror::Error;

/// 自定义错误，兼容项目错误内容
#[derive(Debug, Error, miette::Diagnostic)]
pub enum Error {
    #[error(transparent)]
    #[diagnostic(code(lsp::server_capabilities))]
    ServerCapabilities(#[from] serde_json::Error),

    #[error(transparent)]
    #[diagnostic(code(lsp::server_init))]
    ServerInit(#[from] lsp_server::ProtocolError),

    #[error(transparent)]
    #[diagnostic(code(lsp::io))]
    Io(#[from] io::Error),

    #[error("Unsupported LSP request: {request}")]
    #[diagnostic(code(lsp::unsupported_lsp_request))]
    UnsupportedLspRequest { request: String },

    #[error(transparent)]
    #[diagnostic(code(lsp::cast_request))]
    CastRequest(#[from] ExtractError<lsp_server::Request>),

    #[error(transparent)]
    #[diagnostic(code(lsp::cast_notification))]
    CastNotification(#[from] ExtractError<lsp_server::Notification>),

    #[error(transparent)]
    #[diagnostic(code(lsp::send))]
    Send(#[from] SendError<Message>),

    #[error(transparent)]
    #[diagnostic(code(lsp::send))]
    PathToUri(#[from] url::ParseError),

    #[error(transparent)]
    #[diagnostic(code(lsp::log))]
    Logger(#[from] flexi_logger::FlexiLoggerError),

    #[error("Not Found: {0}")]
    NotFound(String),
    // #[error("unknown data store error")]
    // Unknown,
}
