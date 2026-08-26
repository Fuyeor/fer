// compiler-rs/analysis/src/types/mod.rs

mod check;
mod collect;
mod expr;
mod model;
mod support;

pub use model::{
    FunctionSignature, FunctionType, TypeCollection, TypeId, TypeKind, TypeRefResolution, TypeTable,
};

use ir::hir::HirFile;

use crate::resolve::ResolutionTable;

/// Collect type references and item signatures without checking function bodies.
pub fn collect_types(source: &str, hir: &HirFile) -> TypeCollection {
    collect::collect_types(source, hir)
}

/// Infer and check expressions and function bodies using name-resolution results.
pub fn analyze_types(source: &str, hir: &HirFile, resolution: &ResolutionTable) -> TypeTable {
    let collection = collect_types(source, hir);
    check::analyze_types(source, hir, resolution, collection)
}

/// Check bodies using an already collected type namespace and signatures.
pub(crate) fn analyze_with_collection(
    source: &str,
    hir: &HirFile,
    resolution: &ResolutionTable,
    collection: TypeCollection,
) -> TypeTable {
    check::analyze_types(source, hir, resolution, collection)
}
