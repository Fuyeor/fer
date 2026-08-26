// compiler-rs/analysis/src/types/collect.rs

use std::collections::BTreeMap;

use infra::{Diagnostic, DiagnosticValue, MessageId, Span};
use ir::hir::{FieldShape, FunctionDef, HirFile, HirId, HirNode, ItemKind, TypeRef};

use super::model::{FunctionSignature, TypeCollection, TypeRefResolution, TypeStore};
use super::{TypeId, TypeKind};

/// Collect type names, explicit type references, and function signatures.
pub(crate) fn collect_types(source: &str, hir: &HirFile) -> TypeCollection {
    Collector::new(source, hir).run()
}

struct Collector<'a> {
    source: &'a str,
    hir: &'a HirFile,
    store: TypeStore,
    type_namespace: BTreeMap<String, HirId>,
    node_types: Vec<Option<TypeId>>,
    signatures: Vec<FunctionSignature>,
    type_refs: Vec<TypeRefResolution>,
    diagnostics: Vec<Diagnostic>,
}

impl<'a> Collector<'a> {
    fn new(source: &'a str, hir: &'a HirFile) -> Self {
        Self {
            source,
            hir,
            store: TypeStore::new(),
            type_namespace: BTreeMap::new(),
            node_types: vec![None; hir.arena.nodes.len()],
            signatures: Vec::new(),
            type_refs: Vec::new(),
            diagnostics: Vec::new(),
        }
    }

    fn run(mut self) -> TypeCollection {
        self.collect_type_namespace();
        self.collect_items();
        TypeCollection {
            file_id: self.hir.file_id,
            node_types: self.node_types,
            signatures: self.signatures,
            type_refs: self.type_refs,
            types: self.store.types,
            diagnostics: self.diagnostics,
        }
    }

    fn collect_type_namespace(&mut self) {
        for &item_id in &self.hir.items {
            let Some(HirNode::Item(item)) = self.hir.arena.node(item_id) else {
                self.invalid_reference(Span::dummy());
                continue;
            };
            let kind = item.kind.clone();
            let (name, type_id) = match kind {
                ItemKind::Struct(structure) => {
                    (structure.name, self.store.intern(TypeKind::Struct(item_id)))
                }
                ItemKind::Enum(enumeration) => {
                    (enumeration.name, self.store.intern(TypeKind::Enum(item_id)))
                }
                ItemKind::Const(_) | ItemKind::Function(_) | ItemKind::Unsupported { .. } => {
                    continue;
                }
            };
            let name_span = name.span;
            let Some(name) = self.name_text(&name) else {
                continue;
            };
            if self.type_namespace.contains_key(&name) {
                self.diagnostics.push(
                    Diagnostic::error(
                        "duplicate-type-definition",
                        MessageId::new("analysis.duplicate-type-definition"),
                        name_span,
                    )
                    .with_arg("name", DiagnosticValue::Identifier(name)),
                );
                continue;
            }
            self.type_namespace.insert(name, item_id);
            self.node_types[item_id.index()] = Some(type_id);
        }
    }

    fn collect_items(&mut self) {
        let items = self.hir.items.clone();
        for item_id in items {
            let Some(HirNode::Item(item)) = self.hir.arena.node(item_id) else {
                self.invalid_reference(Span::dummy());
                continue;
            };
            let kind = item.kind.clone();
            match kind {
                ItemKind::Struct(structure) => self.collect_struct(item_id, structure.fields),
                ItemKind::Enum(_) | ItemKind::Const(_) | ItemKind::Unsupported { .. } => {}
                ItemKind::Function(function) => self.collect_function(item_id, function),
            }
        }
    }

