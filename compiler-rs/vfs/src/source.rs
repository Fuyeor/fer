// vfs/src/source.rs

use crate::path::VirtualPath;
use infra::Span;
use infra::symbol::Interner;
use std::collections::HashMap;

/// Opaque identifier for a source file in the virtual filesystem.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FileId(pub u32);

/// The complete source text of a file, along with its path.
#[derive(Debug, Clone)]
pub struct SourceFile {
    pub path: VirtualPath,
    pub content: String,
}

/// Central storage for all source files.
/// Provides content lookup by `FileId` and bi‑directional mapping to paths.
pub struct SourceMap {
    interner: Interner,
    files: Vec<SourceFile>, // indexed by FileId.0 as vec index
    path_to_id: HashMap<VirtualPath, FileId>,
}

impl SourceMap {
    pub fn new() -> Self {
        Self {
            interner: Interner::new(),
            files: Vec::new(),
            path_to_id: HashMap::new(),
        }
    }

    /// Add a file and return its unique `FileId`.
    ///
    /// The `path_str` is validated and interned via the internal `Interner`.
    /// Returns `None` if the path is invalid.
    pub fn add_file(&mut self, path_str: &str, content: String) -> Option<FileId> {
        let path = VirtualPath::new(path_str, &mut self.interner)?;
        if let Some(&id) = self.path_to_id.get(&path) {
            self.files[id.0 as usize] = SourceFile { path, content };
            return Some(id);
        }
        let id = FileId(self.files.len() as u32);
        self.files.push(SourceFile {
            path: path.clone(),
            content,
        });
        self.path_to_id.insert(path, id);
        Some(id)
    }

    /// Retrieve the source content for a file.
    pub fn content(&self, id: FileId) -> Option<&str> {
        self.files.get(id.0 as usize).map(|f| f.content.as_str())
    }

    /// Retrieve the `VirtualPath` for a file.
    pub fn path(&self, id: FileId) -> Option<VirtualPath> {
        self.files.get(id.0 as usize).map(|f| f.path)
    }

    /// Look up a `FileId` by its path.
    pub fn id_of(&self, path: VirtualPath) -> Option<FileId> {
        self.path_to_id.get(&path).copied()
    }

    /// Convert a byte‑offset `Span` inside the given file into a `(line, column)` pair.
    /// Lines are 1‑based, columns are 0‑based (UTF‑8 bytes).
    pub fn resolve(&self, id: FileId, span: Span) -> Option<(u32, u32)> {
        let content = self.content(id)?;
        if span.start > content.len() {
            return None;
        }
        let prefix = &content[..span.start];
        let line = prefix.bytes().filter(|&b| b == b'\n').count() as u32 + 1;
        let last_newline = prefix.rfind('\n').map(|i| i + 1).unwrap_or(0);
        let col = (span.start - last_newline) as u32;
        Some((line, col))
    }

    /// Expose the interner for external use (e.g., diagnostics).
    pub fn interner(&self) -> &Interner {
        &self.interner
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_and_retrieve_content() {
        let mut map = SourceMap::new();
        let id = map.add_file("test.fer", "hello".into()).unwrap();
        assert_eq!(map.content(id), Some("hello"));
    }

    #[test]
    fn id_of_returns_correct_id() {
        let mut map = SourceMap::new();
        let id = map.add_file("lib.fer", "".into()).unwrap();
        let path = map.path(id).unwrap();
        assert_eq!(map.id_of(path), Some(id));
    }

    #[test]
    fn resolve_span_line_col() {
        let mut map = SourceMap::new();
        let id = map.add_file("lines.fer", "abc\ndef\nghi".into()).unwrap();
        let span = Span::new(4, 4); // start of second line (0-based)
        let (line, col) = map.resolve(id, span).unwrap();
        assert_eq!(line, 2);
        assert_eq!(col, 0);
    }

    #[test]
    fn rejects_invalid_path() {
        let mut map = SourceMap::new();
        assert!(map.add_file("../secret.fer", "".into()).is_none());
        assert!(map.add_file("src/Main.fer", "".into()).is_none());
        assert!(map.add_file("no-ext", "".into()).is_none());
    }
}
