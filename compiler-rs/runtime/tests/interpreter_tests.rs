// compiler-rs/runtime/tests/interpreter_tests.rs

use std::sync::Arc;

use infra::{DiagnosticBag, Interner};
use ir::lowering::{CstFile, lower_file};
use runtime::{ExecutionReport, Interpreter, RuntimeError, Value};
use syntax::{Lexer, Parser};
use vfs::FileId;

fn lower_source(source_text: &str) -> (Arc<str>, ir::HirFile) {
    let source: Arc<str> = Arc::from(source_text);
    let mut interner = Interner::new();
    let lexer = Lexer::new(source.as_ref(), &mut interner);
    let mut nodes = Vec::new();
    let mut diagnostics = DiagnosticBag::new();
    let mut parser = Parser::new(lexer, &mut nodes, &mut diagnostics, FileId(0));
    let root = parser
        .parse_file()
        .expect("runtime fixture must parse successfully");
    let hir = lower_file(&CstFile {
        file_id: FileId(0),
        source: source.clone(),
        root,
        nodes,
    });
    assert!(hir.diagnostics.is_empty(), "HIR lowering must succeed");
    (source, hir)
}

fn execute(source_text: &str) -> Result<ExecutionReport, RuntimeError> {
    let (source, hir) = lower_source(source_text);
    let resolution = analysis::resolve_names(source.as_ref(), &hir);
    assert!(
        resolution.diagnostics.is_empty(),
        "resolution must succeed: {:?}",
        resolution.diagnostics
    );
    let mut interpreter = Interpreter::new(&hir, &resolution);
    interpreter.run()
}

#[test]
fn executes_top_level_constants_and_returns_the_tail_value() {
    let result = execute("answer = 40 + 2\nanswer").expect("program should execute");
    assert_eq!(result.result, Value::Integer(42));
}

#[test]
fn invokes_a_function_with_local_immutable_bindings() {
    let result =
        execute("main() -> i32 { base = 40 base + 2 }\nmain()").expect("program should execute");
    assert_eq!(result.result, Value::Integer(42));
}

#[test]
fn evaluates_match_arms_in_source_order() {
    let result = execute("main() -> i32 { value = 2 value { 1 { 10 } 2 { 42 } { 0 } } }\nmain()")
        .expect("program should execute");
    assert_eq!(result.result, Value::Integer(42));
}

#[test]
fn evaluates_nested_quantifiers_as_boolean_values() {
    let result = execute("main() -> bool { all (true, any (false, true)) }\nmain()")
        .expect("program should execute");
    assert_eq!(result.result, Value::Bool(true));
}

#[test]
fn reports_division_by_zero_without_panicking() {
    let error = execute("main() { 1 / 0 }\nmain()").expect_err("division by zero must fail");
    assert!(matches!(error, RuntimeError::DivisionByZero { .. }));
}
