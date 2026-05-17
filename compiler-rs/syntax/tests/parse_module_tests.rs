// syntax/tests/parse_module_tests.rs
use infra::{DiagnosticBag, Interner};
use syntax::cst::{CstNode, NodeKind};
use syntax::{Lexer, Parser};

fn parse_module(source: &str) -> Vec<CstNode> {
    let mut interner = Interner::new();
    let lexer = Lexer::new(source, &mut interner);
    let mut nodes = Vec::new();
    let mut diag = DiagnosticBag::new();
    let mut parser = Parser::new(lexer, &mut nodes, &mut diag, vfs::FileId(0));
    let _ = parser.parse_declaration();
    nodes
}

#[test]
fn parse_import_with_item_nodes() {
    let nodes = parse_module("{ a b = alias } = @fer/std");
    let import = nodes
        .iter()
        .find(|n| matches!(n.kind, NodeKind::ImportDecl))
        .expect("ImportDecl not found");
    let children = &nodes[import.id.0 as usize].children;
    // children[0] is source path, rest are ImportItem nodes
    let item_count = children.len() - 1;
    assert_eq!(item_count, 2, "Expected two import items");
    // Check first item
    let first_item = &nodes[children[1].0 as usize];
    assert!(matches!(first_item.kind, NodeKind::ImportItem { .. }));
    // Check second item has alias
    let second_item = &nodes[children[2].0 as usize];
    if let NodeKind::ImportItem { alias, .. } = &second_item.kind {
        assert!(alias.is_some(), "Second item should have alias");
    }
}

#[test]
fn parse_import_at_slash() {
    let nodes = parse_module("{ a b c } = @/example");
    assert!(nodes.iter().any(|n| matches!(n.kind, NodeKind::ImportDecl)));
}

#[test]
fn parse_import_dot_slash() {
    let nodes = parse_module("{ a b c } = ./example");
    assert!(nodes.iter().any(|n| matches!(n.kind, NodeKind::ImportDecl)));
}

#[test]
fn parse_export() {
    let nodes = parse_module("exports { io fs }");
    let _export = nodes
        .iter()
        .find(|n| matches!(n.kind, NodeKind::ExportDecl))
        .expect("ExportDecl not found");
}
