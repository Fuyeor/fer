// ir/src/lowering/context.rs

use std::sync::Arc;

use infra::{Diagnostic, DiagnosticValue, MessageId, Span};
use syntax::cst::{CstNode, NodeId, NodeKind};
use vfs::FileId;

use crate::hir::{Expr, ExprKind, HirArena, HirNode};

/// An owned syntax snapshot consumed by the lowering query.
#[derive(Debug, Clone)]
pub struct CstFile {
    pub file_id: FileId,
    pub source: Arc<str>,
    pub root: NodeId,
    pub nodes: Vec<CstNode>,
}

/// Shared state and safe accessors for one lowering operation.
pub(crate) struct LoweringContext<'a> {
    pub(crate) input: &'a CstFile,
    pub(crate) arena: HirArena,
    pub(crate) diagnostics: Vec<Diagnostic>,
}

impl<'a> LoweringContext<'a> {
    pub(crate) fn new(input: &'a CstFile) -> Self {
        Self {
            input,
            arena: HirArena::default(),
            diagnostics: Vec::new(),
        }
    }

    /// Read a CST node without allowing an invalid ID to panic the lowering pass.
    pub(crate) fn node(&mut self, id: NodeId) -> Option<&CstNode> {
        if let Some(node) = self.input.nodes.get(id.0 as usize) {
            return Some(node);
        }
        self.report(
            "invalid-cst-node",
            format!("CST node {} is outside the node arena", id.0),
            Span::dummy(),
        );
        None
    }

    /// Copy the node kind and span before recursively mutating the lowering arena.
    pub(crate) fn node_shape(&mut self, id: NodeId) -> Option<(NodeKind, Span)> {
        self.node(id).map(|node| (node.kind.clone(), node.span))
    }

    /// Return the exact source text covered by a span.
    pub(crate) fn source_text(&mut self, span: Span) -> Option<String> {
        if let Some(source) = self.input.source.get(span.start..span.end) {
            return Some(source.to_owned());
        }
        self.report(
            "invalid-source-span",
            format!(
                "source span {}..{} is outside the input",
                span.start, span.end
            ),
            span,
        );
        None
    }

    /// Clone child IDs before recursively lowering their CST nodes.
    pub(crate) fn child_ids(&mut self, id: NodeId) -> Vec<NodeId> {
        self.node(id)
            .map(|node| node.children.clone())
            .unwrap_or_default()
    }

    /// Record a lowering diagnostic with a stable kebab-case code.
    pub(crate) fn report(&mut self, code: &'static str, message: String, span: Span) {
        self.diagnostics.push(
            Diagnostic::error(code, MessageId::new("ir.lowering-error"), span)
                .with_arg("message", DiagnosticValue::Text(message)),
        );
    }

    /// Allocate a placeholder node for an unsupported or malformed CST item.
    pub(crate) fn error_node(&mut self, span: Span) -> crate::hir::HirId {
        self.arena.alloc_node(HirNode::Error { span })
    }

    /// Allocate a placeholder expression while preserving the source span.
    pub(crate) fn error_expr(&mut self, span: Span) -> crate::hir::ExprId {
        self.arena.alloc_expr(Expr {
            span,
            kind: ExprKind::Error,
        })
    }
}
