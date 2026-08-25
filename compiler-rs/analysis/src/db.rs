// compiler-rs/analysis/src/db.rs

use std::rc::Rc;

use ir::lowering::CstFile;
use query::{Database, QueryId};

use crate::resolve::{ResolutionTable, resolve_names};

/// Derived query returning the read-only name-resolution table.
pub const RESOLVE_NAMES_QUERY: QueryId = QueryId(2);

/// Register analysis queries after the IR queries have been registered.
pub fn register_queries(database: &mut Database) {
    database.register_query_with_dependencies(
        RESOLVE_NAMES_QUERY,
        Rc::new(|database, _query_id| {
            let cst: CstFile = database.input(ir::CST_INPUT_QUERY);
            let hir: ir::HirFile = database.query(ir::LOWER_HIR_QUERY);
            Box::new(resolve_names(cst.source.as_ref(), &hir))
        }),
        &[ir::CST_INPUT_QUERY, ir::LOWER_HIR_QUERY],
    );
}

/// Set the source-bearing CST consumed by IR and analysis queries.
pub fn set_cst_file(database: &Database, input: CstFile) {
    ir::set_cst_file(database, input);
}

/// Keep the derived query's return type visible to downstream callers.
pub type ResolveNames = ResolutionTable;
