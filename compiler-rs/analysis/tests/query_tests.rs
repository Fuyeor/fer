// compiler-rs/analysis/tests/query_tests.rs

pub mod support;

use analysis::db::{RESOLVE_NAMES_QUERY, register_queries, set_cst_file};
use analysis::resolve::ResolutionTable;
use query::Database;
use support::parse_cst;
use vfs::SourceMap;

fn database() -> Database {
    let mut database = Database::new(SourceMap::new(), infra::Interner::new(), 3);
    ir::register_queries(&mut database);
    register_queries(&mut database);
    database
}

#[test]
fn resolve_query_caches_and_invalidates_with_cst_input() {
    let database = database();
    set_cst_file(&database, parse_cst("answer = value\nvalue = 1"));
    let first: ResolutionTable = database.query(RESOLVE_NAMES_QUERY);
    assert!(first.diagnostics.is_empty());

    let cached: ResolutionTable = database.query(RESOLVE_NAMES_QUERY);
    assert_eq!(first, cached);

    set_cst_file(&database, parse_cst("answer = missing"));
    let updated: ResolutionTable = database.query(RESOLVE_NAMES_QUERY);
    assert_ne!(first, updated);
    assert_eq!(updated.diagnostics.len(), 1);
    assert_eq!(updated.diagnostics[0].code, "undefined-name");
}

#[test]
fn resolve_query_uses_the_cst_file_identity() {
    let database = database();
    let mut cst = parse_cst("answer = 1");
    cst.file_id = vfs::FileId(7);
    set_cst_file(&database, cst);
    let result: ResolutionTable = database.query(RESOLVE_NAMES_QUERY);
    assert_eq!(result.file_id, vfs::FileId(7));
}
