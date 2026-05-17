// syntax/tests/parse_stmt_tests.rs
use infra::{DiagnosticBag, Interner};
use syntax::cst::{CstNode, NodeKind};
use syntax::{Lexer, Parser};

/// Parse a single statement/declaration and return the produced CST nodes.
fn parse_stmt(source: &str) -> Vec<CstNode> {
    let mut interner = Interner::new();
    let lexer = Lexer::new(source, &mut interner);
    let mut nodes = Vec::new();
    let mut diag = DiagnosticBag::new();
    let mut parser = Parser::new(lexer, &mut nodes, &mut diag, vfs::FileId(0));
    let _ = parser.parse_stmt();
    nodes
}

/// Parse a declaration (function, struct, enum, const) and return the nodes.
fn parse_decl(source: &str) -> Vec<CstNode> {
    let mut interner = Interner::new();
    let lexer = Lexer::new(source, &mut interner);
    let mut nodes = Vec::new();
    let mut diag = DiagnosticBag::new();
    let mut parser = Parser::new(lexer, &mut nodes, &mut diag, vfs::FileId(0));
    let _ = parser.parse_declaration();
    nodes
}

/// Helper: find the first node of a specific kind in the tree.
fn find_node<'a, F>(nodes: &'a [CstNode], predicate: F) -> Option<&'a CstNode>
where
    F: Fn(&NodeKind) -> bool,
{
    nodes.iter().find(|n| predicate(&n.kind))
}

#[test]
fn parse_simple_assignment() {
    let nodes = parse_stmt("x = 42");
    let assign = find_node(&nodes, |k| matches!(k, NodeKind::AssignStmt { .. }))
        .expect("AssignStmt not found");
    if let NodeKind::AssignStmt { target, value } = &assign.kind {
        assert!(
            matches!(nodes[target.0 as usize].kind, NodeKind::Ident(_)),
            "target should be Ident"
        );
        assert!(
            matches!(nodes[value.0 as usize].kind, NodeKind::LitInteger),
            "value should be integer"
        );
    } else {
        panic!("Expected AssignStmt");
    }
}

#[test]
fn parse_expression_statement() {
    let nodes = parse_stmt("print(`hi`)");
    assert!(
        nodes
            .iter()
            .any(|n| matches!(n.kind, NodeKind::Call { .. })),
        "Call node expected"
    );
}

#[test]
fn parse_struct_definition() {
    let nodes = parse_decl("Candidate = struct { id = i32 nickname = string }");
    let struct_node = find_node(&nodes, |k| matches!(k, NodeKind::StructDef { .. }))
        .expect("StructDef not found");
    if let NodeKind::StructDef { fields, .. } = &struct_node.kind {
        assert_eq!(fields.len(), 2, "expected 2 fields");
        for &f in fields {
            assert!(
                matches!(nodes[f.0 as usize].kind, NodeKind::FieldDef { .. }),
                "field should be FieldDef"
            );
        }
    }
}

#[test]
fn parse_enum_definition() {
    let nodes = parse_decl("Status = enum { nice pass failed }");
    let enum_node =
        find_node(&nodes, |k| matches!(k, NodeKind::EnumDef { .. })).expect("EnumDef not found");
    if let NodeKind::EnumDef { variants, .. } = &enum_node.kind {
        assert_eq!(variants.len(), 3, "expected 3 variants");
    }
}

#[test]
fn parse_function_with_params() {
    let nodes = parse_decl("add(x: i32, y: i32) -> i32 { x + y }");
    let func_node = find_node(&nodes, |k| matches!(k, NodeKind::FunctionDef { .. }))
        .expect("FunctionDef not found");
    if let NodeKind::FunctionDef { params, .. } = &func_node.kind {
        assert_eq!(params.len(), 2, "expected 2 params");
        for &param_id in params {
            let param = &nodes[param_id.0 as usize];
            assert!(
                matches!(param.kind, NodeKind::Param { .. }),
                "param should be Param node"
            );
        }
    } else {
        panic!("Expected FunctionDef");
    }
}
