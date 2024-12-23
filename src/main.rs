use hx_lsp::{lsp::Server, variables::get_time_offset};

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
