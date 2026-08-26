// compiler-rs/analysis/src/types/support.rs

use infra::{Diagnostic, DiagnosticValue, MessageId, Span};
use ir::hir::{BinaryOp, HirId, HirNode, ItemKind};

use super::TypeId;
use super::check::Checker;
use super::model::{FunctionType, TypeKind};
use crate::resolve::{BuiltinKind, DefTarget, LocalId};

impl<'a> Checker<'a> {
    pub(super) fn builtin_type(&mut self, builtin: BuiltinKind) -> TypeId {
        match builtin {
            BuiltinKind::Print => {
                let parameter = self.store.unknown();
                let return_type = self.store.unit();
                self.store.intern(TypeKind::Function(FunctionType {
                    params: vec![parameter],
                    return_type,
                }))
            }
        }
    }

    pub(super) fn target_type(&mut self, target: DefTarget) -> TypeId {
        match target {
            DefTarget::Item(item_id) => self.infer_item(item_id),
            DefTarget::Param(param_id) => self
                .collection
                .node_type(param_id)
                .unwrap_or_else(|| self.store.error()),
            DefTarget::Local(local_id) => self
                .local_types
                .get(local_id.index())
                .copied()
                .flatten()
                .unwrap_or_else(|| self.store.unknown()),
        }
    }

    pub(super) fn explicit_return_type(&self, item_id: HirId) -> Option<TypeId> {
        let HirNode::Item(item) = self.hir.arena.node(item_id)? else {
            return None;
        };
        let ItemKind::Function(function) = &item.kind else {
            return None;
        };
        function
            .return_type
            .as_ref()
            .and_then(|_| self.collection.node_type(item_id))
            .and_then(|function_type| match self.collection.kind(function_type)? {
                TypeKind::Function(function) => Some(function.return_type),
                _ => None,
            })
    }

    pub(super) fn item_kind(&self, item_id: HirId) -> Option<ItemKind> {
        match self.hir.arena.node(item_id)? {
            HirNode::Item(item) => Some(item.kind.clone()),
            _ => None,
        }
    }

    pub(super) fn item_name(&self, item_id: HirId) -> String {
        let Some(HirNode::Item(item)) = self.hir.arena.node(item_id) else {
            return String::from("<invalid>");
        };
        let name = match &item.kind {
            ItemKind::Const(constant) => &constant.name,
            ItemKind::Struct(structure) => &structure.name,
            ItemKind::Enum(enumeration) => &enumeration.name,
            ItemKind::Function(function) => &function.name,
            ItemKind::Unsupported { .. } => return String::from("<unsupported>"),
        };
        self.source
            .get(name.span.start..name.span.end)
            .unwrap_or("<invalid>")
            .to_owned()
    }

    pub(super) fn item_span(&self, item_id: HirId) -> Span {
        match self.hir.arena.node(item_id) {
            Some(HirNode::Item(item)) => item.span,
            _ => Span::dummy(),
        }
    }

    pub(super) fn expr_span(&self, id: ir::hir::ExprId) -> Span {
        self.hir
            .arena
            .expr(id)
            .map(|expression| expression.span)
            .unwrap_or_else(Span::dummy)
    }

    pub(super) fn report_invalid(&mut self, span: Span) -> TypeId {
        self.diagnostics.push(Diagnostic::error(
            "invalid-type-reference",
            MessageId::new("analysis.invalid-type-reference"),
            span,
        ));
        self.store.error()
    }

    pub(super) fn require_bool(&mut self, actual: TypeId, span: Span) {
        if !self.is_error(actual) && !self.is_bool(actual) {
            self.diagnostics.push(
                Diagnostic::error(
                    "non-boolean-condition",
                    MessageId::new("analysis.non-boolean-condition"),
                    span,
                )
                .with_arg("found", DiagnosticValue::Type(self.display_type(actual))),
            );
        }
    }

