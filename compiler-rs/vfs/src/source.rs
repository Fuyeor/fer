// vfs/src/source.rs

use infra::span::Span;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FileId(pub u32);

pub struct SourceMap {
    // Mapping logic will be implemented here
}

impl SourceMap {
    pub fn resolve(&self, _file: FileId, _span: Span) -> (u32, u32) {
        (0, 0) // (line, column)
    }
}
