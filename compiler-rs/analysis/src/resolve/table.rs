// compiler-rs/analysis/src/resolve/table.rs

use infra::{Diagnostic, Span};
use ir::hir::{ExprId, HirId};
use vfs::FileId;

use super::BuiltinKind;
use super::scope::{LocalId, Scope, ScopeId};

/// A typed identifier for one definition record.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct BindingId(pub u32);

impl BindingId {
    pub const fn new(index: usize) -> Self {
        Self(index as u32)
    }

    pub const fn index(self) -> usize {
        self.0 as usize
    }
}

/// The HIR or analysis-owned definition targeted by a name expression.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DefTarget {
    Item(HirId),
    Param(HirId),
    Local(LocalId),
}

/// A definition recorded in one lexical scope.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Definition {
    pub id: BindingId,
    pub target: DefTarget,
    pub name: String,
    pub span: Span,
    pub scope: ScopeId,
}

/// Metadata for one analysis-owned local binding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalBinding {
    pub id: LocalId,
    pub name: String,
    pub span: Span,
    pub scope: ScopeId,
}

/// Owned parts used to construct a read-only resolution table.
#[derive(Debug)]
pub(crate) struct ResolutionParts {
    pub(crate) file_id: FileId,
    pub(crate) expr_targets: Vec<Option<DefTarget>>,
    pub(crate) builtin_calls: Vec<Option<BuiltinKind>>,
    pub(crate) assignment_locals: Vec<Option<LocalId>>,
    pub(crate) definitions: Vec<Definition>,
    pub(crate) locals: Vec<LocalBinding>,
    pub(crate) scopes: Vec<Scope>,
    pub(crate) diagnostics: Vec<Diagnostic>,
}

/// Read-only name-resolution output for one HIR file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolutionTable {
    pub file_id: FileId,
    pub expr_targets: Vec<Option<DefTarget>>,
    pub builtin_calls: Vec<Option<BuiltinKind>>,
    pub assignment_locals: Vec<Option<LocalId>>,
    pub definitions: Vec<Definition>,
    pub locals: Vec<LocalBinding>,
    pub scopes: Vec<Scope>,
    pub diagnostics: Vec<Diagnostic>,
}

impl ResolutionTable {
    /// Return the resolved target for a name expression, if resolution succeeded.
    pub fn target(&self, id: ExprId) -> Option<&DefTarget> {
        self.expr_targets.get(id.index()).and_then(Option::as_ref)
    }

    /// Return the resolved target for a name expression, if resolution succeeded.
    pub fn target_for_expr(&self, id: ExprId) -> Option<&DefTarget> {
        self.target(id)
    }

    /// Return the language builtin targeted by a name expression, if any.
    pub fn builtin_for_expr(&self, id: ExprId) -> Option<BuiltinKind> {
        self.builtin_calls.get(id.index()).copied().flatten()
    }

    /// Return the analysis-owned local introduced by an assignment target.
    pub fn assignment_local(&self, id: ExprId) -> Option<LocalId> {
        self.assignment_locals.get(id.index()).copied().flatten()
    }

    /// Return a definition by its stable binding ID.
    pub fn definition(&self, id: BindingId) -> Option<&Definition> {
        self.definitions.get(id.index())
    }

    /// Build a table from the resolver's owned output vectors.
    pub(crate) fn from_parts(parts: ResolutionParts) -> Self {
        Self {
            file_id: parts.file_id,
            expr_targets: parts.expr_targets,
            builtin_calls: parts.builtin_calls,
            assignment_locals: parts.assignment_locals,
            definitions: parts.definitions,
            locals: parts.locals,
            scopes: parts.scopes,
            diagnostics: parts.diagnostics,
        }
    }
}
