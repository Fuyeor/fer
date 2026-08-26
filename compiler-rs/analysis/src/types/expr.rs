// compiler-rs/analysis/src/types/expr.rs

use infra::{Diagnostic, DiagnosticValue, MessageId, Span};
use ir::hir::{BinaryOp, ConditionKind, ExprKind, HirId, ItemKind, Literal, Stmt, UnaryOp};

use super::TypeId;
use super::check::{Checker, ItemState};
use super::model::{FunctionType, TypeKind};

impl<'a> Checker<'a> {
    pub(super) fn infer_item(&mut self, item_id: HirId) -> TypeId {
        let index = item_id.index();
        if index >= self.item_states.len() {
            return self.report_invalid(Span::dummy());
        }
        if self.item_states[index] == ItemState::Visiting {
            if self.reported_cycles.insert(item_id) {
                self.diagnostics.push(
                    Diagnostic::error(
                        "cyclic-type-dependency",
                        MessageId::new("analysis.cyclic-type-dependency"),
                        self.item_span(item_id),
                    )
                    .with_arg("name", DiagnosticValue::Identifier(self.item_name(item_id))),
                );
            }
            return self.store.error();
        }
        if self.item_states[index] == ItemState::Done {
            return self
                .collection
                .node_type(item_id)
                .unwrap_or_else(|| self.store.error());
        }
        let Some(ItemKind::Const(constant)) = self.item_kind(item_id) else {
            return self
                .collection
                .node_type(item_id)
                .unwrap_or_else(|| self.store.error());
        };
        self.item_states[index] = ItemState::Visiting;
        let type_id = self.infer_expr(constant.value, None);
        self.collection.node_types[index] = Some(type_id);
        self.item_states[index] = ItemState::Done;
        type_id
    }

    pub(super) fn infer_expr(&mut self, id: ir::hir::ExprId, expected: Option<TypeId>) -> TypeId {
        if id.index() >= self.hir.arena.exprs.len() {
            return self.report_invalid(Span::dummy());
        }
        let Some(expression) = self.hir.arena.expr(id).cloned() else {
            return self.report_invalid(Span::dummy());
        };
        let inferred = match expression.kind {
            ExprKind::Literal(literal) => self.infer_literal(literal, expected),
            ExprKind::Name(_) => {
                let type_id = if let Some(builtin) = self.resolution.builtin_for_expr(id) {
                    self.builtin_type(builtin)
                } else {
                    let Some(target) = self.resolution.target(id).copied() else {
                        self.report_invalid(expression.span);
                        return self.store.error();
                    };
                    self.target_type(target)
                };
                expected.map_or(type_id, |expected| {
                    self.unify(type_id, expected, expression.span)
                })
            }
            ExprKind::Unary { op, expr } => self.infer_unary(op, expr, expected, expression.span),
            ExprKind::Binary { op, lhs, rhs } => {
                self.infer_binary(op, lhs, rhs, expected, expression.span)
            }
            ExprKind::Call { callee, arguments } => {
                self.infer_call(callee, arguments, expected, expression.span)
            }
            ExprKind::Chain { base, steps } => {
                self.infer_expr(base, None);
                for step in steps {
                    match step.kind {
                        ir::hir::ChainStepKind::Field { .. } => {}
                        ir::hir::ChainStepKind::Call { arguments } => {
                            for argument in arguments {
                                self.infer_expr(argument.value, None);
                            }
                        }
                        ir::hir::ChainStepKind::Index { index } => {
                            self.infer_expr(index, None);
                        }
                    }
                }
                self.store.unknown()
            }
            ExprKind::Index { base, index } => {
                self.infer_expr(base, None);
                self.infer_expr(index, None);
                self.store.unknown()
            }
            ExprKind::Match(match_id) => self.infer_match(match_id, expected, expression.span),
            ExprKind::Quantifier { conditions, .. } => {
                for condition in conditions {
                    let condition_type = self.infer_expr(condition, None);
                    self.require_bool(condition_type, self.expr_span(condition));
                }
                self.store.bool()
            }
            ExprKind::Error => self.store.error(),
        };
        self.expr_types[id.index()] = Some(inferred);
        inferred
    }

    pub(super) fn infer_literal(&mut self, literal: Literal, expected: Option<TypeId>) -> TypeId {
        match literal {
            Literal::Integer(_) => expected
                .filter(|id| self.is_integer(*id))
                .unwrap_or_else(|| self.store.integer(true, 64)),
            Literal::Float(_) => expected
                .filter(|id| self.is_float(*id))
                .unwrap_or_else(|| self.store.float(64)),
            Literal::String(_) => self.store.intern(TypeKind::String),
            Literal::Bool(_) => self.store.bool(),
            Literal::Regex(_) => self.store.intern(TypeKind::Regex),
            Literal::Char(_) => self.store.intern(TypeKind::Char),
        }
    }

    pub(super) fn infer_unary(
        &mut self,
        op: UnaryOp,
        expr: ir::hir::ExprId,
        expected: Option<TypeId>,
        span: Span,
    ) -> TypeId {
        let operand = self.infer_expr(expr, None);
        let result = match op {
            UnaryOp::Negate if self.is_numeric(operand) => operand,
            UnaryOp::Not if self.is_bool(operand) => self.store.bool(),
            UnaryOp::Negate => {
                let integer_type = self.store.integer(true, 64);
                self.report_type_mismatch(integer_type, operand, span)
            }
            UnaryOp::Not => {
                let bool_type = self.store.bool();
                self.report_type_mismatch(bool_type, operand, span)
            }
        };
        expected.map_or(result, |expected| self.unify(result, expected, span))
    }

