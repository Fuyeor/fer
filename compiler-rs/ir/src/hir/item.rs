// ir/src/hir/item.rs

use infra::Span;

use super::id::{BodyId, ExprId, HirId};

/// A source-backed semantic name retained for later symbol resolution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Name {
    pub span: Span,
}

/// A top-level HIR node stored in the shared node arena.
#[derive(Debug, Clone, PartialEq)]
pub enum HirNode {
    Item(Item),
    Field(Field),
    EnumVariant(EnumVariant),
    Param(Param),
    Annotation(Annotation),
    Error { span: Span },
}

/// A module item after syntax-specific details have been removed.
#[derive(Debug, Clone, PartialEq)]
pub struct Item {
    pub span: Span,
    pub annotations: Vec<HirId>,
    pub kind: ItemKind,
}

/// The semantic category of a module item.
#[derive(Debug, Clone, PartialEq)]
pub enum ItemKind {
    Const(ConstDef),
    Struct(StructDef),
    Enum(EnumDef),
    Function(FunctionDef),
    Unsupported { span: Span },
}

/// A constant declaration represented by a name and value expression.
#[derive(Debug, Clone, PartialEq)]
pub struct ConstDef {
    pub name: Name,
    pub type_annotation: Option<TypeRef>,
    pub value: ExprId,
}

/// A struct declaration with field node IDs.
#[derive(Debug, Clone, PartialEq)]
pub struct StructDef {
    pub name: Name,
    pub fields: Vec<HirId>,
}

/// An enum declaration with variant node IDs.
#[derive(Debug, Clone, PartialEq)]
pub struct EnumDef {
    pub name: Name,
    pub variants: Vec<HirId>,
}

/// A simple enum variant from the current syntax CST.
#[derive(Debug, Clone, PartialEq)]
pub struct EnumVariant {
    pub span: Span,
    pub name: Name,
}

/// A function declaration with an indexed body.
#[derive(Debug, Clone, PartialEq)]
pub struct FunctionDef {
    pub name: Name,
    pub params: Vec<HirId>,
    pub return_type: Option<TypeRef>,
    pub body: BodyId,
}

/// A typed function parameter.
#[derive(Debug, Clone, PartialEq)]
pub struct Param {
    pub span: Span,
    pub name: Name,
    pub type_ref: TypeRef,
}

/// A struct field with an explicit semantic shape.
#[derive(Debug, Clone, PartialEq)]
pub struct Field {
    pub span: Span,
    pub annotations: Vec<HirId>,
    pub name: Name,
    pub shape: FieldShape,
}

/// The three valid struct-field forms.
#[derive(Debug, Clone, PartialEq)]
pub enum FieldShape {
    Inferred { default: ExprId },
    Typed { type_ref: TypeRef, default: ExprId },
    Required { type_ref: TypeRef },
}

/// A type name preserved until semantic type resolution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeRef {
    pub name: Name,
}

/// A declaration or field annotation.
#[derive(Debug, Clone, PartialEq)]
pub struct Annotation {
    pub span: Span,
    pub name: Name,
    pub arguments: Vec<AnnotationArg>,
}

/// One positional or named annotation argument.
#[derive(Debug, Clone, PartialEq)]
pub struct AnnotationArg {
    pub span: Span,
    pub name: Option<Name>,
    pub value: ExprId,
}