    pub(super) fn report_type_mismatch(
        &mut self,
        expected: TypeId,
        found: TypeId,
        span: Span,
    ) -> TypeId {
        if !self.is_error(found) {
            self.diagnostics.push(
                Diagnostic::error(
                    "type-mismatch",
                    MessageId::new("analysis.type-mismatch"),
                    span,
                )
                .with_arg(
                    "expected",
                    DiagnosticValue::Type(self.display_type(expected)),
                )
                .with_arg("found", DiagnosticValue::Type(self.display_type(found))),
            );
        }
        self.store.error()
    }

    pub(super) fn unify(&mut self, left: TypeId, right: TypeId, span: Span) -> TypeId {
        if left == right || self.is_error(left) || self.is_error(right) {
            return if self.is_error(left) { left } else { right };
        }
        if self.is_unknown(left) {
            return right;
        }
        if self.is_unknown(right) {
            return left;
        }
        self.report_type_mismatch(left, right, span)
    }

    pub(super) fn display_type(&self, type_id: TypeId) -> String {
        match self.store.kind(type_id) {
            Some(TypeKind::Bool) => String::from("bool"),
            Some(TypeKind::Char) => String::from("char"),
            Some(TypeKind::Integer { signed, bits }) => {
                if *signed {
                    format!("i{bits}")
                } else {
                    format!("u{bits}")
                }
            }
            Some(TypeKind::Float { bits }) => format!("f{bits}"),
            Some(TypeKind::String) => String::from("string"),
            Some(TypeKind::Regex) => String::from("regex"),
            Some(TypeKind::Unit) => String::from("void"),
            Some(TypeKind::Never) => String::from("never"),
            Some(TypeKind::Struct(_)) => String::from("struct"),
            Some(TypeKind::Enum(_)) => String::from("enum"),
            Some(TypeKind::Function(_)) => String::from("function"),
            Some(TypeKind::Unknown) => String::from("unknown"),
            Some(TypeKind::Error) | None => String::from("error"),
        }
    }

    pub(super) fn is_bool(&self, type_id: TypeId) -> bool {
        matches!(self.store.kind(type_id), Some(TypeKind::Bool))
    }

    pub(super) fn is_integer(&self, type_id: TypeId) -> bool {
        matches!(self.store.kind(type_id), Some(TypeKind::Integer { .. }))
    }

    pub(super) fn is_float(&self, type_id: TypeId) -> bool {
        matches!(self.store.kind(type_id), Some(TypeKind::Float { .. }))
    }

    pub(super) fn is_numeric(&self, type_id: TypeId) -> bool {
        self.is_integer(type_id) || self.is_float(type_id)
    }

    pub(super) fn is_unknown(&self, type_id: TypeId) -> bool {
        matches!(self.store.kind(type_id), Some(TypeKind::Unknown))
    }

    pub(super) fn is_error(&self, type_id: TypeId) -> bool {
        matches!(self.store.kind(type_id), Some(TypeKind::Error))
    }
}

/// Return whether an operator requires numeric operands.
pub(super) fn is_arithmetic(op: BinaryOp) -> bool {
    matches!(
        op,
        BinaryOp::Add
            | BinaryOp::Subtract
            | BinaryOp::Multiply
            | BinaryOp::Divide
            | BinaryOp::Remainder
    )
}

/// Return whether an operator produces a boolean comparison result.
pub(super) fn is_comparison(op: BinaryOp) -> bool {
    matches!(
        op,
        BinaryOp::Lt
            | BinaryOp::Less
            | BinaryOp::Gt
            | BinaryOp::More
            | BinaryOp::LtEq
            | BinaryOp::Least
            | BinaryOp::GtEq
            | BinaryOp::Most
            | BinaryOp::Equals
            | BinaryOp::Contains
            | BinaryOp::Matches
            | BinaryOp::Starts
            | BinaryOp::Ends
            | BinaryOp::In
    )
}

impl From<LocalId> for TypeId {
    fn from(value: LocalId) -> Self {
        TypeId(value.0)
    }
}
