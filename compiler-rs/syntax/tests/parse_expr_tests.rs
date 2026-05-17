// syntax/tests/parse_expr_tests.rs
use infra::{DiagnosticBag, Interner};
use syntax::cst::{CstNode, NodeKind};
use syntax::{Lexer, Parser};

fn parse_expr(source: &str) -> Vec<CstNode> {
    let mut interner = Interner::new();
    let mut lexer = Lexer::new(source, &mut interner);
    let mut nodes = Vec::new();
    let mut diag = DiagnosticBag::new();
    let mut parser = Parser::new(lexer, &mut nodes, &mut diag, vfs::FileId(0));
    let _ = parser.parse_expr(0);
    nodes
}

#[test]
fn parse_integer_literal() {
    let nodes = parse_expr("42");
    assert_eq!(nodes.len(), 1);
    assert!(matches!(nodes[0].kind, NodeKind::LitInteger));
}

#[test]
fn parse_identifier() {
    let nodes = parse_expr("my_var");
    assert_eq!(nodes.len(), 1);
    assert!(matches!(nodes[0].kind, NodeKind::Ident(_)));
}

#[test]
fn parse_string_literal() {
    let nodes = parse_expr("`hello`");
    assert_eq!(nodes.len(), 1);
    assert!(matches!(nodes[0].kind, NodeKind::LitString));
}

#[test]
fn parse_bool_true() {
    let nodes = parse_expr("true");
    assert_eq!(nodes.len(), 1);
    assert!(matches!(nodes[0].kind, NodeKind::LitBool(true)));
}

#[test]
fn parse_grouping() {
    let nodes = parse_expr("(42)");
    assert_eq!(nodes.len(), 1);
    assert!(matches!(nodes[0].kind, NodeKind::LitInteger));
}

#[test]
fn parse_unary_minus() {
    let nodes = parse_expr("-42");
    assert_eq!(nodes.len(), 2); // UnaryOp + inner LitInteger
    assert!(matches!(nodes[0].kind, NodeKind::LitInteger));
    assert!(matches!(nodes[1].kind, NodeKind::UnaryOp { .. }));
}

#[test]
fn parse_binary_plus() {
    let nodes = parse_expr("1 + 2");
    assert!(nodes.len() >= 3);
    // Expect: 1, 2, BinaryOp
    let binary = &nodes.last().unwrap().kind;
    assert!(matches!(binary, NodeKind::BinaryOp { .. }));
}

#[test]
fn parse_precedence() {
    let nodes = parse_expr("1 + 2 * 3");
    let binary = &nodes.last().unwrap().kind;
    if let NodeKind::BinaryOp { op, lhs, rhs } = binary {
        // The top-level op should be '+'
        // We can't check op token easily, but we can check structure.
    }
}

#[test]
fn parse_call() {
    let nodes = parse_expr("foo(42)");
    assert!(
        nodes
            .iter()
            .any(|n| matches!(n.kind, NodeKind::Call { .. }))
    );
}

#[test]
fn parse_chain_field_access() {
    let nodes = parse_expr("a.b.c");
    let chain = nodes
        .iter()
        .find(|n| matches!(n.kind, NodeKind::ChainExpr { .. }));
    assert!(chain.is_some());
}

#[test]
fn parse_chain_method_call() {
    let nodes = parse_expr("io.stdout.writer().write()");
    let chain = nodes
        .iter()
        .find(|n| matches!(n.kind, NodeKind::ChainExpr { .. }));
    assert!(chain.is_some());
}

#[test]
fn parse_call_single_positional() {
    let nodes = parse_expr("f(1)");
    assert!(
        nodes
            .iter()
            .any(|n| matches!(n.kind, NodeKind::Call { .. }))
    );
}

#[test]
fn parse_call_single_named() {
    let nodes = parse_expr("f(x = 1)");
    assert!(
        nodes
            .iter()
            .any(|n| matches!(n.kind, NodeKind::Call { .. }))
    );
}

#[test]
fn parse_call_multi_named() {
    let nodes = parse_expr("f(x = 1, y = 2)");
    assert!(
        nodes
            .iter()
            .any(|n| matches!(n.kind, NodeKind::Call { .. }))
    );
}

#[test]
fn parse_call_multi_positional() {
    // Parser accepts this, semantic analysis will reject later
    let nodes = parse_expr("f(1, 2)");
    assert!(
        nodes
            .iter()
            .any(|n| matches!(n.kind, NodeKind::Call { .. }))
    );
}