    fn collect_struct(&mut self, _item_id: HirId, fields: Vec<HirId>) {
        for field_id in fields {
            let Some(HirNode::Field(field)) = self.hir.arena.node(field_id) else {
                self.invalid_reference(Span::dummy());
                continue;
            };
            let shape = field.shape.clone();
            if let FieldShape::Typed { type_ref, .. } | FieldShape::Required { type_ref } = shape {
                let type_id = self.resolve_type_ref(field_id, &type_ref);
                self.node_types[field_id.index()] = Some(type_id);
            }
        }
    }

    fn collect_function(&mut self, item_id: HirId, function: FunctionDef) {
        let mut params = Vec::with_capacity(function.params.len());
        for parameter_id in function.params {
            let Some(HirNode::Param(parameter)) = self.hir.arena.node(parameter_id) else {
                self.invalid_reference(Span::dummy());
                params.push(self.store.error());
                continue;
            };
            let type_ref = parameter.type_ref.clone();
            let type_id = self.resolve_type_ref(parameter_id, &type_ref);
            self.node_types[parameter_id.index()] = Some(type_id);
            params.push(type_id);
        }
        let return_type = function
            .return_type
            .as_ref()
            .map(|type_ref| self.resolve_type_ref(item_id, type_ref))
            .unwrap_or_else(|| self.store.unknown());
        let function_type = self.store.intern(TypeKind::Function(super::FunctionType {
            params: params.clone(),
            return_type,
        }));
        self.node_types[item_id.index()] = Some(function_type);
        self.signatures.push(FunctionSignature {
            item: item_id,
            params,
            return_type,
            body: function.body,
        });
    }

    fn resolve_type_ref(&mut self, owner: HirId, type_ref: &TypeRef) -> TypeId {
        let span = type_ref.name.span;
        let Some(name) = self.source.get(span.start..span.end) else {
            self.invalid_reference(span);
            return self.store.error();
        };
        let type_id = if let Some(type_id) = builtin_type(name, &mut self.store) {
            type_id
        } else if let Some(item_id) = self.type_namespace.get(name).copied() {
            self.node_types[item_id.index()].unwrap_or_else(|| self.store.error())
        } else {
            self.diagnostics.push(
                Diagnostic::error(
                    "unknown-type",
                    MessageId::new("analysis.unknown-type"),
                    span,
                )
                .with_arg("name", DiagnosticValue::Identifier(name.to_owned())),
            );
            self.store.error()
        };
        self.type_refs.push(TypeRefResolution {
            owner,
            type_id,
            span,
        });
        type_id
    }

    fn name_text(&mut self, name: &ir::hir::Name) -> Option<String> {
        let Some(text) = self.source.get(name.span.start..name.span.end) else {
            self.invalid_reference(name.span);
            return None;
        };
        Some(text.to_owned())
    }

    fn invalid_reference(&mut self, span: Span) {
        self.diagnostics.push(Diagnostic::error(
            "invalid-resolution-reference",
            MessageId::new("analysis.invalid-resolution-reference"),
            span,
        ));
    }
}

/// Resolve a built-in type spelling into its canonical semantic type.
fn builtin_type(name: &str, store: &mut TypeStore) -> Option<TypeId> {
    let type_id = match name {
        "bool" => store.bool(),
        "char" => store.intern(TypeKind::Char),
        "int" => store.integer(true, 64),
        "float" => store.float(64),
        "byte" => store.integer(false, 8),
        "string" => store.intern(TypeKind::String),
        "regex" => store.intern(TypeKind::Regex),
        "void" => store.unit(),
        "never" => store.intern(TypeKind::Never),
        "i8" => store.integer(true, 8),
        "i16" => store.integer(true, 16),
        "i32" => store.integer(true, 32),
        "i64" => store.integer(true, 64),
        "i128" => store.integer(true, 128),
        "u8" => store.integer(false, 8),
        "u16" => store.integer(false, 16),
        "u32" => store.integer(false, 32),
        "u64" => store.integer(false, 64),
        "u128" => store.integer(false, 128),
        "f32" => store.float(32),
        "f64" => store.float(64),
        _ => return None,
    };
    Some(type_id)
}
