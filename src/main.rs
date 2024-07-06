use flexi_logger::{FileSpec, Logger, WriteMode};
use lsp_server::Connection;

use hx_lsp::{
    errors::Error,
    lsp::{server_capabilities, Server},
    variables::get_time_offset,
};
use lsp_types::InitializeParams;

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
    let _ = get_time_offset();
    run_lsp_server()
}

fn run_lsp_server() -> Result<(), Error> {
    log::info!("hx-lsp: server up");

    let (connection, io_threads) = Connection::stdio();

    let server_capabilities =
        serde_json::to_value(server_capabilities()).expect("Must be serializable");

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
    log::info!("hx-lsp: Shutting down server");

    Ok(())
}
