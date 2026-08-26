// compiler-rs/runtime/src/expression.rs

use analysis::resolve::{BuiltinKind, DefTarget};
use ir::hir::{
    BinaryOp, ConditionKind, ExprKind, HirId, HirNode, InterpolatedPart, Literal, UnaryOp,
};

use crate::error::RuntimeError;
use crate::evaluator::Interpreter;
use crate::ops::{evaluate_binary, evaluate_condition, values_equal};
use crate::value::{Value, ValueKind};
use syntax::normalize_multiline_string;

impl<'a> Interpreter<'a> {
    pub(crate) fn eval_expr(&mut self, expr_id: ir::hir::ExprId) -> Result<Value, RuntimeError> {
        let expression =
            self.hir
                .arena
                .expr(expr_id)
                .cloned()
                .ok_or(RuntimeError::InvalidReference {
                    span: infra::Span::dummy(),
                    arena: "expression",
                })?;
        match expression.kind {
            ExprKind::Literal(literal) => Ok(eval_literal(literal)),
            ExprKind::Name(_) => {
                let target = self.resolution.target(expr_id).copied().ok_or(
                    RuntimeError::InvalidReference {
                        span: expression.span,
                        arena: "resolution",
                    },
                )?;
                self.eval_target(target, expression.span)
            }
            ExprKind::Unary { op, expr } => self.eval_unary(op, expr, expression.span),
            ExprKind::Binary { op, lhs, rhs } => self.eval_binary(op, lhs, rhs, expression.span),
            ExprKind::Call { callee, arguments } => {
                let arguments = arguments
                    .into_iter()
                    .map(|argument| self.eval_expr(argument.value))
                    .collect::<Result<Vec<_>, _>>()?;
                if let Some(builtin) = self.resolution.builtin_for_expr(callee) {
                    return self.eval_builtin(builtin, arguments, expression.span);
                }
                let callee = self.eval_expr(callee)?;
                let Value::Function(item_id) = callee else {
                    return Err(RuntimeError::TypeMismatch {
                        span: expression.span,
                        expected: ValueKind::Function,
                        found: callee.kind(),
                    });
                };
                self.eval_function(item_id, arguments)
            }
            ExprKind::InterpolatedString { parts } => self.eval_interpolated_string(parts),
            ExprKind::Chain { .. } | ExprKind::Index { .. } => Err(RuntimeError::Unsupported {
                span: expression.span,
                feature: "chain or index expression",
            }),
            ExprKind::Match(match_id) => self.eval_match(match_id, expression.span),
            ExprKind::Quantifier { kind, conditions } => self.eval_quantifier(kind, conditions),
            ExprKind::Error => Err(RuntimeError::Unsupported {
                span: expression.span,
                feature: "error expression",
            }),
        }
    }

    fn eval_interpolated_string(
        &mut self,
        parts: Vec<InterpolatedPart>,
    ) -> Result<Value, RuntimeError> {
        let mut raw = String::new();
        for part in parts {
            match part {
                InterpolatedPart::Text(text) => raw.push_str(&text),
                InterpolatedPart::Expr(expr) => raw.push_str(&self.eval_expr(expr)?.to_string()),
            }
        }
        Ok(Value::String(normalize_multiline_string(&raw)))
    }

    fn eval_builtin(
        &mut self,
        builtin: BuiltinKind,
        arguments: Vec<Value>,
        span: infra::Span,
    ) -> Result<Value, RuntimeError> {
        match builtin {
            BuiltinKind::Print => {
                if arguments.len() != 1 {
                    return Err(RuntimeError::ArgumentCount {
                        span,
                        expected: 1,
                        found: arguments.len(),
                    });
                }
                self.output.push(arguments[0].to_string());
                Ok(Value::Unit)
            }
        }
    }

    fn eval_target(&mut self, target: DefTarget, span: infra::Span) -> Result<Value, RuntimeError> {
        match target {
            DefTarget::Item(item_id) => self.eval_item(item_id),
            DefTarget::Param(parameter_id) => self
                .frames
                .last()
                .and_then(|frame| frame.parameters.get(&parameter_id).cloned())
                .ok_or(RuntimeError::InvalidReference {
                    span,
                    arena: "parameter",
                }),
            DefTarget::Local(local_id) => self
                .frames
                .last()
                .and_then(|frame| frame.locals.get(local_id.index()))
                .and_then(Option::clone)
                .ok_or(RuntimeError::InvalidReference {
                    span,
                    arena: "local",
                }),
        }
    }

