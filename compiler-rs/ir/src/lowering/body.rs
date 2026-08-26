// ir/src/lowering/body.rs

use infra::Span;
use syntax::cst::{NodeId, NodeKind};

use crate::hir::{Body, Stmt};

use super::context::LoweringContext;

impl<'a> LoweringContext<'a> {
    /// Lower a CST block into a body arena entry.
    pub(crate) fn lower_body(&mut self, id: NodeId) -> crate::hir::BodyId {
        let Some((kind, span)) = self.node_shape(id) else {
            return self.arena.alloc_body(Body {
                span: Span::dummy(),
                statements: Vec::new(),
            });
        };
        let NodeKind::Block { statements } = kind else {
            self.report(
                "invalid-body",
                "expected a Block CST node for a body".into(),
                span,
            );
            let error = self.error_node(span);
            return self.arena.alloc_body(Body {
                span,
                statements: vec![Stmt::Error(error)],
            });
        };
        let statements = statements
            .into_iter()
            .map(|statement| self.lower_statement(statement))
            .collect();
        self.arena.alloc_body(Body { span, statements })
    }

    /// Lower a block statement while preserving declaration versus expression semantics.
    fn lower_statement(&mut self, id: NodeId) -> Stmt {
        let Some((kind, span)) = self.node_shape(id) else {
            return Stmt::Error(self.error_node(Span::dummy()));
        };
        match kind {
            NodeKind::ExprStmt { expr } => Stmt::Expr {
                span,
                expr: self.lower_expr(expr),
            },
            NodeKind::AssignStmt {
                annotations,
                target,
                type_annotation,
                value,
            } => Stmt::Assign {
                span,
                annotations: self.lower_annotations(&annotations),
                target: self.lower_expr(target),
                type_annotation: type_annotation.map(|type_id| self.lower_type(type_id)),
                value: self.lower_expr(value),
            },
            NodeKind::StructDef { .. }
            | NodeKind::EnumDef { .. }
            | NodeKind::FunctionDef { .. } => Stmt::Item(self.lower_item(id)),
            other => {
                self.report(
                    "unsupported-body-statement",
                    format!("cannot lower {other:?} inside a body"),
                    span,
                );
                Stmt::Error(self.error_node(span))
            }
        }
    }
}
