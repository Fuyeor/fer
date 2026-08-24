// ir/src/lowering/mod.rs

mod body;
mod context;
mod expr;
mod items;
mod r#match;

use infra::Span;
use syntax::cst::NodeKind;

use crate::hir::HirFile;

pub use context::CstFile;
use context::LoweringContext;

/// Lower one owned syntax snapshot into an indexed HIR file.
pub fn lower_file(input: &CstFile) -> HirFile {
    let mut context = LoweringContext::new(input);
    let items = if input.nodes.get(input.root.0 as usize).is_none() {
        context.report(
            "invalid-cst-root",
            "CST root does not address a node".into(),
            Span::dummy(),
        );
        Vec::new()
    } else {
        match context.node_shape(input.root) {
            Some((NodeKind::Module, _)) => context
                .child_ids(input.root)
                .into_iter()
                .map(|item| context.lower_item(item))
                .collect(),
            Some((kind, span)) => {
                context.report(
                    "invalid-cst-root",
                    format!("expected a Module CST root, found {kind:?}"),
                    span,
                );
                Vec::new()
            }
            None => Vec::new(),
        }
    };
    HirFile {
        file_id: input.file_id,
        items,
        arena: context.arena,
        diagnostics: context.diagnostics,
    }
}
