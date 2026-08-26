// compiler-rs/fer-lsp/src/lib.rs

mod diagnostics;
mod document;
mod position;
mod server;

pub use server::Backend;

/// Run the diagnostics-only language server over standard input and output.
pub async fn run_stdio() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();
    let (service, socket) = tower_lsp_server::LspService::new(Backend::new);
    tower_lsp_server::Server::new(stdin, stdout, socket)
        .concurrency_level(1)
        .serve(service)
        .await;
}
