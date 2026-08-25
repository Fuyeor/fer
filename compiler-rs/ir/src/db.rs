// ir/src/db.rs

use std::rc::Rc;

use query::{Database, QueryId};

use crate::hir::HirFile;
use crate::lowering::{CstFile, lower_file};

/// Input query containing the owned CST snapshot to lower.
pub const CST_INPUT_QUERY: QueryId = QueryId(0);

/// Derived query returning the lowered HIR file.
pub const LOWER_HIR_QUERY: QueryId = QueryId(1);

/// Register the CST input and HIR lowering queries in a Fer database.
pub fn register_queries(database: &mut Database) {
    database.register_input(CST_INPUT_QUERY);
    database.register_query_with_dependencies(
        LOWER_HIR_QUERY,
        Rc::new(|database, _query_id| {
            let input: CstFile = database.input(CST_INPUT_QUERY);
            Box::new(lower_file(&input))
        }),
        &[CST_INPUT_QUERY],
    );
}

/// Set the CST snapshot consumed by the HIR lowering query.
pub fn set_cst_file(database: &Database, input: CstFile) {
    database.set_input(CST_INPUT_QUERY, input);
}

/// Keep the derived query's return type visible to downstream callers.
pub type LowerHir = HirFile;
