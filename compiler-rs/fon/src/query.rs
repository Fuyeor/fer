// compiler-rs/fon/src/query.rs

use crate::{ParsedFonSource, parse_source};
use query::{Database, QueryId};
use vfs::FileId;

/// Input query containing the source file to parse.
pub const FON_SOURCE_QUERY: QueryId = QueryId(0);

/// Derived query returning the parsed FON source.
pub const FON_PARSE_QUERY: QueryId = QueryId(1);

/// Register the FON source and parse queries in a Fer database.
pub fn register_queries(database: &mut Database) {
    database.register_input(FON_SOURCE_QUERY);
    database.register_query_with_dependencies(
        FON_PARSE_QUERY,
        std::rc::Rc::new(|database, _query_id| {
            let file_id: FileId = database.input(FON_SOURCE_QUERY);
            let source = database
                .source_map
                .content(file_id)
                .expect("FON source query must reference an existing source file");
            Box::new(parse_source(file_id, source))
        }),
        &[FON_SOURCE_QUERY],
    );
}

/// Set the source file consumed by the FON parse query.
pub fn set_source_file(database: &Database, file_id: FileId) {
    database.set_input(FON_SOURCE_QUERY, file_id);
}

/// Keep the query module's return type visible to downstream callers.
pub type FonParse = ParsedFonSource;
