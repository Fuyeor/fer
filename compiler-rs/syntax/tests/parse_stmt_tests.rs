// syntax/tests/parse_stmt_tests.rs
use infra::{DiagnosticBag, Interner};
use syntax::cst::{CstNode, NodeKind};
use syntax::{Lexer, Parser};

fn parse_stmt(source: &str) -> Vec<CstNode> {
    let mut interner = Interner::new();
    let mut lexer = Lexer::new(source, &mut interner);
    let mut nodes = Vec::new();
    let mut diag = DiagnosticBag::new();
    let mut parser = Parser::new(lexer, &mut nodes, &mut diag, vfs::FileId(0));
    let _ = parser.parse_stmt();
    nodes
}

#[test]
fn parse_simple_assignment() {
    let source = "x = 42";
    let mut interner = Interner::new();
    let mut lexer = Lexer::new(source, &mut interner);
    let mut nodes = Vec::new();
    let mut diag = DiagnosticBag::new();
    let mut parser = Parser::new(lexer, &mut nodes, &mut diag, vfs::FileId(0));
    let result = parser.parse_stmt();
    assert!(result.is_ok());
    // Expect an AssignStmt node
    let assign = nodes
        .iter()
        .find(|n| matches!(n.kind, NodeKind::AssignStmt { .. }))
        .expect("AssignStmt not found");
    if let NodeKind::AssignStmt { target, value } = &assign.kind {
        // target is Identifier, value is LitInteger
        assert!(matches!(nodes[target.0 as usize].kind, NodeKind::Ident(_)));
        assert!(matches!(nodes[value.0 as usize].kind, NodeKind::LitInteger));
    } else {
        panic!("Expected AssignStmt");
    }
}

#[test]
fn parse_expression_statement() {
    let source = "print(`hi`)";
    let mut interner = Interner::new();
    let mut lexer = Lexer::new(source, &mut interner);
    let mut nodes = Vec::new();
    let mut diag = DiagnosticBag::new();
    let mut parser = Parser::new(lexer, &mut nodes, &mut diag, vfs::FileId(0));
    let result = parser.parse_stmt();
    assert!(result.is_ok());
    // Should contain a Call node
    assert!(
        nodes
            .iter()
            .any(|n| matches!(n.kind, NodeKind::Call { .. }))
    );
}

#[test]
fn parse_struct_definition() {
    let source = "Candidate = struct { id = i32 nickname = string }";
    let mut interner = Interner::new();
    let mut lexer = Lexer::new(source, &mut interner);
    let mut nodes = Vec::new();
    let mut diag = DiagnosticBag::new();
    let mut parser = Parser::new(lexer, &mut nodes, &mut diag, vfs::FileId(0));
    let result = parser.parse_declaration();
    assert!(result.is_ok());
    let struct_node = nodes
        .iter()
        .find(|n| matches!(n.kind, NodeKind::StructDef { .. }))
        .expect("StructDef not found");
    if let NodeKind::StructDef { fields, .. } = &struct_node.kind {
        assert_eq!(fields.len(), 2);
        // Check that field nodes are FieldDef
        for &f in fields {
            assert!(matches!(
                nodes[f.0 as usize].kind,
                NodeKind::FieldDef { .. }
            ));
        }
    }
}

#[test]
fn parse_enum_definition() {
    let source = "Status = enum { nice pass failed }";
    let mut interner = Interner::new();
    let mut lexer = Lexer::new(source, &mut interner);
    let mut nodes = Vec::new();
    let mut diag = DiagnosticBag::new();
    let mut parser = Parser::new(lexer, &mut nodes, &mut diag, vfs::FileId(0));
    let result = parser.parse_declaration();
    assert!(result.is_ok());
    let enum_node = nodes
        .iter()
        .find(|n| matches!(n.kind, NodeKind::EnumDef { .. }))
        .expect("EnumDef not found");
    if let NodeKind::EnumDef { variants, .. } = &enum_node.kind {
        assert_eq!(variants.len(), 3);
    }
}

#[test]
fn parse_function_with_params() {
    let source = "add(x: i32, y: i32) -> i32 { x + y }";
    let mut interner = Interner::new();
    let mut lexer = Lexer::new(source, &mut interner);
    let mut nodes = Vec::new();
    let mut diag = DiagnosticBag::new();
    let mut parser = Parser::new(lexer, &mut nodes, &mut diag, vfs::FileId(0));
    let result = parser.parse_declaration();
    assert!(result.is_ok());

    let func_node = nodes
        .iter()
        .find(|n| matches!(n.kind, NodeKind::FunctionDef { .. }))
        .expect("FunctionDef not found");
    if let NodeKind::FunctionDef { params, .. } = &func_node.kind {
        assert_eq!(params.len(), 2);
        for &param_id in params {
            let param = &nodes[param_id.0 as usize];
            assert!(matches!(param.kind, NodeKind::Param { .. }));
        }
    } else {
        panic!("Expected FunctionDef");
    }
}
