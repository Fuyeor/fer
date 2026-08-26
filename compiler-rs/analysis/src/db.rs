// compiler-rs/analysis/src/db.rs

use std::rc::Rc;

use ir::lowering::CstFile;
use query::{Database, QueryId};

use crate::resolve::{ResolutionTable, resolve_names};
use crate::types::{TypeCollection, TypeTable, analyze_with_collection, collect_types};

/// Derived query returning the read-only name-resolution table.
pub const RESOLVE_NAMES_QUERY: QueryId = QueryId(2);

/// Derived query collecting the type namespace and item signatures.
pub const COLLECT_TYPES_QUERY: QueryId = QueryId(3);

/// Derived query checking expression and function-body types.
pub const TYPE_ANALYSIS_QUERY: QueryId = QueryId(4);

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
    database.register_query_with_dependencies(
        COLLECT_TYPES_QUERY,
        Rc::new(|database, _query_id| {
            let cst: CstFile = database.input(ir::CST_INPUT_QUERY);
            let hir: ir::HirFile = database.query(ir::LOWER_HIR_QUERY);
            Box::new(collect_types(cst.source.as_ref(), &hir))
        }),
        &[ir::CST_INPUT_QUERY, ir::LOWER_HIR_QUERY],
    );
    database.register_query_with_dependencies(
        TYPE_ANALYSIS_QUERY,
        Rc::new(|database, _query_id| {
            let cst: CstFile = database.input(ir::CST_INPUT_QUERY);
            let hir: ir::HirFile = database.query(ir::LOWER_HIR_QUERY);
            let resolution: ResolutionTable = database.query(RESOLVE_NAMES_QUERY);
            let collection: TypeCollection = database.query(COLLECT_TYPES_QUERY);
            Box::new(analyze_with_collection(
                cst.source.as_ref(),
                &hir,
                &resolution,
                collection,
            ))
        }),
        &[
            ir::CST_INPUT_QUERY,
            ir::LOWER_HIR_QUERY,
            RESOLVE_NAMES_QUERY,
            COLLECT_TYPES_QUERY,
        ],
    );
}

/// Set the source-bearing CST consumed by IR and analysis queries.
pub fn set_cst_file(database: &Database, input: CstFile) {
    ir::set_cst_file(database, input);
}

/// Keep the derived query's return type visible to downstream callers.
pub type ResolveNames = ResolutionTable;

/// Keep the type-collection query result visible to downstream callers.
pub type TypeCollectionResult = TypeCollection;

/// Keep the type-analysis query result visible to downstream callers.
pub type TypeAnalysis = TypeTable;
