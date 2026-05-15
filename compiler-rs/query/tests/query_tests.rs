// query/tests/query_tests.rs
use infra::Interner;
use query::{Database, QueryId};
use vfs::SourceMap;

#[test]
fn database_can_be_created() {
    let source_map = SourceMap::new();
    let interner = Interner::new();
    let _db = Database::new(source_map, interner, 2);
}

#[test]
fn set_and_read_input() {
    let source_map = SourceMap::new();
    let interner = Interner::new();
    let mut db = Database::new(source_map, interner, 2);

    let source_id = QueryId(0);
    db.register_input(source_id);
    db.set_input(source_id, "constant = `hello`".to_string());

    let text: String = db.query(source_id);
    assert_eq!(text, "constant = `hello`");
}

#[test]
fn derived_query_uses_input() {
    let source_map = SourceMap::new();
    let interner = Interner::new();
    let mut db = Database::new(source_map, interner, 2);

    let source_id = QueryId(0);
    let len_id = QueryId(1);

    db.register_input(source_id);
    db.register_query(len_id, {
        let source_id = source_id;
        std::rc::Rc::new(move |db: &Database, _this_id: QueryId| {
            let text: String = db.query(source_id);
            let len = text.len();
            Box::new(len)
        })
    });

    db.set_input(source_id, "hello".to_string());
    let len: usize = db.query(len_id);
    assert_eq!(len, 5);
}
