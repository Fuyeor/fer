// ir/tests/query_tests.rs

use std::sync::Arc;

use infra::{DiagnosticBag, Interner};
use ir::{CST_INPUT_QUERY, CstFile, LOWER_HIR_QUERY, LowerHir, register_queries, set_cst_file};
use query::Database;
use syntax::{Lexer, Parser};
use vfs::{FileId, SourceMap};

fn cst_source(source: &str) -> CstFile {
    let source: Arc<str> = Arc::from(source);
    let mut interner = Interner::new();
    let lexer = Lexer::new(source.as_ref(), &mut interner);
    let mut nodes = Vec::new();
    let mut diagnostics = DiagnosticBag::new();
    let mut parser = Parser::new(lexer, &mut nodes, &mut diagnostics, FileId(0));
    let root = parser
        .parse_file()
        .expect("CST query fixture must parse successfully");
    CstFile {
        file_id: FileId(0),
        source,
        root,
        nodes,
    }
}

#[test]
fn lower_hir_query_caches_and_invalidates() {
    let mut database = Database::new(SourceMap::new(), Interner::new(), 2);
    register_queries(&mut database);
    assert_eq!(CST_INPUT_QUERY.0, 0);
    assert_eq!(LOWER_HIR_QUERY.0, 1);

    set_cst_file(&database, cst_source("answer = 42"));
    let first: LowerHir = database.query(LOWER_HIR_QUERY);
    let cached: LowerHir = database.query(LOWER_HIR_QUERY);
    assert_eq!(first, cached);
    assert_eq!(first.items.len(), 1);

    set_cst_file(&database, cst_source("answer = 43\nother = true"));
    let changed: LowerHir = database.query(LOWER_HIR_QUERY);
    assert_ne!(first, changed);
    assert_eq!(changed.items.len(), 2);
}
