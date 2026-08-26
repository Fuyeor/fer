// syntax/tests/lossless_cst_tests.rs

use infra::Span;
use syntax::cst::NodeKind;
use syntax::{LosslessCstError, parse_lossless_cst};
use vfs::FileId;

#[test]
fn associates_nested_cst_nodes_with_contiguous_token_ranges() {
    let source = "main = () -> i64 {\n  answer = 40 + 2\n  answer\n}\n";
    let cst = parse_lossless_cst(source, FileId(0)).expect("valid source must parse");

    let function = cst
        .nodes()
        .iter()
        .find(|node| matches!(node.kind, NodeKind::FunctionDef { .. }))
        .expect("function node must exist");
    let block = cst
        .nodes()
        .iter()
        .find(|node| matches!(node.kind, NodeKind::Block { .. }))
        .expect("block node must exist");

    let function_tokens = cst.tokens_for(function.id).expect("function token range");
    let block_tokens = cst.tokens_for(block.id).expect("block token range");
    assert_eq!(
        function_tokens.first().and_then(|token| token.text(source)),
        Some("main")
    );
    assert_eq!(
        function_tokens.last().and_then(|token| token.text(source)),
        Some("}")
    );
    assert_eq!(
        block_tokens.first().and_then(|token| token.text(source)),
        Some("{")
    );
    assert_eq!(
        block_tokens.last().and_then(|token| token.text(source)),
        Some("}")
    );
    assert!(block_tokens.len() < function_tokens.len());
}

#[test]
fn node_token_ranges_retain_comment_trivia_and_original_spelling() {
    let source = "main = () -> i64 {\n  // keep `raw`\n  answer=40+2\n}\n";
    let cst = parse_lossless_cst(source, FileId(0)).expect("valid source must parse");
    let assignment = cst
        .nodes()
        .iter()
        .find(|node| matches!(node.kind, NodeKind::AssignStmt { .. }))
        .expect("assignment node must exist");
    let tokens = cst
        .tokens_for(assignment.id)
        .expect("assignment token range");

    assert_eq!(
        tokens.first().and_then(|token| token.text(source)),
        Some("answer")
    );
    assert_eq!(
        tokens.last().and_then(|token| token.text(source)),
        Some("2")
    );
    assert_eq!(tokens[0].trivia(source), Some("\n  // keep `raw`\n  "));
    assert_eq!(cst.source(), source);
}

#[test]
fn rejects_lexical_error_tokens_before_parser_association() {
    let error = syntax::parse_lossless_cst("main = () -> i64 { € }", FileId(0))
        .expect_err("lexical error token must prevent CST construction");
    assert!(matches!(error, LosslessCstError::InvalidToken { .. }));
}

#[test]
fn rejects_non_contiguous_node_ids_before_association() {
    let source = "main = () -> i64 { 42 }";
    let tokens = syntax::LosslessTokenStream::from_source(source).expect("source must lex");
    let nodes = vec![syntax::CstNode {
        id: syntax::NodeId(1),
        kind: NodeKind::Error,
        span: Span::new(0, source.len()),
        children: Vec::new(),
    }];

    assert!(matches!(
        syntax::associate_lossless_tokens(&nodes, &tokens),
        Err(LosslessCstError::InvalidNodeId { .. })
    ));
}

#[test]
fn rejects_invalid_cst_spans_before_association() {
    let source = "main = () -> i64 { 42 }";
    let tokens = syntax::LosslessTokenStream::from_source(source).expect("source must lex");
    let nodes = vec![syntax::CstNode {
        id: syntax::NodeId(0),
        kind: NodeKind::Error,
        span: Span::new(0, source.len() + 1),
        children: Vec::new(),
    }];

    assert!(matches!(
        syntax::associate_lossless_tokens(&nodes, &tokens),
        Err(LosslessCstError::InvalidNodeSpan { .. })
    ));
}
