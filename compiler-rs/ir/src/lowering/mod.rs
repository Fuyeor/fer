// ir/src/lowering/mod.rs

mod body;
mod context;
mod expr;
mod items;
mod r#match;

use infra::Span;
use syntax::cst::NodeKind;

use crate::hir::{Body, Stmt};

use crate::hir::HirFile;

pub use context::CstFile;
use context::LoweringContext;

/// Lower one owned syntax snapshot into an indexed HIR file.
pub fn lower_file(input: &CstFile) -> HirFile {
    let mut context = LoweringContext::new(input);
    let mut items = Vec::new();
    let mut module_statements = Vec::new();
    let module_span = input
        .nodes
        .get(input.root.0 as usize)
        .map(|node| node.span)
        .unwrap_or_else(Span::dummy);

    if input.nodes.get(input.root.0 as usize).is_none() {
        context.report(
            "invalid-cst-root",
            "CST root does not address a node".into(),
            Span::dummy(),
        );
    } else if let Some((NodeKind::Module, _)) = context.node_shape(input.root) {
        for child in context.child_ids(input.root) {
            let Some((kind, span)) = context.node_shape(child) else {
                items.push(context.lower_item(child));
                continue;
            };
            if let NodeKind::ExprStmt { expr } = kind {
                module_statements.push(Stmt::Expr {
                    span,
                    expr: context.lower_expr(expr),
                });
            } else {
                items.push(context.lower_item(child));
            }
        }
    } else if let Some((kind, span)) = context.node_shape(input.root) {
        context.report(
            "invalid-cst-root",
            format!("expected a Module CST root, found {kind:?}"),
            span,
        );
    }

    let module_body = context.arena.alloc_body(Body {
        span: module_span,
        statements: module_statements,
    });
    HirFile {
        file_id: input.file_id,
        items,
        module_body,
        arena: context.arena,
        diagnostics: context.diagnostics,
    }
}
