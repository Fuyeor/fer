// compiler-rs/fon/tests/query_tests.rs

use fon::{FON_PARSE_QUERY, register_queries, set_source_file};
use infra::symbol::Interner;
use query::Database;
use vfs::{FileId, SourceMap};

#[test]
fn invalidates_the_cached_parse_when_the_source_input_changes() {
    let mut source_map = SourceMap::new();
    let first_id = source_map
        .add_file("first.fer", "name = `first`\n".into())
        .expect("valid source path");
    let second_id = source_map
        .add_file("second.fer", "name = `second`\n".into())
        .expect("valid source path");
    let mut database = Database::new(source_map, Interner::new(), 2);
    register_queries(&mut database);
    set_source_file(&database, first_id);

    let first: fon::ParsedFonSource = database.query(FON_PARSE_QUERY);
    set_source_file(&database, second_id);
    let second: fon::ParsedFonSource = database.query(FON_PARSE_QUERY);

    assert_eq!(first.file_id, first_id);
    assert_eq!(second.file_id, second_id);
}

#[test]
fn registers_and_runs_the_fon_parse_query() {
    let mut source_map = SourceMap::new();
    let file_id = source_map
        .add_file("manifest.fer", "name = `Fuyeor`\n".into())
        .expect("valid source path");
    let mut database = Database::new(source_map, Interner::new(), 2);
    register_queries(&mut database);
    set_source_file(&database, file_id);

    let parsed: fon::ParsedFonSource = database.query(FON_PARSE_QUERY);

    assert_eq!(parsed.file_id, FileId(0));
    assert!(!parsed.result.has_errors());
}
