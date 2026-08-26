// compiler-rs/fer-lsp/src/document.rs

use std::collections::HashMap;
use std::sync::Arc;

use tower_lsp_server::ls_types::{TextDocumentContentChangeEvent, TextDocumentItem, Uri};

#[derive(Debug, Clone)]
pub struct DocumentSnapshot {
    pub uri: Uri,
    pub version: i32,
    pub source: Arc<str>,
}

#[derive(Debug)]
pub enum DocumentError {
    UnknownDocument,
    UnsupportedIncrementalChange,
}

#[derive(Debug, Default)]
pub struct DocumentStore {
    documents: HashMap<Uri, StoredDocument>,
}

#[derive(Debug)]
struct StoredDocument {
    version: i32,
    source: Arc<str>,
}

impl DocumentStore {
    /// Store an opened document and return its immutable analysis snapshot.
    pub fn open(&mut self, document: TextDocumentItem) -> DocumentSnapshot {
        let snapshot = DocumentSnapshot {
            uri: document.uri.clone(),
            version: document.version,
            source: Arc::from(document.text),
        };
        self.documents.insert(
            document.uri,
            StoredDocument {
                version: snapshot.version,
                source: snapshot.source.clone(),
            },
        );
        snapshot
    }

    /// Apply one full-text update if it is newer than the stored version.
    pub fn change(
        &mut self,
        uri: Uri,
        version: i32,
        changes: &[TextDocumentContentChangeEvent],
    ) -> Result<Option<DocumentSnapshot>, DocumentError> {
        let Some(document) = self.documents.get_mut(&uri) else {
            return Err(DocumentError::UnknownDocument);
        };
        if version <= document.version {
            return Ok(None);
        }
        if changes.len() != 1 || changes[0].range.is_some() {
            return Err(DocumentError::UnsupportedIncrementalChange);
        }

        document.version = version;
        document.source = Arc::from(changes[0].text.clone());
        Ok(Some(DocumentSnapshot {
            uri,
            version,
            source: document.source.clone(),
        }))
    }

    /// Return the current immutable snapshot for an open document.
    pub fn snapshot(&self, uri: &Uri) -> Option<DocumentSnapshot> {
        self.documents.get(uri).map(|document| DocumentSnapshot {
            uri: uri.clone(),
            version: document.version,
            source: document.source.clone(),
        })
    }

    /// Remove a document and report whether a stored snapshot was cleared.
    pub fn close(&mut self, uri: &Uri) -> bool {
        self.documents.remove(uri).is_some()
    }
}
