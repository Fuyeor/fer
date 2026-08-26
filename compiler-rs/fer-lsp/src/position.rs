// compiler-rs/fer-lsp/src/position.rs

use tower_lsp_server::ls_types::{Position, Range};
use vfs::SourceRange;

pub(crate) use vfs::LineIndex;

/// Convert a protocol-independent source range into an LSP UTF-16 range.
pub(crate) const fn to_lsp_range(source_range: SourceRange) -> Range {
    Range::new(
        to_lsp_position(source_range.start),
        to_lsp_position(source_range.end),
    )
}

/// Convert one protocol-independent source position into an LSP position.
const fn to_lsp_position(position: vfs::SourcePosition) -> Position {
    Position::new(position.line, position.utf16_column)
}
