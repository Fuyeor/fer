// compiler-rs/runtime/src/value.rs

use ir::hir::HirId;

/// A runtime value produced by evaluating Fer HIR.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Unit,
    Integer(i128),
    Float(f64),
    String(String),
    Bool(bool),
    Char(String),
    Regex(String),
    Function(HirId),
}

impl Value {
    /// Return the stable diagnostic category of this value.
    pub(crate) const fn kind(&self) -> ValueKind {
        match self {
            Self::Unit => ValueKind::Unit,
            Self::Integer(_) => ValueKind::Integer,
            Self::Float(_) => ValueKind::Float,
            Self::String(_) => ValueKind::String,
            Self::Bool(_) => ValueKind::Bool,
            Self::Char(_) => ValueKind::Char,
            Self::Regex(_) => ValueKind::Regex,
            Self::Function(_) => ValueKind::Function,
        }
    }
}

/// A runtime value category used in structured execution errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueKind {
    Unit,
    Integer,
    Float,
    String,
    Bool,
    Char,
    Regex,
    Function,
}
