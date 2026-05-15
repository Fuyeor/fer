// query/src/db.rs

use infra::symbol::Symbol;
use vfs::source::FileId;

/// The central Database trait for the query system (Salsa-like).
pub trait Database {
    // Syntax layer
    fn get_source(&self, file_id: FileId) -> String;
    fn parse_cst(&self, file_id: FileId) -> CstNodeIdx;

    // Semantic layer
    fn resolve_symbol(&self, symbol: Symbol) -> SymbolIdx;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CstNodeIdx(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SymbolIdx(pub u32);
