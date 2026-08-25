// ir/src/lowering/expr.rs

use infra::Span;
use syntax::cst::{ChainStepKind as CstChainStepKind, NodeId, NodeKind};

use crate::hir::{
    BinaryOp, CallArg, ChainStep, ChainStepKind, Expr, ExprKind, Literal, Name, QuantifierKind,
    UnaryOp,
};

use super::context::LoweringContext;

impl<'a> LoweringContext<'a> {
    /// Lower one CST expression into the expression arena.
    pub(crate) fn lower_expr(&mut self, id: NodeId) -> crate::hir::ExprId {
        let Some((kind, span)) = self.node_shape(id) else {
            return self.error_expr(Span::dummy());
        };
        let expression = match kind {
            NodeKind::LitInteger => ExprKind::Literal(Literal::Integer(self.parse_integer(span))),
            NodeKind::LitFloat => ExprKind::Literal(Literal::Float(self.parse_float(span))),
            NodeKind::LitString => ExprKind::Literal(Literal::String(self.parse_string(span))),
            NodeKind::LitChar => ExprKind::Literal(Literal::Char(self.source(span))),
            NodeKind::LitRegex => ExprKind::Literal(Literal::Regex(self.source(span))),
            NodeKind::LitBool(value) => ExprKind::Literal(Literal::Bool(value)),
            NodeKind::Ident(name) => ExprKind::Name(Name { span: name }),
            NodeKind::UnaryOp { op, expr } => ExprKind::Unary {
                op: self.lower_unary_op(op),
                expr: self.lower_expr(expr),
            },
            NodeKind::BinaryOp { op, lhs, rhs } => ExprKind::Binary {
                op: self.lower_binary_op(op),
                lhs: self.lower_expr(lhs),
                rhs: self.lower_expr(rhs),
            },
            NodeKind::Call { func, args } => ExprKind::Call {
                callee: self.lower_expr(func),
                arguments: args
                    .into_iter()
                    .map(|argument| self.lower_call_arg(argument))
                    .collect(),
            },
            NodeKind::ChainExpr { base, steps } => ExprKind::Chain {
                base: self.lower_expr(base),
                steps: steps
                    .into_iter()
                    .map(|step| self.lower_chain_step(step))
                    .collect(),
            },
            NodeKind::Index { base, index, .. } => ExprKind::Index {
                base: self.lower_expr(base),
                index: self.lower_expr(index),
            },
            NodeKind::MatchExpr { scrutinee, arms } => {
                let match_id = self.lower_match(span, scrutinee, arms);
                ExprKind::Match(match_id)
            }
            NodeKind::Quantifier { kind, conditions } => ExprKind::Quantifier {
                kind: self.lower_quantifier_kind(kind),
                conditions: conditions
                    .into_iter()
                    .map(|condition| self.lower_expr(condition))
                    .collect(),
            },
            other => {
                self.report(
                    "unsupported-expression",
                    format!("cannot lower {other:?} as an expression"),
                    span,
                );
                ExprKind::Error
            }
        };
        self.arena.alloc_expr(Expr {
            span,
            kind: expression,
        })
    }

    /// Lower a call argument, including the CST's named-argument wrapper.
    fn lower_call_arg(&mut self, id: NodeId) -> CallArg {
        let Some((kind, span)) = self.node_shape(id) else {
            return CallArg {
                span: Span::dummy(),
                name: None,
                value: self.error_expr(Span::dummy()),
            };
        };
        if let NodeKind::NamedArg { name, value } = kind {
            return CallArg {
                span,
                name: Some(Name { span: name }),
                value: self.lower_expr(value),
            };
        }
        CallArg {
            span,
            name: None,
            value: self.lower_expr(id),
        }
    }

