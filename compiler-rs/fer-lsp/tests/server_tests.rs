// compiler-rs/fer-lsp/tests/server_tests.rs

use std::io::{BufRead, BufReader, Read, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

use serde_json::{Value, json};

struct LspProcess {
    child: Child,
    input: ChildStdin,
    output: BufReader<ChildStdout>,
}

impl LspProcess {
    fn spawn() -> Self {
        let binary = std::env::var("CARGO_BIN_EXE_fer-lsp").expect("fer-lsp binary path");
        let mut child = Command::new(binary)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("fer-lsp must start");
        let input = child.stdin.take().expect("LSP stdin must be available");
        let output = BufReader::new(child.stdout.take().expect("LSP stdout must be available"));
        Self {
            child,
            input,
            output,
        }
    }

    fn send(&mut self, message: Value) {
        let payload = serde_json::to_vec(&message).expect("JSON-RPC message must serialize");
        write!(self.input, "Content-Length: {}\r\n\r\n", payload.len())
            .expect("LSP header must write");
        self.input
            .write_all(&payload)
            .expect("LSP payload must write");
        self.input.flush().expect("LSP input must flush");
    }

    fn receive(&mut self) -> Value {
        let mut content_length = None;
        let mut line = String::new();
        loop {
            line.clear();
            self.output
                .read_line(&mut line)
                .expect("LSP header must read");
            assert!(
                !line.is_empty(),
                "LSP server exited before sending a message"
            );
            if line == "\r\n" {
                break;
            }
            if let Some(value) = line.strip_prefix("Content-Length:") {
                content_length = Some(value.trim().parse::<usize>().expect("valid content length"));
            }
        }

        let length = content_length.expect("LSP response must include Content-Length");
        let mut payload = vec![0; length];
        self.output
            .read_exact(&mut payload)
            .expect("LSP payload must read");
        serde_json::from_slice(&payload).expect("LSP payload must be JSON")
    }

    fn wait_for_method(&mut self, method: &str) -> Value {
        loop {
            let message = self.receive();
            if message.get("method").and_then(Value::as_str) == Some(method) {
                return message;
            }
        }
    }
}

impl Drop for LspProcess {
    fn drop(&mut self) {
        let _ = self.child.kill();
    }
}

#[test]
fn publishes_diagnostics_for_unsaved_documents_and_clears_them_after_change() {
    let mut lsp = LspProcess::spawn();
    lsp.send(json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "processId": null,
            "rootUri": null,
            "capabilities": {
                "general": { "positionEncodings": ["utf-16", "utf-8"] }
            }
        }
    }));
    let initialize = lsp.receive();
    assert_eq!(initialize["id"], 1);
    assert_eq!(initialize["result"]["capabilities"]["textDocumentSync"], 1);
    assert_eq!(
        initialize["result"]["capabilities"]["documentFormattingProvider"],
        true
    );

    lsp.send(json!({ "jsonrpc": "2.0", "method": "initialized", "params": {} }));
    lsp.send(json!({
        "jsonrpc": "2.0",
        "method": "textDocument/didOpen",
        "params": {
            "textDocument": {
                "uri": "file:///workspace/main.fer",
                "languageId": "fer",
                "version": 1,
                "text": "main = () -> i64 { missing }"
            }
        }
    }));
    let first = lsp.wait_for_method("textDocument/publishDiagnostics");
    assert_eq!(first["params"]["version"], 1);
    assert_eq!(first["params"]["diagnostics"][0]["code"], "undefined-name");
    assert_eq!(first["params"]["diagnostics"][0]["source"], "fer");
    assert_eq!(first["params"]["diagnostics"][0]["severity"], 1);

    lsp.send(json!({
        "jsonrpc": "2.0",
        "method": "textDocument/didChange",
        "params": {
            "textDocument": { "uri": "file:///workspace/main.fer", "version": 2 },
            "contentChanges": [{ "text": "main = () -> i64 { 42 }" }]
        }
    }));
    let second = lsp.wait_for_method("textDocument/publishDiagnostics");
    assert_eq!(second["params"]["version"], 2);
    assert_eq!(second["params"]["diagnostics"], json!([]));

    lsp.send(json!({
        "jsonrpc": "2.0",
        "method": "textDocument/didChange",
        "params": {
            "textDocument": { "uri": "file:///workspace/main.fer", "version": 3 },
            "contentChanges": [{
                "text": "main=()->i64{\nanswer=40+2\nanswer\n}\n"
            }]
        }
    }));
    let third = lsp.wait_for_method("textDocument/publishDiagnostics");
    assert_eq!(third["params"]["version"], 3);
    assert_eq!(third["params"]["diagnostics"], json!([]));

    lsp.send(json!({
        "jsonrpc": "2.0",
        "id": 3,
        "method": "textDocument/formatting",
        "params": {
            "textDocument": { "uri": "file:///workspace/main.fer" },
            "options": { "tabSize": 2, "insertSpaces": true }
        }
    }));
    let formatting = lsp.receive();
    assert_eq!(formatting["id"], 3);
    assert_eq!(
        formatting["result"][0]["range"]["start"],
        json!({ "line": 0, "character": 0 })
    );
    assert_eq!(
        formatting["result"][0]["range"]["end"],
        json!({ "line": 4, "character": 0 })
    );
    assert_eq!(
        formatting["result"][0]["newText"],
        "main = () -> i64 {\n  answer = 40 + 2\n  answer\n}\n"
    );

    lsp.send(json!({
        "jsonrpc": "2.0",
        "method": "textDocument/didClose",
        "params": { "textDocument": { "uri": "file:///workspace/main.fer" } }
    }));
    let fourth = lsp.wait_for_method("textDocument/publishDiagnostics");
    assert_eq!(fourth["params"]["version"], Value::Null);
    assert_eq!(fourth["params"]["diagnostics"], json!([]));

    lsp.send(json!({ "jsonrpc": "2.0", "id": 2, "method": "shutdown", "params": null }));
    let shutdown = lsp.receive();
    assert_eq!(shutdown["id"], 2);
    assert_eq!(shutdown["result"], Value::Null);
    lsp.send(json!({ "jsonrpc": "2.0", "method": "exit", "params": null }));
    assert!(lsp.child.wait().expect("LSP process must exit").success());
}
