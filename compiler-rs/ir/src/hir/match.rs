// ir/src/hir/match.rs

use infra::Span;

use super::id::{BodyId, ConditionId, ExprId, MatchArmId};

/// A match expression stored separately from the expression arena.
#[derive(Debug, Clone, PartialEq)]
pub struct Match {
    pub span: Span,
    pub scrutinee: ExprId,
    pub arms: Vec<MatchArmId>,
}

/// One match arm with an optional condition and an indexed body.
#[derive(Debug, Clone, PartialEq)]
pub struct MatchArm {
    pub span: Span,
    pub condition: Option<ConditionId>,
    pub body: BodyId,
}

/// A normalized match condition.
#[derive(Debug, Clone, PartialEq)]
pub struct Condition {
    pub span: Span,
    pub kind: ConditionKind,
}

/// The condition forms currently emitted by the syntax parser.
#[derive(Debug, Clone, PartialEq)]
pub enum ConditionKind {
    Equals(ExprId),
    Predicate { op: ConditionOp, rhs: ExprId },
}

/// Keyword and symbolic predicate operators supported by match patterns.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConditionOp {
    Contains,
    Matches,
    Starts,
    Ends,
    Less,
    More,
    Least,
    Most,
    Equals,
    In,
    Lt,
    Gt,
    LtEq,
    GtEq,
}
