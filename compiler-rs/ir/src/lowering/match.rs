// ir/src/lowering/match.rs

use infra::Span;
use syntax::cst::{NodeId, NodeKind};

use crate::hir::{Condition, ConditionKind, ConditionOp, Match, MatchArm};

use super::context::LoweringContext;

impl<'a> LoweringContext<'a> {
    /// Lower a CST match expression into match, arm, and condition arenas.
    pub(crate) fn lower_match(
        &mut self,
        span: Span,
        scrutinee: NodeId,
        arm_ids: Vec<NodeId>,
    ) -> crate::hir::MatchId {
        let scrutinee = self.lower_expr(scrutinee);
        let arms = arm_ids
            .into_iter()
            .map(|arm| self.lower_match_arm(arm))
            .collect();
        self.arena.alloc_match(Match {
            span,
            scrutinee,
            arms,
        })
    }

    /// Lower one pattern/body pair, with no condition for the default arm.
    fn lower_match_arm(&mut self, id: NodeId) -> crate::hir::MatchArmId {
        let Some((kind, span)) = self.node_shape(id) else {
            let body = self.lower_body(id);
            return self.arena.alloc_match_arm(MatchArm {
                span: Span::dummy(),
                condition: None,
                body,
            });
        };
        let NodeKind::MatchArm { pattern, body } = kind else {
            self.report(
                "invalid-match-arm",
                "expected a MatchArm CST node".into(),
                span,
            );
            let body = self.lower_body(id);
            return self.arena.alloc_match_arm(MatchArm {
                span,
                condition: None,
                body,
            });
        };
        let condition = pattern.map(|pattern| self.lower_pattern(pattern));
        let body = self.lower_body(body);
        self.arena.alloc_match_arm(MatchArm {
            span,
            condition,
            body,
        })
    }

    /// Convert a CST pattern into equality or predicate condition semantics.
    fn lower_pattern(&mut self, id: NodeId) -> crate::hir::ConditionId {
        let Some((kind, span)) = self.node_shape(id) else {
            let error = self.error_expr(Span::dummy());
            return self.arena.alloc_condition(Condition {
                span: Span::dummy(),
                kind: ConditionKind::Equals(error),
            });
        };
        let kind = match kind {
            NodeKind::PatternCondition { op, rhs } => ConditionKind::Predicate {
                op: self.lower_condition_op(op),
                rhs: self.lower_expr(rhs),
            },
            _ => ConditionKind::Equals(self.lower_expr(id)),
        };
        self.arena.alloc_condition(Condition { span, kind })
    }

    fn lower_condition_op(&mut self, span: Span) -> ConditionOp {
        let operator = self.source(span);
        let Some(value) = condition_op(operator.as_str()) else {
            self.report(
                "unsupported-condition-operator",
                format!("unsupported match condition operator `{operator}`"),
                span,
            );
            return ConditionOp::Equals;
        };
        value
    }
}

fn condition_op(operator: &str) -> Option<ConditionOp> {
    Some(match operator {
        "contains" => ConditionOp::Contains,
        "matches" => ConditionOp::Matches,
        "starts" => ConditionOp::Starts,
        "ends" => ConditionOp::Ends,
        "less" => ConditionOp::Less,
        "more" => ConditionOp::More,
        "least" => ConditionOp::Least,
        "most" => ConditionOp::Most,
        "equals" => ConditionOp::Equals,
        "in" => ConditionOp::In,
        "<" => ConditionOp::Lt,
        ">" => ConditionOp::Gt,
        "<=" => ConditionOp::LtEq,
        ">=" => ConditionOp::GtEq,
        _ => return None,
    })
}
