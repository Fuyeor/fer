// compiler-rs/analysis/tests/support/mod.rs

use std::sync::Arc;

use infra::{DiagnosticBag, Interner};
use ir::hir::{ExprId, ExprKind, HirFile, HirNode, ItemKind};
use ir::lowering::{CstFile, lower_file};
use syntax::{Lexer, Parser};
use vfs::FileId;

pub fn parse_cst(source: &str) -> CstFile {
    let source: Arc<str> = Arc::from(source);
    let mut interner = Interner::new();
    let lexer = Lexer::new(source.as_ref(), &mut interner);
    let mut nodes = Vec::new();
    let mut diagnostics = DiagnosticBag::new();
    let mut parser = Parser::new(lexer, &mut nodes, &mut diagnostics, FileId(0));
    let root = parser
        .parse_file()
        .expect("analysis fixture must parse successfully");
    CstFile {
        file_id: FileId(0),
        source,
        root,
        nodes,
    }
}

pub fn lower_source(source: &str) -> (Arc<str>, HirFile) {
    let cst = parse_cst(source);
    let source = Arc::clone(&cst.source);
    let hir = lower_file(&cst);
    (source, hir)
}

pub fn const_value(hir: &HirFile, item_index: usize) -> ExprId {
    let item_id = hir.items[item_index];
    let HirNode::Item(item) = &hir.arena.nodes[item_id.index()] else {
        panic!("expected item node");
    };
    let ItemKind::Const(constant) = &item.kind else {
        panic!("expected const item");
    };
    constant.value
}

pub fn name_expr_ids(hir: &HirFile, source: &str, name: &str) -> Vec<ExprId> {
    hir.arena
        .exprs
        .iter()
        .enumerate()
        .filter_map(|(index, expression)| {
            let ExprKind::Name(name_node) = &expression.kind else {
                return None;
            };
            (source.get(name_node.span.start..name_node.span.end) == Some(name))
                .then_some(ExprId::new(index))
        })
        .collect()
}
