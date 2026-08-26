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

/// Parse a declaration while preserving the parser result for negative tests.
fn parse_decl_result(source: &str) -> Result<(), syntax::parse::ParseError> {
    let mut interner = Interner::new();
    let lexer = Lexer::new(source, &mut interner);
    let mut nodes = Vec::new();
    let mut diag = DiagnosticBag::new();
    let mut parser = Parser::new(lexer, &mut nodes, &mut diag, vfs::FileId(0));
    parser.parse_declaration().map(|_| ())
}

/// Helper: find the first node of a specific kind in the tree.
fn find_node<F>(nodes: &[CstNode], predicate: F) -> Option<&CstNode>
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
    if let NodeKind::AssignStmt { target, value, .. } = &assign.kind {
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
    let nodes = parse_decl(
        "Candidate = struct { id = 0 nickname: string = `guest` legacy = i32 required: i32 }",
    );
    let struct_node = find_node(&nodes, |k| matches!(k, NodeKind::StructDef { .. }))
        .expect("StructDef not found");
    if let NodeKind::StructDef { fields, .. } = &struct_node.kind {
        assert_eq!(fields.len(), 4, "expected 4 fields");
        let first = &nodes[fields[0].0 as usize];
        let second = &nodes[fields[1].0 as usize];
        let legacy = &nodes[fields[2].0 as usize];
        let third = &nodes[fields[3].0 as usize];
        assert!(matches!(
            first.kind,
            NodeKind::FieldDef {
                type_annotation: None,
                default_value: Some(_),
                ..
            }
        ));
        assert!(matches!(
            second.kind,
            NodeKind::FieldDef {
                type_annotation: Some(_),
                default_value: Some(_),
                ..
            }
        ));
        let NodeKind::FieldDef {
            type_annotation: None,
            default_value: Some(legacy_default),
            ..
        } = &legacy.kind
        else {
            panic!("Expected a default-only legacy field");
        };
        assert!(matches!(
            nodes[legacy_default.0 as usize].kind,
            NodeKind::Ident(_)
        ));
        assert!(matches!(
            third.kind,
            NodeKind::FieldDef {
                type_annotation: Some(_),
                default_value: None,
                ..
            }
        ));
    }
}

#[test]
fn rejects_field_without_type_or_default() {
    let error = parse_decl_result("Candidate = struct { invalid }")
        .expect_err("a field without type or default must be rejected");
    assert!(error.message.contains("field type or default value"));
}

#[test]
fn parse_annotations_on_declarations_and_fields() {
    let nodes = parse_decl(
        "#[derive = `Debug`, mode = stable] Candidate = struct { #[required] id: i32 = 0 }",
    );
    let struct_node = find_node(&nodes, |kind| matches!(kind, NodeKind::StructDef { .. }))
        .expect("StructDef not found");

    let NodeKind::StructDef {
        fields,
        annotations,
        ..
    } = &struct_node.kind
    else {
        panic!("Expected StructDef");
    };
    assert_eq!(annotations.len(), 1);
    let declaration_annotation = &nodes[annotations[0].0 as usize];
    assert!(matches!(
        declaration_annotation.kind,
        NodeKind::Annotation { .. }
    ));
    assert_eq!(struct_node.children.first(), Some(&annotations[0]));
    let NodeKind::Annotation { arguments, .. } = &declaration_annotation.kind else {
        panic!("Expected Annotation");
    };
    assert_eq!(arguments.len(), 2);
    assert!(matches!(
        nodes[arguments[0].0 as usize].kind,
        NodeKind::AnnotationArg { name: None, .. }
    ));
    assert!(matches!(
        nodes[arguments[1].0 as usize].kind,
        NodeKind::AnnotationArg { name: Some(_), .. }
    ));

    let field = &nodes[fields[0].0 as usize];
    let NodeKind::FieldDef { annotations, .. } = &field.kind else {
        panic!("Expected FieldDef");
    };
    assert_eq!(annotations.len(), 1);
    assert!(matches!(
        nodes[annotations[0].0 as usize].kind,
        NodeKind::Annotation { .. }
    ));
    assert_eq!(
        field.span.start, nodes[annotations[0].0 as usize].span.start,
        "field span should include its annotation"
    );
}

#[test]
fn parse_annotations_on_other_declarations() {
    let function_nodes = parse_decl("#[inline] add = (x: i32) -> i32 { x }");
    let function = find_node(&function_nodes, |kind| {
        matches!(kind, NodeKind::FunctionDef { .. })
    })
    .expect("FunctionDef not found");
    let NodeKind::FunctionDef { annotations, .. } = &function.kind else {
        panic!("Expected FunctionDef");
    };
    assert_eq!(annotations.len(), 1);
    assert_eq!(function.children.first(), Some(&annotations[0]));

    let enum_nodes = parse_decl("#[closed] Status = enum { nice pass }");
    let enum_node = find_node(&enum_nodes, |kind| matches!(kind, NodeKind::EnumDef { .. }))
        .expect("EnumDef not found");
    let NodeKind::EnumDef { annotations, .. } = &enum_node.kind else {
        panic!("Expected EnumDef");
    };
    assert_eq!(annotations.len(), 1);
    assert_eq!(enum_node.children.first(), Some(&annotations[0]));

    let assignment_nodes = parse_decl("#[const] answer = 42");
    let assignment = find_node(&assignment_nodes, |kind| {
        matches!(kind, NodeKind::AssignStmt { .. })
    })
    .expect("AssignStmt not found");
    let NodeKind::AssignStmt { annotations, .. } = &assignment.kind else {
        panic!("Expected AssignStmt");
    };
    assert_eq!(annotations.len(), 1);
    assert_eq!(assignment.children.first(), Some(&annotations[0]));
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
    let nodes = parse_decl("add = (x: i32, y: i32) -> i32 { x + y }");
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

#[test]
fn reject_legacy_function_declaration_syntax() {
    assert!(parse_decl_result("add(x: i32) -> i32 { x }").is_err());
}

#[test]
fn parse_zero_parameter_function_with_formal_binding() {
    let nodes = parse_decl("main = () -> i64 { 42 }");
    let function = find_node(&nodes, |kind| matches!(kind, NodeKind::FunctionDef { .. }))
        .expect("FunctionDef not found");
    let NodeKind::FunctionDef { params, .. } = &function.kind else {
        panic!("Expected FunctionDef");
    };
    assert!(params.is_empty(), "expected no parameters");
}

#[test]
fn preserve_parenthesized_constant_rhs_as_assignment() {
    let nodes = parse_decl("answer = (40 + 2)");
    assert!(find_node(&nodes, |kind| matches!(kind, NodeKind::AssignStmt { .. })).is_some());
}
