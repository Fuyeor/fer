// compiler-rs/analysis/tests/resolve_scopes.rs

mod support;

use analysis::resolve::{BuiltinKind, DefTarget, resolve};
use ir::hir::{ExprKind, HirNode, ItemKind, QuantifierKind};
use support::{const_value, lower_source, name_expr_ids};

#[test]
fn resolves_top_level_forward_reference() {
    let (source, hir) = lower_source("second = first\nfirst = 1");
    let before = hir.clone();
    let table = resolve(&hir, &source);
    assert_eq!(hir, before);
    assert!(table.diagnostics.is_empty());

    let first = hir.items[1];
    let reference = name_expr_ids(&hir, &source, "first")
        .into_iter()
        .find(|id| *id != const_value(&hir, 1))
        .expect("forward reference expression");
    assert_eq!(
        table.target_for_expr(reference),
        Some(&DefTarget::Item(first))
    );
}

#[test]
fn resolves_function_parameters_and_block_constants() {
    let (source, hir) =
        lower_source("add = (value: i32) -> i32 { doubled = value + value doubled }");
    let table = resolve(&hir, &source);
    assert!(table.diagnostics.is_empty());

    let parameter = hir
        .arena
        .nodes
        .iter()
        .enumerate()
        .find_map(|(index, node)| {
            matches!(node, HirNode::Param(_)).then_some(ir::hir::HirId::new(index))
        })
        .expect("parameter node");
    let value_references = name_expr_ids(&hir, &source, "value");
    assert_eq!(value_references.len(), 2);
    for reference in value_references {
        assert_eq!(
            table.target_for_expr(reference),
            Some(&DefTarget::Param(parameter))
        );
    }

    let doubled_references = name_expr_ids(&hir, &source, "doubled");
    assert_eq!(doubled_references.len(), 2);
    assert!(
        table
            .target_for_expr(*doubled_references.last().unwrap())
            .is_some()
    );
    assert!(table.target_for_expr(doubled_references[0]).is_none());
    assert_eq!(table.locals.len(), 1);
    assert_eq!(table.locals[0].id, analysis::LocalId(0));
}

#[test]
fn rhs_is_resolved_before_first_local_binding() {
    let (source, hir) = lower_source("compute = () -> i32 { value = value + 1 }");
    let table = resolve(&hir, &source);
    assert_eq!(table.diagnostics.len(), 1);
    assert_eq!(table.diagnostics[0].code, "undefined-name");
    let rhs_start = source.rfind("value").expect("RHS name");
    assert_eq!(
        table.diagnostics[0].primary,
        infra::Span::new(rhs_start, rhs_start + 5)
    );
}

#[test]
fn duplicate_definitions_are_first_wins() {
    let (source, hir) = lower_source("value = 1\nvalue = 2\nresult = value");
    let table = resolve(&hir, &source);
    assert_eq!(table.diagnostics.len(), 1);
    assert_eq!(table.diagnostics[0].code, "duplicate-definition");

    let first = hir.items[0];
    let references = name_expr_ids(&hir, &source, "value");
    let reference = references.last().expect("result value reference");
    assert_eq!(
        table.target_for_expr(*reference),
        Some(&DefTarget::Item(first))
    );
}

#[test]
fn match_arm_scopes_allow_shadowing_without_duplicate_diagnostic() {
    let source_text =
        "outer = 1\nrun = () -> void { result = outer { `A` { outer = 2 outer } { outer } } }";
    let (source, hir) = lower_source(source_text);
    let table = resolve(&hir, &source);
    assert!(table.diagnostics.is_empty());

    let references = name_expr_ids(&hir, &source, "outer");
    assert_eq!(references.len(), 4);
    let item_targets = references
        .iter()
        .filter_map(|reference| table.target_for_expr(*reference))
        .filter(|target| matches!(target, DefTarget::Item(_)))
        .count();
    let local_targets = references
        .iter()
        .filter_map(|reference| table.target_for_expr(*reference))
        .filter(|target| matches!(target, DefTarget::Local(_)))
        .count();
    assert_eq!(item_targets, 2);
    assert_eq!(local_targets, 1);
    assert_eq!(
        references
            .iter()
            .filter(|id| table.target_for_expr(**id).is_none())
            .count(),
        1
    );
}

#[test]
fn quantifier_conditions_use_the_current_scope() {
    let (source, hir) = lower_source("limit = 10\nready = all (limit > 0, any (limit > 5))");
    let table = resolve(&hir, &source);
    assert!(table.diagnostics.is_empty());

    let item = match &hir.arena.nodes[hir.items[1].index()] {
        HirNode::Item(item) => item,
        other => panic!("expected item, got {other:?}"),
    };
    let ItemKind::Const(constant) = &item.kind else {
        panic!("expected const item");
    };
    let ExprKind::Quantifier { kind, conditions } = &hir.arena.exprs[constant.value.index()].kind
    else {
        panic!("expected quantifier expression");
    };
    assert_eq!(*kind, QuantifierKind::All);
    assert_eq!(conditions.len(), 2);
    let limit = hir.items[0];
    for expression in hir.arena.exprs.iter().filter(|expression| {
        matches!(expression.kind, ExprKind::Name(_))
            && source.get(expression.span.start..expression.span.end) == Some("limit")
    }) {
        let id = ir::hir::ExprId::new(
            hir.arena
                .exprs
                .iter()
                .position(|candidate| std::ptr::eq(candidate, expression))
                .expect("expression index"),
        );
        assert_eq!(table.target_for_expr(id), Some(&DefTarget::Item(limit)));
    }
}

#[test]
fn unresolved_name_reports_its_source_span() {
    let (source, hir) = lower_source("answer = missing");
    let table = resolve(&hir, &source);
    assert_eq!(table.diagnostics.len(), 1);
    assert_eq!(table.diagnostics[0].code, "undefined-name");
    assert_eq!(table.diagnostics[0].primary, infra::Span::new(9, 16));
}

#[test]
fn resolves_print_as_builtin_without_a_hir_target() {
    let (source, hir) = lower_source("print(`hello`)");
    let table = resolve(&hir, &source);
    assert!(table.diagnostics.is_empty());

    let print_expr = name_expr_ids(&hir, &source, "print")[0];
    assert_eq!(table.target_for_expr(print_expr), None);
    assert_eq!(table.builtin_for_expr(print_expr), Some(BuiltinKind::Print));
}

#[test]
fn user_definition_takes_precedence_over_builtin_name() {
    let (source, hir) = lower_source("print = () -> i64 { 1 }\nprint()");
    let table = resolve(&hir, &source);
    assert!(table.diagnostics.is_empty());

    let print_expr = name_expr_ids(&hir, &source, "print")[0];
    assert_eq!(table.builtin_for_expr(print_expr), None);
    assert_eq!(
        table.target_for_expr(print_expr),
        Some(&DefTarget::Item(hir.items[0]))
    );
}
