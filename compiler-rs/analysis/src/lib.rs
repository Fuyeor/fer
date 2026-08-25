// compiler-rs/analysis/src/lib.rs

pub mod db;
pub mod resolve;

pub use db::{RESOLVE_NAMES_QUERY, ResolveNames, register_queries, set_cst_file};
pub use resolve::{DefTarget, LocalBinding, LocalId, ResolutionTable, resolve_names};
