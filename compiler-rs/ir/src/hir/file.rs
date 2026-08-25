// ir/src/hir/file.rs

use infra::Diagnostic;
use vfs::FileId;

use super::arena::HirArena;
use super::id::HirId;

/// The complete lowered representation of one source file.
#[derive(Debug, Clone, PartialEq)]
pub struct HirFile {
    pub file_id: FileId,
    pub items: Vec<HirId>,
    pub arena: HirArena,
    pub diagnostics: Vec<Diagnostic>,
}