    pub(super) fn infer_binary(
        &mut self,
        op: BinaryOp,
        lhs: ir::hir::ExprId,
        rhs: ir::hir::ExprId,
        expected: Option<TypeId>,
        span: Span,
    ) -> TypeId {
        let lhs_type = self.infer_expr(lhs, None);
        let rhs_type = self.infer_expr(
            rhs,
            if self.is_numeric(lhs_type) {
                Some(lhs_type)
            } else {
                None
            },
        );
        let result = if super::support::is_comparison(op) {
            self.unify(lhs_type, rhs_type, span);
            self.store.bool()
        } else if super::support::is_arithmetic(op) {
            if !self.is_numeric(lhs_type) {
                let integer_type = self.store.integer(true, 64);
                self.report_type_mismatch(integer_type, lhs_type, span)
            } else if !self.is_numeric(rhs_type) {
                self.report_type_mismatch(lhs_type, rhs_type, span)
            } else {
                self.unify(lhs_type, rhs_type, span)
            }
        } else {
            self.store.error()
        };
        expected.map_or(result, |expected| self.unify(result, expected, span))
    }

    pub(super) fn infer_call(
        &mut self,
        callee: ir::hir::ExprId,
        arguments: Vec<ir::hir::CallArg>,
        expected: Option<TypeId>,
        span: Span,
    ) -> TypeId {
        let callee_type = self.infer_expr(callee, None);
        let Some(TypeKind::Function(function)) = self.store.kind(callee_type).cloned() else {
            let unknown_type = self.store.unknown();
            let function_type = self.store.intern(TypeKind::Function(FunctionType {
                params: Vec::new(),
                return_type: unknown_type,
            }));
            let result = self.report_type_mismatch(function_type, callee_type, span);
            return expected.map_or(result, |expected| self.unify(result, expected, span));
        };
        if arguments.len() != function.params.len() {
            self.diagnostics.push(
                Diagnostic::error(
                    "wrong-argument-count",
                    MessageId::new("analysis.wrong-argument-count"),
                    span,
                )
                .with_arg(
                    "expected",
                    DiagnosticValue::Unsigned(function.params.len() as u128),
                )
                .with_arg("found", DiagnosticValue::Unsigned(arguments.len() as u128)),
            );
        }
        for (index, argument) in arguments.into_iter().enumerate() {
            let argument_expected = function.params.get(index).copied();
            let argument_type = self.infer_expr(argument.value, argument_expected);
            if let Some(argument_expected) = argument_expected {
                self.unify(argument_type, argument_expected, argument.span);
            }
        }
        let result = function.return_type;
        expected.map_or(result, |expected| self.unify(result, expected, span))
    }

    pub(super) fn infer_match(
        &mut self,
        match_id: ir::hir::MatchId,
        expected: Option<TypeId>,
        span: Span,
    ) -> TypeId {
        let Some(expression) = self.hir.arena.match_expr(match_id).cloned() else {
            return self.report_invalid(span);
        };
        let scrutinee = self.infer_expr(expression.scrutinee, None);
        let mut result = None;
        for arm_id in expression.arms {
            let Some(arm) = self.hir.arena.match_arm(arm_id).cloned() else {
                self.report_invalid(span);
                continue;
            };
            if let Some(condition_id) = arm.condition {
                let Some(condition) = self.hir.arena.condition(condition_id).cloned() else {
                    self.report_invalid(arm.span);
                    continue;
                };
                match condition.kind {
                    ConditionKind::Equals(value) => {
                        let value_type = self.infer_expr(value, Some(scrutinee));
                        self.unify(value_type, scrutinee, condition.span);
                    }
                    ConditionKind::Predicate { rhs, .. } => {
                        let value_type = self.infer_expr(rhs, Some(scrutinee));
                        self.unify(value_type, scrutinee, condition.span);
                    }
                }
            }
            let arm_type = self.infer_body(arm.body, None);
            result = Some(match result {
                Some(previous) => self.unify(previous, arm_type, arm.span),
                None => arm_type,
            });
        }
        let result = result.unwrap_or_else(|| self.store.unit());
        expected.map_or(result, |expected| self.unify(result, expected, span))
    }

    pub(super) fn infer_body(
        &mut self,
        body_id: ir::hir::BodyId,
        expected: Option<TypeId>,
    ) -> TypeId {
        let Some(body) = self.hir.arena.body(body_id).cloned() else {
            return self.report_invalid(Span::dummy());
        };
        let mut result = self.store.unit();
        let last = body.statements.len().checked_sub(1);
        for (index, statement) in body.statements.into_iter().enumerate() {
            let is_tail = Some(index) == last;
            match statement {
                Stmt::Expr { expr, .. } => {
                    result = self.infer_expr(expr, is_tail.then_some(expected).flatten());
                }
                Stmt::Assign { target, value, .. } => {
                    let value_type = self.infer_expr(value, None);
                    if let Some(local) = self.resolution.assignment_local(target) {
                        if let Some(local_type) = self.local_types.get_mut(local.index()) {
                            *local_type = Some(value_type);
                        } else {
                            self.report_invalid(self.expr_span(target));
                        }
                    } else {
                        self.infer_expr(target, None);
                    }
                    result = self.store.unit();
                }
                Stmt::Item(item_id) => {
                    self.infer_item(item_id);
                    result = self.store.unit();
                }
                Stmt::Error(_) => {
                    result = self.store.error();
                }
            }
        }
        expected.map_or(result, |expected| self.unify(result, expected, body.span))
    }
}
