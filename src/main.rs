use flexi_logger::{FileSpec, Logger, WriteMode};
use lsp_server::Connection;

use hx_lsp::{
    lsp::{server_capabilities, Server},
    variables::get_time_offset,
    Result,
};
use lsp_types::InitializeParams;

fn main() -> Result<()> {
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
    let _ = get_time_offset();
    let mut server = Server::new();
    server.run()
}
