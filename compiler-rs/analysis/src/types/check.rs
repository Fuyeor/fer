// compiler-rs/analysis/src/types/check.rs

use std::collections::HashSet;

use infra::{Diagnostic, Span};
use ir::hir::{HirFile, HirId, ItemKind};

use super::TypeId;
use super::model::{FunctionType, TypeCollection, TypeKind, TypeStore, TypeTable};
use crate::resolve::ResolutionTable;

/// Infer and check the current HIR file using collected signatures and name targets.
pub(crate) fn analyze_types(
    source: &str,
    hir: &HirFile,
    resolution: &ResolutionTable,
    collection: TypeCollection,
) -> TypeTable {
    Checker::new(source, hir, resolution, collection).run()
}

pub(super) struct Checker<'a> {
    pub(super) source: &'a str,
    pub(super) hir: &'a HirFile,
    pub(super) resolution: &'a ResolutionTable,
    pub(super) collection: TypeCollection,
    pub(super) store: TypeStore,
    pub(super) expr_types: Vec<Option<TypeId>>,
    pub(super) definition_types: Vec<Option<TypeId>>,
    pub(super) local_types: Vec<Option<TypeId>>,
    pub(super) diagnostics: Vec<Diagnostic>,
    pub(super) item_states: Vec<ItemState>,
    pub(super) reported_cycles: HashSet<HirId>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum ItemState {
    Unvisited,
    Visiting,
    Done,
}

impl<'a> Checker<'a> {
    fn new(
        source: &'a str,
        hir: &'a HirFile,
        resolution: &'a ResolutionTable,
        collection: TypeCollection,
    ) -> Self {
        let store = TypeStore {
            types: collection.types.clone(),
        };
        Self {
            source,
            hir,
            resolution,
            collection,
            store,
            expr_types: vec![None; hir.arena.exprs.len()],
            definition_types: vec![None; resolution.definitions.len()],
            local_types: vec![None; resolution.locals.len()],
            diagnostics: Vec::new(),
            item_states: vec![ItemState::Unvisited; hir.arena.nodes.len()],
            reported_cycles: HashSet::new(),
        }
    }

    fn run(mut self) -> TypeTable {
        let items = self.hir.items.clone();
        for item_id in &items {
            if matches!(self.item_kind(*item_id), Some(ItemKind::Const(_))) {
                self.infer_item(*item_id);
            }
        }
        for signature_index in 0..self.collection.signatures.len() {
            let signature = self.collection.signatures[signature_index].clone();
            let explicit_return = self.explicit_return_type(signature.item);
            let inferred = self.infer_body(signature.body, explicit_return);
            if explicit_return.is_none() {
                self.collection.signatures[signature_index].return_type = inferred;
                let function_type = self.store.intern(TypeKind::Function(FunctionType {
                    params: signature.params,
                    return_type: inferred,
                }));
                if let Some(node_type) = self.collection.node_types.get_mut(signature.item.index())
                {
                    *node_type = Some(function_type);
                } else {
                    self.report_invalid(Span::dummy());
                }
            }
        }
        self.infer_body(self.hir.module_body, None);
        for (index, definition) in self.resolution.definitions.iter().enumerate() {
            self.definition_types[index] = Some(self.target_type(definition.target));
        }
        let mut diagnostics = self.collection.diagnostics;
        diagnostics.append(&mut self.diagnostics);
        TypeTable {
            file_id: self.hir.file_id,
            expr_types: self.expr_types,
            definition_types: self.definition_types,
            local_types: self.local_types,
            signatures: self.collection.signatures,
            type_refs: self.collection.type_refs,
            types: self.store.types,
            diagnostics,
        }
    }
}