    /// Lower one chain step while keeping all operands in typed arenas.
    fn lower_chain_step(&mut self, step: syntax::cst::ChainStep) -> ChainStep {
        let (kind, end) = match step.kind {
            CstChainStepKind::FieldAccess(name) => (
                ChainStepKind::Field {
                    name: Name { span: name },
                },
                name.end,
            ),
            CstChainStepKind::Call {
                args, close_paren, ..
            } => (
                ChainStepKind::Call {
                    arguments: args
                        .into_iter()
                        .map(|argument| self.lower_call_arg(argument))
                        .collect(),
                },
                close_paren.end,
            ),
            CstChainStepKind::Index {
                index,
                close_bracket,
                ..
            } => (
                ChainStepKind::Index {
                    index: self.lower_expr(index),
                },
                close_bracket.end,
            ),
        };
        let span = Span::new(step.dot_token.start, end);
        ChainStep { span, kind }
    }

    fn lower_quantifier_kind(&mut self, kind: syntax::cst::QuantifierKind) -> QuantifierKind {
        match kind {
            syntax::cst::QuantifierKind::All => QuantifierKind::All,
            syntax::cst::QuantifierKind::Any => QuantifierKind::Any,
            syntax::cst::QuantifierKind::One => QuantifierKind::One,
            syntax::cst::QuantifierKind::None => QuantifierKind::None,
        }
    }

    fn lower_unary_op(&mut self, span: Span) -> UnaryOp {
        let operator = self.source(span);
        match operator.as_str() {
            "-" => UnaryOp::Negate,
            "not" => UnaryOp::Not,
            operator => {
                self.report(
                    "unsupported-unary-operator",
                    format!("unsupported unary operator `{operator}`"),
                    span,
                );
                UnaryOp::Not
            }
        }
    }

    fn lower_binary_op(&mut self, span: Span) -> BinaryOp {
        let operator = self.source(span);
        let Some(value) = binary_op(operator.as_str()) else {
            self.report(
                "unsupported-binary-operator",
                format!("unsupported binary operator `{operator}`"),
                span,
            );
            return BinaryOp::Add;
        };
        value
    }

    fn parse_integer(&mut self, span: Span) -> i128 {
        let source = self.source(span);
        match source.parse() {
            Ok(value) => value,
            Err(_) => {
                self.report(
                    "invalid-integer-literal",
                    format!("invalid integer literal `{source}`"),
                    span,
                );
                0
            }
        }
    }

    fn parse_float(&mut self, span: Span) -> f64 {
        let source = self.source(span);
        match source.parse() {
            Ok(value) => value,
            Err(_) => {
                self.report(
                    "invalid-float-literal",
                    format!("invalid float literal `{source}`"),
                    span,
                );
                0.0
            }
        }
    }

    fn parse_string(&mut self, span: Span) -> String {
        let source = self.source(span);
        if source.starts_with('`') && source.ends_with('`') && source.len() >= 2 {
            return source[1..source.len() - 1].to_owned();
        }
        source
    }

    pub(crate) fn source(&mut self, span: Span) -> String {
        self.source_text(span).unwrap_or_default()
    }
}

fn binary_op(operator: &str) -> Option<BinaryOp> {
    Some(match operator {
        "<" => BinaryOp::Lt,
        "less" => BinaryOp::Less,
        ">" => BinaryOp::Gt,
        "more" => BinaryOp::More,
        "<=" => BinaryOp::LtEq,
        "least" => BinaryOp::Least,
        ">=" => BinaryOp::GtEq,
        "most" => BinaryOp::Most,
        "equals" => BinaryOp::Equals,
        "contains" => BinaryOp::Contains,
        "matches" => BinaryOp::Matches,
        "starts" => BinaryOp::Starts,
        "ends" => BinaryOp::Ends,
        "in" => BinaryOp::In,
        "+" => BinaryOp::Add,
        "-" => BinaryOp::Subtract,
        "*" => BinaryOp::Multiply,
        "/" => BinaryOp::Divide,
        "%" => BinaryOp::Remainder,
        _ => return None,
    })
}
