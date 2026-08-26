// compiler-rs/fer-lsp/src/server.rs

use std::sync::Mutex;

use tower_lsp_server::ls_types::{
    Diagnostic, DidChangeTextDocumentParams, DidCloseTextDocumentParams, DidOpenTextDocumentParams,
    InitializeParams, InitializeResult, InitializedParams, MessageType, PositionEncodingKind,
    ServerCapabilities, ServerInfo, TextDocumentSyncCapability, TextDocumentSyncKind,
};
use tower_lsp_server::{Client, LanguageServer, jsonrpc::Result};

use crate::diagnostics::to_lsp_diagnostics;
use crate::document::{DocumentError, DocumentSnapshot, DocumentStore};
use crate::position::LineIndex;

#[derive(Debug)]
pub struct Backend {
    client: Client,
    documents: Mutex<DocumentStore>,
}

impl Backend {
    /// Create a server backend with isolated in-memory document state.
    pub fn new(client: Client) -> Self {
        Self {
            client,
            documents: Mutex::new(DocumentStore::default()),
        }
    }

    /// Analyze one document snapshot and publish its current diagnostics.
    async fn publish_snapshot(&self, snapshot: DocumentSnapshot) {
        let uri = snapshot.uri.clone();
        let version = snapshot.version;
        let line_index = LineIndex::new(snapshot.source.as_ref());
        let path = source_path(&uri);
        let diagnostics = match analyze_document(&path, snapshot.source.as_ref(), &line_index) {
            Ok(diagnostics) => diagnostics,
            Err(error) => {
                self.client.log_message(MessageType::ERROR, error).await;
                return;
            }
        };
        self.client
            .publish_diagnostics(uri, diagnostics, Some(version))
            .await;
    }

    /// Log a document protocol error without writing non-protocol data to stdout.
    async fn log_document_error(&self, error: DocumentError) {
        self.client
            .log_message(
                MessageType::ERROR,
                format!("document synchronization failed: {error:?}"),
            )
            .await;
    }
}

impl LanguageServer for Backend {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                position_encoding: Some(PositionEncodingKind::UTF16),
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                ..ServerCapabilities::default()
            },
            server_info: Some(ServerInfo {
                name: "fer-lsp".to_owned(),
                version: Some(env!("CARGO_PKG_VERSION").to_owned()),
            }),
            ..InitializeResult::default()
        })
    }

    async fn initialized(&self, _: InitializedParams) {}

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let snapshot = self
            .documents
            .lock()
            .expect("document store mutex must not be poisoned")
            .open(params.text_document);
        self.publish_snapshot(snapshot).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let change = self
            .documents
            .lock()
            .expect("document store mutex must not be poisoned")
            .change(
                params.text_document.uri,
                params.text_document.version,
                &params.content_changes,
            );
        match change {
            Ok(Some(snapshot)) => self.publish_snapshot(snapshot).await,
            Ok(None) => {}
            Err(error) => self.log_document_error(error).await,
        }
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        let removed = self
            .documents
            .lock()
            .expect("document store mutex must not be poisoned")
            .close(&params.text_document.uri);
        if removed {
            self.client
                .publish_diagnostics(params.text_document.uri, Vec::<Diagnostic>::new(), None)
                .await;
        }
    }
}

/// Convert a client URI into the relative path accepted by the Fer virtual file system.
fn source_path(uri: &tower_lsp_server::ls_types::Uri) -> String {
    let Some(path) = uri.to_file_path() else {
        return "untitled.fer".to_owned();
    };
    let path = path.to_string_lossy().replace('\\', "/");
    let path = path.strip_prefix('/').unwrap_or(&path);
    if path.ends_with(".fer") {
        path.to_owned()
    } else {
        "untitled.fer".to_owned()
    }
}

/// Analyze a source snapshot and recover both successful and erroneous diagnostics.
fn analyze_document(
    path: &str,
    source: &str,
    line_index: &LineIndex,
) -> std::result::Result<Vec<Diagnostic>, String> {
    let diagnostics = match fer::analyze_source(path, source) {
        Ok(snapshot) => snapshot.diagnostics,
        Err(fer::DriverError::Diagnostics(diagnostics)) => diagnostics,
        Err(fer::DriverError::InvalidPath) => {
            return Err(format!("cannot analyze document with invalid path: {path}"));
        }
        Err(fer::DriverError::Runtime(_)) => {
            return Err("analysis unexpectedly entered the runtime".to_owned());
        }
    };
    to_lsp_diagnostics(&diagnostics, line_index)
}
