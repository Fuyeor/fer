// ir/src/hir/expr.rs

use infra::Span;

use super::id::{ExprId, MatchId};
use super::item::Name;

/// An expression stored in the expression arena.
#[derive(Debug, Clone, PartialEq)]
pub struct Expr {
    pub span: Span,
    pub kind: ExprKind,
}

/// Semantic expression forms independent of source formatting.
#[derive(Debug, Clone, PartialEq)]
pub enum ExprKind {
    Literal(Literal),
    Name(Name),
    Unary {
        op: UnaryOp,
        expr: ExprId,
    },
    Binary {
        op: BinaryOp,
        lhs: ExprId,
        rhs: ExprId,
    },
    Call {
        callee: ExprId,
        arguments: Vec<CallArg>,
    },
    InterpolatedString {
        parts: Vec<InterpolatedPart>,
    },
    Chain {
        base: ExprId,
        steps: Vec<ChainStep>,
    },
    Index {
        base: ExprId,
        index: ExprId,
    },
    Match(MatchId),
    Quantifier {
        kind: QuantifierKind,
        conditions: Vec<ExprId>,
    },
    Error,
}

/// One literal or expression segment in an interpolated string.
#[derive(Debug, Clone, PartialEq)]
pub enum InterpolatedPart {
    Text(String),
    Expr(ExprId),
}

/// Quantifier forms used to combine condition expressions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuantifierKind {
    All,
    Any,
    One,
    None,
}

/// Literal values that can be represented without syntax delimiters.
#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    Integer(i128),
    Float(f64),
    String(String),
    Bool(bool),
    Regex(String),
    Char(String),
}

/// Unary operators supported by the current Fer grammar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Negate,
    Not,
}

/// Binary operators normalized from Fer keyword and symbolic tokens.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    Lt,
    Less,
    Gt,
    More,
    LtEq,
    Least,
    GtEq,
    Most,
    Equals,
    Contains,
    Matches,
    Starts,
    Ends,
    In,
    Add,
    Subtract,
    Multiply,
    Divide,
    Remainder,
}

/// One positional or named function-call argument.
#[derive(Debug, Clone, PartialEq)]
pub struct CallArg {
    pub span: Span,
    pub name: Option<Name>,
    pub value: ExprId,
}

/// One semantic step in a field/method access chain.
#[derive(Debug, Clone, PartialEq)]
pub struct ChainStep {
    pub span: Span,
    pub kind: ChainStepKind,
}

/// The operation represented by one chain step.
#[derive(Debug, Clone, PartialEq)]
pub enum ChainStepKind {
    Field { name: Name },
    Call { arguments: Vec<CallArg> },
    Index { index: ExprId },
}
