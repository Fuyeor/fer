// ir/tests/lowering_snapshot_tests.rs

use std::sync::Arc;

use infra::{DiagnosticBag, Interner};
use ir::hir::{ExprKind, FieldShape, HirNode, InterpolatedPart, ItemKind, QuantifierKind};
use ir::lowering::{CstFile, lower_file};
use syntax::{Lexer, Parser};
use vfs::FileId;

fn lower_source(source: &str) -> ir::hir::HirFile {
    let source: Arc<str> = Arc::from(source);
    let mut interner = Interner::new();
    let lexer = Lexer::new(source.as_ref(), &mut interner);
    let mut nodes = Vec::new();
    let mut diagnostics = DiagnosticBag::new();
    let mut parser = Parser::new(lexer, &mut nodes, &mut diagnostics, FileId(0));
    let root = parser
        .parse_file()
        .expect("CST fixture must parse successfully");
    lower_file(&CstFile {
        file_id: FileId(0),
        source,
        root,
        nodes,
    })
}

#[test]
fn lower_module_body_preserves_top_level_expression_order() {
    let hir = lower_source("answer = 40 + 2\nanswer\nanswer + 1");
    assert!(hir.diagnostics.is_empty());

    let body = hir
        .arena
        .body(hir.module_body)
        .expect("module body must be present");
    assert_eq!(body.statements.len(), 2);
    assert!(matches!(body.statements[0], ir::hir::Stmt::Expr { .. }));
    assert!(matches!(body.statements[1], ir::hir::Stmt::Expr { .. }));
}

#[test]
fn rejects_invalid_cst_root_without_panicking() {
    let hir = lower_file(&CstFile {
        file_id: FileId(0),
        source: Arc::from(""),
        root: syntax::cst::NodeId(1),
        nodes: Vec::new(),
    });
    assert!(hir.items.is_empty());
    assert_eq!(hir.diagnostics.len(), 1);
    assert_eq!(hir.diagnostics[0].code, "invalid-cst-root");
}

#[test]
fn lower_const_and_struct_field_shapes() {
    let hir = lower_source(
        "total = 42\nConfig = struct { id: i32 name = `guest` limit: i32 = 5 legacy = Type }",
    );
    assert!(hir.diagnostics.is_empty());
    assert_eq!(hir.items.len(), 2);

    let struct_item = match &hir.arena.nodes[hir.items[1].index()] {
        HirNode::Item(item) => match &item.kind {
            ItemKind::Struct(definition) => definition,
            other => panic!("expected struct item, got {other:?}"),
        },
        other => panic!("expected item node, got {other:?}"),
    };
    assert_eq!(struct_item.fields.len(), 4);
    assert!(matches!(
        &hir.arena.nodes[struct_item.fields[0].index()],
        HirNode::Field(field) if matches!(field.shape, FieldShape::Required { .. })
    ));
    assert!(matches!(
        &hir.arena.nodes[struct_item.fields[1].index()],
        HirNode::Field(field) if matches!(field.shape, FieldShape::Inferred { .. })
    ));
    assert!(matches!(
        &hir.arena.nodes[struct_item.fields[2].index()],
        HirNode::Field(field) if matches!(field.shape, FieldShape::Typed { .. })
    ));
    let legacy = match &hir.arena.nodes[struct_item.fields[3].index()] {
        HirNode::Field(field) => field,
        other => panic!("expected field node, got {other:?}"),
    };
    let FieldShape::Inferred { default } = &legacy.shape else {
        panic!("expected an inferred legacy field");
    };
    assert!(matches!(
        hir.arena.exprs[default.index()].kind,
        ExprKind::Name(_)
    ));
    insta::assert_debug_snapshot!(hir);
}

#[test]
fn lower_quantifier_expression() {
    let hir = lower_source("ready = all (x > 1, y equals 2)");
    assert!(hir.diagnostics.is_empty());
    let item = match &hir.arena.nodes[hir.items[0].index()] {
        HirNode::Item(item) => item,
        other => panic!("expected item node, got {other:?}"),
    };
    let ItemKind::Const(constant) = &item.kind else {
        panic!("expected constant item");
    };
    let expression = &hir.arena.exprs[constant.value.index()];
    let ExprKind::Quantifier { kind, conditions } = &expression.kind else {
        panic!("expected quantifier expression, got {expression:?}");
    };
    assert_eq!(*kind, QuantifierKind::All);
    assert_eq!(conditions.len(), 2);
    assert!(conditions.iter().all(|condition| matches!(
        hir.arena.exprs[condition.index()].kind,
        ExprKind::Binary { .. }
    )));
    insta::assert_debug_snapshot!(hir);
}

