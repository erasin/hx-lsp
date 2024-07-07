use thiserror::Error;

/// 自定义错误，兼容项目错误内容
#[derive(Debug, Error, miette::Diagnostic)]
pub enum Error {
    #[error("Unsupported LSP request: {request}")]
    #[diagnostic(code(lsp::unsupported_lsp_request))]
    UnsupportedLspRequest { request: String },

    #[error("position {0}:{1} is out of bounds")]
    PositionOutOfBounds(u32, u32),

    #[error("clipboard provider: stdin is missing")]
    ClipboardMissStdin,

    #[error("clipboard provider: stdout is missing")]
    ClipboardMissStdout,

    #[error("clipboard provider {0} failed")]
    ClipboardFail(&'static str),

    #[error("Not Found: {0}")]
    NotFound(String),
}

impl<T> From<Error> for crate::Result<T> {
    fn from(val: Error) -> Self {
        Err(Box::new(val))
    }
}
