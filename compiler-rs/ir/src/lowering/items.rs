// ir/src/lowering/items.rs

use infra::Span;
use syntax::cst::{NodeId, NodeKind};

use crate::hir::{
    Annotation, ConstDef, EnumDef, EnumVariant, Field, FieldShape, FunctionDef, HirNode, Item,
    ItemKind, Name, Param, StructDef, TypeRef,
};

use super::context::LoweringContext;

impl<'a> LoweringContext<'a> {
    /// Lower one module-level CST node into an indexed HIR item or error node.
    pub(crate) fn lower_item(&mut self, id: NodeId) -> crate::hir::HirId {
        let Some((kind, span)) = self.node_shape(id) else {
            return self.error_node(Span::dummy());
        };
        match kind {
            NodeKind::AssignStmt {
                annotations,
                target,
                value,
            } => self.lower_const(span, annotations, target, value),
            NodeKind::StructDef {
                annotations,
                name,
                fields,
            } => self.lower_struct(span, annotations, name, fields),
            NodeKind::EnumDef {
                annotations,
                name,
                variants,
            } => self.lower_enum(span, annotations, name, variants),
            NodeKind::FunctionDef {
                annotations,
                name,
                params,
                return_type,
                body,
            } => self.lower_function(span, annotations, name, params, return_type, body),
            NodeKind::ExprStmt { .. }
            | NodeKind::ImportDecl
            | NodeKind::ExportDecl
            | NodeKind::Module
            | NodeKind::Block { .. } => {
                self.report(
                    "unsupported-module-item",
                    format!("cannot lower {kind:?} as a module item"),
                    span,
                );
                self.error_node(span)
            }
            other => {
                self.report(
                    "invalid-module-item",
                    format!("unexpected CST node {other:?} at module level"),
                    span,
                );
                self.error_node(span)
            }
        }
    }

    /// Lower a module assignment into a constant item.
    fn lower_const(
        &mut self,
        span: Span,
        annotation_ids: Vec<NodeId>,
        target: NodeId,
        value: NodeId,
    ) -> crate::hir::HirId {
        let annotations = self.lower_annotations(&annotation_ids);
        let name = self.lower_name(target, "constant name");
        let value = self.lower_expr(value);
        self.arena.alloc_node(HirNode::Item(Item {
            span,
            annotations,
            kind: ItemKind::Const(ConstDef { name, value }),
        }))
    }

    /// Lower a struct declaration and all of its field nodes.
    fn lower_struct(
        &mut self,
        span: Span,
        annotation_ids: Vec<NodeId>,
        name: Span,
        field_ids: Vec<NodeId>,
    ) -> crate::hir::HirId {
        let annotations = self.lower_annotations(&annotation_ids);
        let fields = field_ids
            .into_iter()
            .map(|field| self.lower_field(field))
            .collect();
        self.arena.alloc_node(HirNode::Item(Item {
            span,
            annotations,
            kind: ItemKind::Struct(StructDef {
                name: Name { span: name },
                fields,
            }),
        }))
    }

    /// Lower an enum declaration and its simple variants.
    fn lower_enum(
        &mut self,
        span: Span,
        annotation_ids: Vec<NodeId>,
        name: Span,
        variant_ids: Vec<NodeId>,
    ) -> crate::hir::HirId {
        let annotations = self.lower_annotations(&annotation_ids);
        let variants = variant_ids
            .into_iter()
            .map(|variant| self.lower_variant(variant))
            .collect();
        self.arena.alloc_node(HirNode::Item(Item {
            span,
            annotations,
            kind: ItemKind::Enum(EnumDef {
                name: Name { span: name },
                variants,
            }),
        }))
    }

    /// Lower a function declaration, including its indexed body.
    fn lower_function(
        &mut self,
        span: Span,
        annotation_ids: Vec<NodeId>,
        name: Span,
        param_ids: Vec<NodeId>,
        return_type: Option<NodeId>,
        body: NodeId,
    ) -> crate::hir::HirId {
        let annotations = self.lower_annotations(&annotation_ids);
        let params = param_ids
            .into_iter()
            .map(|param| self.lower_param(param))
            .collect();
        let return_type = return_type.map(|type_id| self.lower_type(type_id));
        let body = self.lower_body(body);
        self.arena.alloc_node(HirNode::Item(Item {
            span,
            annotations,
            kind: ItemKind::Function(FunctionDef {
                name: Name { span: name },
                params,
                return_type,
                body,
            }),
        }))
    }

