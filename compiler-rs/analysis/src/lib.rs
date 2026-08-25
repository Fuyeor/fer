// compiler-rs/analysis/src/lib.rs

pub mod db;
pub mod resolve;
pub mod types;

pub use db::{
    COLLECT_TYPES_QUERY, RESOLVE_NAMES_QUERY, ResolveNames, TYPE_ANALYSIS_QUERY, TypeAnalysis,
    TypeCollectionResult, register_queries, set_cst_file,
};
pub use resolve::{DefTarget, LocalBinding, LocalId, ResolutionTable, resolve_names};
pub use types::{
    FunctionSignature, FunctionType, TypeCollection, TypeId, TypeKind, TypeRefResolution,
    TypeTable, analyze_types, collect_types,
};
