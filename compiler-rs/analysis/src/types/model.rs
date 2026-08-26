// compiler-rs/analysis/src/types/model.rs

use infra::{Diagnostic, Span};
use ir::hir::{BodyId, ExprId, HirId};
use vfs::FileId;

use crate::resolve::LocalId;

/// A canonical analysis-owned type identifier.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TypeId(pub u32);

impl TypeId {
    /// Construct an ID from an arena index.
    pub const fn new(index: usize) -> Self {
        Self(index as u32)
    }

    /// Return the arena index represented by this ID.
    pub const fn index(self) -> usize {
        self.0 as usize
    }
}

/// A function type stored in the canonical type arena.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionType {
    pub params: Vec<TypeId>,
    pub return_type: TypeId,
}

/// Canonical semantic types understood by the first type-analysis phase.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeKind {
    Bool,
    Char,
    Integer { signed: bool, bits: u16 },
    Float { bits: u16 },
    String,
    Regex,
    Unit,
    Never,
    Struct(HirId),
    Enum(HirId),
    Function(FunctionType),
    Unknown,
    Error,
}

/// A resolved explicit type reference and its source location.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeRefResolution {
    pub owner: HirId,
    pub type_id: TypeId,
    pub span: Span,
}

/// A collected function signature, independent from its body implementation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionSignature {
    pub item: HirId,
    pub params: Vec<TypeId>,
    pub return_type: TypeId,
    pub body: BodyId,
}

/// Type namespace and item-signature information collected before body checking.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeCollection {
    pub file_id: FileId,
    pub node_types: Vec<Option<TypeId>>,
    pub signatures: Vec<FunctionSignature>,
    pub type_refs: Vec<TypeRefResolution>,
    pub types: Vec<TypeKind>,
    pub diagnostics: Vec<Diagnostic>,
}

impl TypeCollection {
    /// Find the collected signature for a function item.
    pub fn signature(&self, item: HirId) -> Option<&FunctionSignature> {
        self.signatures
            .iter()
            .find(|signature| signature.item == item)
    }

    /// Return the collected type for a HIR node.
    pub fn node_type(&self, item: HirId) -> Option<TypeId> {
        self.node_types.get(item.index()).copied().flatten()
    }

    /// Return a canonical type by ID.
    pub fn kind(&self, id: TypeId) -> Option<&TypeKind> {
        self.types.get(id.index())
    }
}

/// Read-only type information and diagnostics for one HIR file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeTable {
    pub file_id: FileId,
    pub expr_types: Vec<Option<TypeId>>,
    pub definition_types: Vec<Option<TypeId>>,
    pub local_types: Vec<Option<TypeId>>,
    pub signatures: Vec<FunctionSignature>,
    pub type_refs: Vec<TypeRefResolution>,
    pub types: Vec<TypeKind>,
    pub diagnostics: Vec<Diagnostic>,
}

impl TypeTable {
    /// Return the inferred type for an expression ID.
    pub fn type_of(&self, id: ExprId) -> Option<TypeId> {
        self.expr_types.get(id.index()).copied().flatten()
    }

    /// Return the canonical type kind for a type ID.
    pub fn kind(&self, id: TypeId) -> Option<&TypeKind> {
        self.types.get(id.index())
    }

    /// Return one collected and checked function signature.
    pub fn signature(&self, item: HirId) -> Option<&FunctionSignature> {
        self.signatures
            .iter()
            .find(|signature| signature.item == item)
    }

    /// Return the inferred type for an analysis-owned local.
    pub fn local_type(&self, id: LocalId) -> Option<TypeId> {
        self.local_types.get(id.index()).copied().flatten()
    }

    /// Return the inferred type for a resolved definition record.
    pub fn definition_type(&self, index: usize) -> Option<TypeId> {
        self.definition_types.get(index).copied().flatten()
    }
}

/// Mutable canonical type arena used only while producing a table.
#[derive(Debug, Clone)]
pub(crate) struct TypeStore {
    pub(crate) types: Vec<TypeKind>,
}

impl TypeStore {
    pub(crate) fn new() -> Self {
        Self { types: Vec::new() }
    }

    pub(crate) fn intern(&mut self, kind: TypeKind) -> TypeId {
        if let Some(index) = self.types.iter().position(|existing| existing == &kind) {
            return TypeId::new(index);
        }
        let id = TypeId::new(self.types.len());
        self.types.push(kind);
        id
    }

    pub(crate) fn kind(&self, id: TypeId) -> Option<&TypeKind> {
        self.types.get(id.index())
    }

    pub(crate) fn unknown(&mut self) -> TypeId {
        self.intern(TypeKind::Unknown)
    }

    pub(crate) fn error(&mut self) -> TypeId {
        self.intern(TypeKind::Error)
    }

    pub(crate) fn unit(&mut self) -> TypeId {
        self.intern(TypeKind::Unit)
    }

    pub(crate) fn bool(&mut self) -> TypeId {
        self.intern(TypeKind::Bool)
    }

    pub(crate) fn integer(&mut self, signed: bool, bits: u16) -> TypeId {
        self.intern(TypeKind::Integer { signed, bits })
    }

    pub(crate) fn float(&mut self, bits: u16) -> TypeId {
        self.intern(TypeKind::Float { bits })
    }
}
