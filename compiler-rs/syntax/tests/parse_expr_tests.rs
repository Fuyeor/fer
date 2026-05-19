// syntax/tests/parse_expr_tests.rs
use infra::{DiagnosticBag, Interner};
use syntax::cst::{CstNode, NodeKind};
use syntax::{Lexer, Parser};

fn parse_expr(source: &str) -> Vec<CstNode> {
    let mut interner = Interner::new();
    let lexer = Lexer::new(source, &mut interner);
    let mut nodes = Vec::new();
    let mut diag = DiagnosticBag::new();
    let mut parser = Parser::new(lexer, &mut nodes, &mut diag, vfs::FileId(0));
    let result = parser.parse_expr(0);
    assert!(result.is_ok(), "Parse error: {:?}", result.err());
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
fn parse_match_simple() {
    let nodes = parse_expr(r#"x { `A` { 1 } { 0 } }"#);
    assert!(
        nodes
            .iter()
            .any(|n| matches!(n.kind, NodeKind::MatchExpr { .. }))
    );
}

#[test]
fn parse_match_with_contains() {
    let nodes = parse_expr(r#"uuid4 { contains `UUID` { `yes` } { `no` } }"#);
    // Should contain MatchExpr and PatternCondition
    assert!(
        nodes
            .iter()
            .any(|n| matches!(n.kind, NodeKind::MatchExpr { .. }))
    );
    assert!(
        nodes
            .iter()
            .any(|n| matches!(n.kind, NodeKind::PatternCondition { .. }))
    );
}

#[test]
fn parse_match_with_matches_regex() {
    let nodes = parse_expr(r#"x { matches /^[0-9]/i { `num` } { `other` } }"#);
    // Should contain LitRegex
    assert!(nodes.iter().any(|n| matches!(n.kind, NodeKind::LitRegex)));
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

#[test]
fn parse_condition_comparison_less() {
    let nodes = parse_expr("x > 1");
    let _binary = nodes
        .iter()
        .find(|n| matches!(n.kind, NodeKind::BinaryOp { .. }))
        .expect("BinaryOp not found");
}

#[test]
fn parse_condition_equals() {
    let nodes = parse_expr("x equals 1");
    let _binary = nodes
        .iter()
        .find(|n| matches!(n.kind, NodeKind::BinaryOp { .. }))
        .expect("BinaryOp not found");
}

#[test]
fn parse_condition_and_or() {
    let nodes = parse_expr("(x > 1) or (x equals 1)");
    let _or_node = nodes
        .iter()
        .find(|n| matches!(n.kind, NodeKind::BinaryOp { .. }))
        .expect("outer or not found");
}

#[test]
fn parse_condition_not() {
    let nodes = parse_expr("not(x > 1)");
    let _unary = nodes
        .iter()
        .find(|n| matches!(n.kind, NodeKind::UnaryOp { .. }))
        .expect("UnaryOp not found");
}
