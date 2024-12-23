use flexi_logger::{FileSpec, Logger, WriteMode};
use lsp_server::Connection;

use hx_lsp::{
    lsp::{server_capabilities, Server},
    variables::get_time_offset,
    Result,
};
use lsp_types::InitializeParams;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    if let Some(arg) = std::env::args().nth(1) {
        if arg.eq("--version") {
            let version = env!("CARGO_PKG_VERSION");
            eprintln!("version: {version}");
        }
    }
    let _ = get_time_offset();
    Server::run().await;
}
