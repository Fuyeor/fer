// compiler-rs/fer-lsp/src/main.rs

#[tokio::main]
async fn main() {
    fer_lsp::run_stdio().await;
}
