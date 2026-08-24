// ir/src/hir/body.rs

use infra::Span;

use super::id::{ExprId, HirId};

/// An indexed function or block body.
#[derive(Debug, Clone, PartialEq)]
pub struct Body {
    pub span: Span,
    pub statements: Vec<Stmt>,
}

/// A semantic statement containing only typed arena references.
#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    Expr {
        span: Span,
        expr: ExprId,
    },
    Assign {
        span: Span,
        annotations: Vec<HirId>,
        target: ExprId,
        value: ExprId,
    },
    Item(HirId),
    Error(HirId),
}
