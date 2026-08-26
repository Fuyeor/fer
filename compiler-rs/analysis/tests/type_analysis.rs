// compiler-rs/analysis/tests/type_analysis.rs

mod support;

use analysis::resolve::resolve_names;
use analysis::types::{TypeKind, TypeTable, analyze_types};
use ir::hir::{ExprId, HirFile, HirNode, ItemKind, Stmt};
use support::{const_value, lower_source, name_expr_ids};

fn analyze(source_text: &str) -> (std::sync::Arc<str>, HirFile, TypeTable) {
    let (source, hir) = lower_source(source_text);
    let resolution = resolve_names(source.as_ref(), &hir);
    let table = analyze_types(source.as_ref(), &hir, &resolution);
    (source, hir, table)
}

fn function_id(hir: &HirFile, index: usize) -> ir::hir::HirId {
    let item_id = hir.items[index];
    let HirNode::Item(item) = &hir.arena.nodes[item_id.index()] else {
        panic!("expected function item");
    };
    assert!(matches!(item.kind, ItemKind::Function(_)));
    item_id
}

fn function_body(hir: &HirFile, item_id: ir::hir::HirId) -> &ir::hir::Body {
    let HirNode::Item(item) = &hir.arena.nodes[item_id.index()] else {
        panic!("expected function item");
    };
    let ItemKind::Function(function) = &item.kind else {
        panic!("expected function kind");
    };
    hir.arena.body(function.body).expect("function body")
}

fn tail_expr(hir: &HirFile, item_id: ir::hir::HirId) -> ExprId {
    let body = function_body(hir, item_id);
    let Stmt::Expr { expr, .. } = body.statements.last().expect("tail statement") else {
        panic!("expected a tail expression");
    };
    *expr
}

fn assert_expr_kind(table: &TypeTable, expr: ExprId, expected: impl FnOnce(&TypeKind) -> bool) {
    let type_id = table.type_of(expr).expect("expression type");
    let kind = table.kind(type_id).expect("canonical type");
    assert!(expected(kind), "unexpected expression type: {kind:?}");
}

#[test]
fn unconstrained_literals_use_confirmed_default_types() {
    let (_, hir, table) = analyze("integer = 1\nfraction = 1.0\nflag = true");

    assert_expr_kind(&table, const_value(&hir, 0), |kind| {
        matches!(
            kind,
            TypeKind::Integer {
                signed: true,
                bits: 64
            }
        )
    });
    assert_expr_kind(&table, const_value(&hir, 1), |kind| {
        matches!(kind, TypeKind::Float { bits: 64 })
    });
    assert_expr_kind(&table, const_value(&hir, 2), |kind| {
        matches!(kind, TypeKind::Bool)
    });
    assert!(table.diagnostics.is_empty());
}

#[test]
fn builtin_aliases_and_system_types_have_stable_kinds() {
    let (_, hir, table) = analyze(
        "byte_value = (value: byte) -> u8 { value }\nchar_value = (value: char) -> char { value }\nempty = () -> void { }",
    );

    let byte_signature = table
        .signature(function_id(&hir, 0))
        .expect("byte function signature");
    assert!(matches!(
        table.kind(byte_signature.params[0]),
        Some(TypeKind::Integer {
            signed: false,
            bits: 8
        })
    ));
    assert_eq!(byte_signature.params[0], byte_signature.return_type);

    let char_signature = table
        .signature(function_id(&hir, 1))
        .expect("char function signature");
    assert!(matches!(
        table.kind(char_signature.return_type),
        Some(TypeKind::Char)
    ));

    let void_signature = table
        .signature(function_id(&hir, 2))
        .expect("void function signature");
    assert!(matches!(
        table.kind(void_signature.return_type),
        Some(TypeKind::Unit)
    ));
    assert!(table.diagnostics.is_empty());
}

#[test]
fn explicit_parameter_type_flows_to_omitted_function_return() {
    let (_, hir, table) = analyze("identity = (value: i32) { value }");
    let item = function_id(&hir, 0);
    let tail = tail_expr(&hir, item);

    assert_expr_kind(&table, tail, |kind| {
        matches!(
            kind,
            TypeKind::Integer {
                signed: true,
                bits: 32
            }
        )
    });
    let signature = table.signature(item).expect("function signature");
    let return_kind = table.kind(signature.return_type).expect("return type");
    assert!(matches!(
        return_kind,
        TypeKind::Integer {
            signed: true,
            bits: 32
        }
    ));
    assert!(table.diagnostics.is_empty());
}

