// syntax/tests/parse_expr_tests.rs
use infra::{DiagnosticBag, Interner};
use syntax::cst::{CstNode, InterpolatedPart, NodeId, NodeKind, QuantifierKind};
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
    let top_id = NodeId((nodes.len() - 1) as u32);
    let NodeKind::BinaryOp { lhs, rhs, .. } = &nodes[top_id.0 as usize].kind else {
        panic!("expected a top-level binary operation");
    };
    assert!(matches!(nodes[lhs.0 as usize].kind, NodeKind::LitInteger));
    assert!(matches!(
        nodes[rhs.0 as usize].kind,
        NodeKind::BinaryOp { .. }
    ));
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
fn parse_quantifier_all_with_mixed_separators() {
    let nodes = parse_expr("all (x > 1, y equals 2\n  z contains `ok`)");
    let quantifier = nodes
        .iter()
        .find_map(|node| match &node.kind {
            NodeKind::Quantifier { kind, conditions } => Some((kind, conditions)),
            _ => None,
        })
        .expect("all quantifier not found");
    assert_eq!(*quantifier.0, QuantifierKind::All);
    assert_eq!(quantifier.1.len(), 3);
    assert!(
        quantifier
            .1
            .iter()
            .all(|id| matches!(nodes[id.0 as usize].kind, NodeKind::BinaryOp { .. }))
    );
}

#[test]
fn parse_nested_quantifiers() {
    let nodes = parse_expr("any (all (x > 1, y > 2), none (z equals 0))");
    let quantifiers = nodes
        .iter()
        .filter_map(|node| match node.kind {
            NodeKind::Quantifier {
                ref kind,
                ref conditions,
            } => Some((kind, conditions)),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(quantifiers.len(), 3);
    assert!(
        quantifiers
            .iter()
            .any(|(kind, _)| **kind == QuantifierKind::Any)
    );
    assert!(
        quantifiers
            .iter()
            .any(|(kind, _)| **kind == QuantifierKind::All)
    );
    assert!(
        quantifiers
            .iter()
            .any(|(kind, _)| **kind == QuantifierKind::None)
    );
}

#[test]
fn reject_quantifier_conditions_separated_by_only_spaces() {
    let mut interner = Interner::new();
    let lexer = Lexer::new("all (x > 1 y > 2)", &mut interner);
    let mut nodes = Vec::new();
    let mut diagnostics = DiagnosticBag::new();
    let mut parser = Parser::new(lexer, &mut nodes, &mut diagnostics, vfs::FileId(0));
    assert!(parser.parse_expr(0).is_err());
}

#[test]
fn parse_all_quantifier_kinds() {
    for (source, expected) in [
        ("all (x > 1)", QuantifierKind::All),
        ("any (x > 1)", QuantifierKind::Any),
        ("one (x > 1)", QuantifierKind::One),
        ("none (x > 1)", QuantifierKind::None),
    ] {
        let nodes = parse_expr(source);
        assert!(nodes.iter().any(|node| matches!(
            node.kind,
            NodeKind::Quantifier { kind, .. } if kind == expected
        )));
    }
}

#[test]
fn parse_condition_not() {
    let nodes = parse_expr("not(x > 1)");
    let _unary = nodes
        .iter()
        .find(|n| matches!(n.kind, NodeKind::UnaryOp { .. }))
        .expect("UnaryOp not found");
}

#[test]
fn parse_interpolated_string_with_name_and_expression() {
    let nodes = parse_expr("`Hello, {name}! {1 + 1}`");
    let interpolated = nodes
        .iter()
        .find_map(|node| {
            let NodeKind::InterpolatedString { parts } = &node.kind else {
                return None;
            };
            Some(parts)
        })
        .expect("interpolated string node");

    assert!(matches!(&interpolated[0], InterpolatedPart::Text(text) if text == "Hello, "));
    assert!(matches!(interpolated[1], InterpolatedPart::Expr(_)));
    assert!(matches!(&interpolated[2], InterpolatedPart::Text(text) if text == "! "));
    assert!(matches!(interpolated[3], InterpolatedPart::Expr(_)));
    assert!(
        nodes
            .iter()
            .any(|node| matches!(node.kind, NodeKind::BinaryOp { .. }))
    );
}
