// ir/src/lib.rs

pub mod db;
pub mod hir;
pub mod lowering;

pub use db::{CST_INPUT_QUERY, LOWER_HIR_QUERY, LowerHir, register_queries, set_cst_file};
pub use hir::HirFile;
pub use lowering::{CstFile, lower_file};