#[test]
fn explicit_return_type_rejects_a_different_tail_type() {
    let (_, hir, table) = analyze("identity = (value: i32) -> string { value }");
    let item = function_id(&hir, 0);
    assert!(
        table
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "type-mismatch")
    );
    assert_expr_kind(&table, tail_expr(&hir, item), |kind| {
        matches!(kind, TypeKind::Error)
    });
}

#[test]
fn local_binding_type_is_published_for_later_references() {
    let (_, hir, table) = analyze("compute = () { value = 1 value }");
    let item = function_id(&hir, 0);

    assert_expr_kind(&table, tail_expr(&hir, item), |kind| {
        matches!(
            kind,
            TypeKind::Integer {
                signed: true,
                bits: 64
            }
        )
    });
    assert_eq!(
        table.local_type(analysis::LocalId(0)),
        table.type_of(tail_expr(&hir, item))
    );
    assert_eq!(
        name_expr_ids(&hir, "compute = () { value = 1 value }", "value").len(),
        2
    );
    assert!(table.diagnostics.is_empty());
}

#[test]
fn collected_function_signature_supports_forward_calls() {
    let (_, hir, table) = analyze("use = () { answer() }\nanswer = () -> i32 { 1 }");
    let use_item = function_id(&hir, 0);

    assert_expr_kind(&table, tail_expr(&hir, use_item), |kind| {
        matches!(
            kind,
            TypeKind::Integer {
                signed: true,
                bits: 32
            }
        )
    });
    assert!(table.diagnostics.is_empty());
}

#[test]
fn call_with_wrong_argument_count_has_a_specific_diagnostic() {
    let (_, _, table) = analyze("answer = (value: i32) -> i32 { value }\nuse = () { answer() }");
    assert!(
        table
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "wrong-argument-count")
    );
}

#[test]
fn quantifier_conditions_must_be_boolean_and_return_boolean() {
    let (_, hir, table) = analyze("check = (value: bool) -> bool { all (value, any (value)) }");
    let item = function_id(&hir, 0);

    assert_expr_kind(&table, tail_expr(&hir, item), |kind| {
        matches!(kind, TypeKind::Bool)
    });
    assert!(table.diagnostics.is_empty());
}

#[test]
fn match_arm_results_are_checked_and_unified() {
    let (_, hir, table) =
        analyze("choose = (value: bool) -> bool { value { true { true } false { false } } }");
    let item = function_id(&hir, 0);

    assert_expr_kind(&table, tail_expr(&hir, item), |kind| {
        matches!(kind, TypeKind::Bool)
    });
    assert!(table.diagnostics.is_empty());
}

#[test]
fn non_boolean_quantifier_condition_has_a_specific_diagnostic() {
    let (_, _, table) = analyze("check = (value: i32) -> bool { all (value) }");
    assert!(
        table
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "non-boolean-condition")
    );
}

#[test]
fn arithmetic_rejects_incompatible_operand_types() {
    let (_, _, table) = analyze("compute = () { 1 + `text` }");
    assert!(
        table
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "type-mismatch")
    );
}

#[test]
fn unknown_type_reference_is_reported_as_an_analysis_diagnostic() {
    let (_, _, table) = analyze("identity = (value: Missing) { value }");
    assert!(
        table
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "unknown-type")
    );
}

#[test]
fn builtin_print_returns_unit_for_a_string_argument() {
    let (_, hir, table) = analyze("print(`hello`)");
    let call = hir
        .arena
        .exprs
        .iter()
        .enumerate()
        .find_map(|(index, expression)| {
            matches!(expression.kind, ir::hir::ExprKind::Call { .. }).then_some(ExprId::new(index))
        })
        .expect("print call expression");

    assert_expr_kind(&table, call, |kind| matches!(kind, TypeKind::Unit));
    assert!(table.diagnostics.is_empty());
}

#[test]
fn builtin_print_reports_wrong_argument_count() {
    let (_, _, table) = analyze("print()");
    assert!(
        table
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "wrong-argument-count")
    );
}