    /// Lower one struct field and make its three legal shapes explicit.
    pub(crate) fn lower_field(&mut self, id: NodeId) -> crate::hir::HirId {
        let Some((kind, span)) = self.node_shape(id) else {
            return self.error_node(Span::dummy());
        };
        let NodeKind::FieldDef {
            annotations,
            name,
            type_annotation,
            default_value,
        } = kind
        else {
            self.report(
                "invalid-struct-field",
                "expected a FieldDef CST node".into(),
                span,
            );
            return self.error_node(span);
        };
        let annotations = self.lower_annotations(&annotations);
        let shape = match (type_annotation, default_value) {
            (None, Some(default)) => FieldShape::Inferred {
                default: self.lower_expr(default),
            },
            (Some(type_id), Some(default)) => FieldShape::Typed {
                type_ref: self.lower_type(type_id),
                default: self.lower_expr(default),
            },
            (Some(type_id), None) => FieldShape::Required {
                type_ref: self.lower_type(type_id),
            },
            (None, None) => {
                self.report(
                    "invalid-struct-field",
                    "struct field has neither a type nor a default value".into(),
                    span,
                );
                return self.error_node(span);
            }
        };
        self.arena.alloc_node(HirNode::Field(Field {
            span,
            annotations,
            name: Name { span: name },
            shape,
        }))
    }

    /// Lower a function parameter into the shared node arena.
    fn lower_param(&mut self, id: NodeId) -> crate::hir::HirId {
        let Some((kind, span)) = self.node_shape(id) else {
            return self.error_node(Span::dummy());
        };
        let NodeKind::Param {
            name,
            type_annotation,
        } = kind
        else {
            self.report(
                "invalid-function-parameter",
                "expected a Param CST node".into(),
                span,
            );
            return self.error_node(span);
        };
        let type_ref = self.lower_type(type_annotation);
        self.arena.alloc_node(HirNode::Param(Param {
            span,
            name: Name { span: name },
            type_ref,
        }))
    }

    /// Lower a simple enum variant.
    fn lower_variant(&mut self, id: NodeId) -> crate::hir::HirId {
        let Some((kind, span)) = self.node_shape(id) else {
            return self.error_node(Span::dummy());
        };
        let NodeKind::Ident(name) = kind else {
            self.report(
                "invalid-enum-variant",
                "expected an identifier enum variant".into(),
                span,
            );
            return self.error_node(span);
        };
        self.arena.alloc_node(HirNode::EnumVariant(EnumVariant {
            span,
            name: Name { span: name },
        }))
    }

    /// Lower a type identifier while preserving an error placeholder name.
    pub(crate) fn lower_type(&mut self, id: NodeId) -> TypeRef {
        let Some((kind, span)) = self.node_shape(id) else {
            return TypeRef {
                name: Name {
                    span: Span::dummy(),
                },
            };
        };
        if !matches!(kind, NodeKind::Ident(_)) {
            self.report(
                "invalid-type-reference",
                "type annotation must be an identifier".into(),
                span,
            );
        }
        TypeRef {
            name: Name { span },
        }
    }

    /// Lower declaration or field annotations in source order.
    pub(crate) fn lower_annotations(&mut self, ids: &[NodeId]) -> Vec<crate::hir::HirId> {
        ids.iter()
            .copied()
            .map(|id| self.lower_annotation(id))
            .collect()
    }

    /// Lower one annotation and each of its expression-valued arguments.
    fn lower_annotation(&mut self, id: NodeId) -> crate::hir::HirId {
        let Some((kind, span)) = self.node_shape(id) else {
            return self.error_node(Span::dummy());
        };
        let NodeKind::Annotation { name, arguments } = kind else {
            self.report(
                "invalid-annotation",
                "expected an Annotation CST node".into(),
                span,
            );
            return self.error_node(span);
        };
        let arguments = arguments
            .into_iter()
            .map(|argument| self.lower_annotation_arg(argument))
            .collect();
        self.arena.alloc_node(HirNode::Annotation(Annotation {
            span,
            name: Name { span: name },
            arguments,
        }))
    }

    /// Lower one positional or named annotation argument.
    fn lower_annotation_arg(&mut self, id: NodeId) -> crate::hir::AnnotationArg {
        let Some((kind, span)) = self.node_shape(id) else {
            return crate::hir::AnnotationArg {
                span: Span::dummy(),
                name: None,
                value: self.error_expr(Span::dummy()),
            };
        };
        let NodeKind::AnnotationArg { name, value } = kind else {
            self.report(
                "invalid-annotation-argument",
                "expected an AnnotationArg CST node".into(),
                span,
            );
            return crate::hir::AnnotationArg {
                span,
                name: None,
                value: self.error_expr(span),
            };
        };
        crate::hir::AnnotationArg {
            span,
            name: name.map(|name| Name { span: name }),
            value: self.lower_expr(value),
        }
    }

    fn lower_name(&mut self, id: NodeId, role: &str) -> Name {
        let Some((kind, span)) = self.node_shape(id) else {
            return Name {
                span: Span::dummy(),
            };
        };
        if !matches!(kind, NodeKind::Ident(_)) {
            self.report(
                "invalid-name",
                format!("{role} must be an identifier"),
                span,
            );
        }
        Name { span }
    }
}