    fn eval_unary(
        &mut self,
        op: UnaryOp,
        expr: ir::hir::ExprId,
        span: infra::Span,
    ) -> Result<Value, RuntimeError> {
        let value = self.eval_expr(expr)?;
        match (op, value) {
            (UnaryOp::Negate, Value::Integer(value)) => value
                .checked_neg()
                .map(Value::Integer)
                .ok_or(RuntimeError::Unsupported {
                    span,
                    feature: "integer overflow",
                }),
            (UnaryOp::Negate, Value::Float(value)) => Ok(Value::Float(-value)),
            (UnaryOp::Not, Value::Bool(value)) => Ok(Value::Bool(!value)),
            (UnaryOp::Negate, value) => Err(RuntimeError::TypeMismatch {
                span,
                expected: ValueKind::Integer,
                found: value.kind(),
            }),
            (UnaryOp::Not, value) => Err(RuntimeError::TypeMismatch {
                span,
                expected: ValueKind::Bool,
                found: value.kind(),
            }),
        }
    }

    fn eval_binary(
        &mut self,
        op: BinaryOp,
        lhs: ir::hir::ExprId,
        rhs: ir::hir::ExprId,
        span: infra::Span,
    ) -> Result<Value, RuntimeError> {
        let left = self.eval_expr(lhs)?;
        let right = self.eval_expr(rhs)?;
        evaluate_binary(op, left, right, span)
    }

    fn eval_match(
        &mut self,
        match_id: ir::hir::MatchId,
        span: infra::Span,
    ) -> Result<Value, RuntimeError> {
        let expression =
            self.hir
                .arena
                .match_expr(match_id)
                .cloned()
                .ok_or(RuntimeError::InvalidReference {
                    span,
                    arena: "match",
                })?;
        let scrutinee = self.eval_expr(expression.scrutinee)?;
        let mut fallback = None;
        for arm_id in expression.arms {
            let arm = self.hir.arena.match_arm(arm_id).cloned().ok_or(
                RuntimeError::InvalidReference {
                    span,
                    arena: "match arm",
                },
            )?;
            let matches = match arm.condition {
                Some(condition_id) => self.eval_condition(condition_id, &scrutinee)?,
                None => {
                    fallback = Some(arm.body);
                    false
                }
            };
            if matches {
                return self.eval_body(arm.body);
            }
        }
        match fallback {
            Some(body) => self.eval_body(body),
            None => Ok(Value::Unit),
        }
    }

    fn eval_condition(
        &mut self,
        condition_id: ir::hir::ConditionId,
        scrutinee: &Value,
    ) -> Result<bool, RuntimeError> {
        let condition = self.hir.arena.condition(condition_id).cloned().ok_or(
            RuntimeError::InvalidReference {
                span: infra::Span::dummy(),
                arena: "condition",
            },
        )?;
        match condition.kind {
            ConditionKind::Equals(expr) => Ok(values_equal(scrutinee, &self.eval_expr(expr)?)),
            ConditionKind::Predicate { op, rhs } => {
                let value = self.eval_expr(rhs)?;
                evaluate_condition(op, scrutinee, &value, condition.span)
            }
        }
    }

    fn eval_quantifier(
        &mut self,
        kind: ir::hir::QuantifierKind,
        conditions: Vec<ir::hir::ExprId>,
    ) -> Result<Value, RuntimeError> {
        let condition_count = conditions.len();
        let mut true_count = 0usize;
        for condition in conditions {
            let value = self.eval_expr(condition)?;
            if !matches!(value, Value::Bool(_)) {
                return Err(RuntimeError::TypeMismatch {
                    span: self.expr_span(condition),
                    expected: ValueKind::Bool,
                    found: value.kind(),
                });
            }
            if matches!(value, Value::Bool(true)) {
                true_count += 1;
            }
        }
        let result = match kind {
            ir::hir::QuantifierKind::All => true_count == condition_count,
            ir::hir::QuantifierKind::Any => true_count > 0,
            ir::hir::QuantifierKind::One => true_count == 1,
            ir::hir::QuantifierKind::None => true_count == 0,
        };
        Ok(Value::Bool(result))
    }

    pub(crate) fn expr_span(&self, expr_id: ir::hir::ExprId) -> infra::Span {
        self.hir
            .arena
            .expr(expr_id)
            .map(|expression| expression.span)
            .unwrap_or_else(infra::Span::dummy)
    }

    pub(crate) fn item_span(&self, item_id: HirId) -> infra::Span {
        match self.hir.arena.node(item_id) {
            Some(HirNode::Item(item)) => item.span,
            _ => infra::Span::dummy(),
        }
    }
}

fn eval_literal(literal: Literal) -> Value {
    match literal {
        Literal::Integer(value) => Value::Integer(value),
        Literal::Float(value) => Value::Float(value),
        Literal::String(value) => Value::String(value),
        Literal::Bool(value) => Value::Bool(value),
        Literal::Regex(value) => Value::Regex(value),
        Literal::Char(value) => Value::Char(value),
    }
}