#[test]
fn lower_all_quantifier_kinds() {
    let hir = lower_source("ready = all (x > 1, any (y > 2), one (z > 3), none (w > 4))");
    assert!(hir.diagnostics.is_empty());
    let item = match &hir.arena.nodes[hir.items[0].index()] {
        HirNode::Item(item) => item,
        other => panic!("expected item node, got {other:?}"),
    };
    let ItemKind::Const(constant) = &item.kind else {
        panic!("expected constant item");
    };
    let ExprKind::Quantifier { kind, conditions } = &hir.arena.exprs[constant.value.index()].kind
    else {
        panic!("expected outer quantifier");
    };
    assert_eq!(*kind, QuantifierKind::All);
    assert_eq!(conditions.len(), 4);
    let nested_kinds = conditions
        .iter()
        .filter_map(|condition| match &hir.arena.exprs[condition.index()].kind {
            ExprKind::Quantifier { kind, .. } => Some(*kind),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        nested_kinds,
        [
            QuantifierKind::Any,
            QuantifierKind::One,
            QuantifierKind::None
        ]
    );
    insta::assert_debug_snapshot!(hir);
}

#[test]
fn lower_annotations() {
    let hir = lower_source("#[meta = 1, mode = stable] Config = struct { #[required] id: i32 }");
    assert!(hir.diagnostics.is_empty());
    assert_eq!(hir.items.len(), 1);

    let item = match &hir.arena.nodes[hir.items[0].index()] {
        HirNode::Item(item) => item,
        other => panic!("expected item node, got {other:?}"),
    };
    assert_eq!(item.annotations.len(), 1);
    let annotation = match &hir.arena.nodes[item.annotations[0].index()] {
        HirNode::Annotation(annotation) => annotation,
        other => panic!("expected annotation node, got {other:?}"),
    };
    assert_eq!(annotation.arguments.len(), 2);
    assert!(annotation.arguments[0].name.is_none());
    assert_eq!(
        annotation.arguments[1].name.as_ref().map(|name| name.span),
        Some(infra::Span::new(12, 16))
    );
    insta::assert_debug_snapshot!(hir);
}

#[test]
fn lower_match_expression() {
    let hir = lower_source("answer = value { `A` { 1 } contains `needle` { 2 } { 0 } }");
    assert!(hir.diagnostics.is_empty());
    assert_eq!(hir.items.len(), 1);
    insta::assert_debug_snapshot!(hir);

    let item = match &hir.arena.nodes[hir.items[0].index()] {
        HirNode::Item(item) => item,
        other => panic!("expected item node, got {other:?}"),
    };
    let ItemKind::Const(constant) = &item.kind else {
        panic!("expected constant item");
    };
    let expression = &hir.arena.exprs[constant.value.index()];
    let ir::hir::ExprKind::Match(match_id) = &expression.kind else {
        panic!("expected match expression, got {expression:?}");
    };
    assert_eq!(hir.arena.matches[match_id.index()].arms.len(), 3);
    assert_eq!(hir.arena.conditions.len(), 2);
}

#[test]
fn lower_interpolated_string_parts() {
    let hir = lower_source("message = `Hello, {name}! {1 + 1}`");
    assert!(hir.diagnostics.is_empty());

    let item = match &hir.arena.nodes[hir.items[0].index()] {
        HirNode::Item(item) => item,
        other => panic!("expected item node, got {other:?}"),
    };
    let ItemKind::Const(constant) = &item.kind else {
        panic!("expected constant item");
    };
    let ExprKind::InterpolatedString { parts } = &hir.arena.exprs[constant.value.index()].kind
    else {
        panic!("expected interpolated string expression");
    };

    assert!(matches!(&parts[0], InterpolatedPart::Text(text) if text == "Hello, "));
    assert!(matches!(&parts[1], InterpolatedPart::Expr(_)));
    assert!(matches!(&parts[2], InterpolatedPart::Text(text) if text == "! "));
    let InterpolatedPart::Expr(expression) = parts[3] else {
        panic!("expected arithmetic interpolation");
    };
    assert!(matches!(
        hir.arena.exprs[expression.index()].kind,
        ExprKind::Binary { .. }
    ));
    insta::assert_debug_snapshot!(hir);
}
