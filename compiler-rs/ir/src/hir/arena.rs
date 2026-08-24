// ir/src/hir/arena.rs

use super::body::Body;
use super::expr::Expr;
use super::id::{BodyId, ConditionId, ExprId, HirId, MatchArmId, MatchId};
use super::item::HirNode;
use super::r#match::{Condition, Match, MatchArm};

/// Flat storage for every HIR entity referenced by a typed integer ID.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct HirArena {
    pub nodes: Vec<HirNode>,
    pub bodies: Vec<Body>,
    pub exprs: Vec<Expr>,
    pub matches: Vec<Match>,
    pub match_arms: Vec<MatchArm>,
    pub conditions: Vec<Condition>,
}

impl HirArena {
    /// Allocate a top-level HIR node and return its stable arena ID.
    pub fn alloc_node(&mut self, node: HirNode) -> HirId {
        let id = HirId::new(next_index(self.nodes.len()));
        self.nodes.push(node);
        id
    }

    /// Read a node through a checked typed ID.
    pub fn node(&self, id: HirId) -> Option<&HirNode> {
        self.nodes.get(id.index())
    }

    /// Read a body through a checked typed ID.
    pub fn body(&self, id: BodyId) -> Option<&Body> {
        self.bodies.get(id.index())
    }

    /// Read an expression through a checked typed ID.
    pub fn expr(&self, id: ExprId) -> Option<&Expr> {
        self.exprs.get(id.index())
    }

    /// Read a match through a checked typed ID.
    pub fn match_expr(&self, id: MatchId) -> Option<&Match> {
        self.matches.get(id.index())
    }

    /// Read a match arm through a checked typed ID.
    pub fn match_arm(&self, id: MatchArmId) -> Option<&MatchArm> {
        self.match_arms.get(id.index())
    }

    /// Read a condition through a checked typed ID.
    pub fn condition(&self, id: ConditionId) -> Option<&Condition> {
        self.conditions.get(id.index())
    }

    /// Allocate a body in the body arena.
    pub fn alloc_body(&mut self, body: Body) -> BodyId {
        let id = BodyId::new(next_index(self.bodies.len()));
        self.bodies.push(body);
        id
    }

    /// Allocate an expression in the expression arena.
    pub fn alloc_expr(&mut self, expr: Expr) -> ExprId {
        let id = ExprId::new(next_index(self.exprs.len()));
        self.exprs.push(expr);
        id
    }

    /// Allocate a match expression in the match arena.
    pub fn alloc_match(&mut self, value: Match) -> MatchId {
        let id = MatchId::new(next_index(self.matches.len()));
        self.matches.push(value);
        id
    }

    /// Allocate a match arm in the match-arm arena.
    pub fn alloc_match_arm(&mut self, arm: MatchArm) -> MatchArmId {
        let id = MatchArmId::new(next_index(self.match_arms.len()));
        self.match_arms.push(arm);
        id
    }

    /// Allocate a match condition in the condition arena.
    pub fn alloc_condition(&mut self, condition: Condition) -> ConditionId {
        let id = ConditionId::new(next_index(self.conditions.len()));
        self.conditions.push(condition);
        id
    }
}

fn next_index(length: usize) -> usize {
    assert!(
        length <= u32::MAX as usize,
        "HIR arena cannot contain more than u32::MAX entries"
    );
    length
}
