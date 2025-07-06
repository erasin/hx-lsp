use hx_lsp::{serve::Server, variables};

#[tokio::main(flavor = "current_thread")]
async fn main() {
    if let Some(arg) = std::env::args().nth(1) {
        if arg.eq("--version") {
            let version = env!("CARGO_PKG_VERSION");
            eprintln!("version: {version}");
        }
    }
    variables::init();
    Server::run().await;
}
