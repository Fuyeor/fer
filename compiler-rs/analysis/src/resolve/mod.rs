// compiler-rs/analysis/src/resolve/mod.rs

pub mod error;
pub mod scope;
pub mod table;

use infra::{Diagnostic, Span};
use ir::hir::{ConditionKind, ExprKind, FieldShape, HirFile, HirNode, ItemKind, Name, Stmt};

pub use scope::{LocalId, Scope, ScopeId};
pub use table::{BindingId, DefTarget, Definition, LocalBinding, ResolutionTable};

use self::error::{duplicate_definition, invalid_reference, undefined_name};
use self::scope::ScopeTree;
use self::table::Definition as DefinitionRecord;

/// Resolve names in a lowered HIR file without mutating the HIR.
pub fn resolve_names(source: &str, hir: &HirFile) -> ResolutionTable {
    Resolver::new(source, hir).run()
}

/// Resolve names using the concise API used by analysis integration tests.
pub fn resolve(hir: &HirFile, source: &str) -> ResolutionTable {
    resolve_names(source, hir)
}

struct Resolver<'a> {
    source: &'a str,
    hir: &'a HirFile,
    scopes: ScopeTree,
    definitions: Vec<DefinitionRecord>,
    locals: Vec<LocalBinding>,
    expr_targets: Vec<Option<DefTarget>>,
    assignment_locals: Vec<Option<LocalId>>,
    diagnostics: Vec<Diagnostic>,
    next_local: usize,
}

impl<'a> Resolver<'a> {
    fn new(source: &'a str, hir: &'a HirFile) -> Self {
        Self {
            source,
            hir,
            scopes: ScopeTree::new(),
            definitions: Vec::new(),
            locals: Vec::new(),
            expr_targets: vec![None; hir.arena.exprs.len()],
            assignment_locals: vec![None; hir.arena.exprs.len()],
            diagnostics: Vec::new(),
            next_local: 0,
        }
    }

    fn run(mut self) -> ResolutionTable {
        let module_scope = ScopeId::new(0);
        self.predeclare_module_items(module_scope);
        let items = self.hir.items.clone();
        for item_id in items {
            self.resolve_item(item_id, module_scope);
        }
        self.resolve_body(self.hir.module_body, module_scope);
        ResolutionTable::from_parts(
            self.hir.file_id,
            self.expr_targets,
            self.assignment_locals,
            self.definitions,
            self.locals,
            self.scopes.into_scopes(),
            self.diagnostics,
        )
    }

    fn predeclare_module_items(&mut self, scope: ScopeId) {
        for &item_id in &self.hir.items {
            let Some(item) = self.item(item_id) else {
                self.invalid_hir_reference(Span::dummy());
                continue;
            };
            let Some((name, target)) = item_name_and_target(&item.kind, item_id) else {
                continue;
            };
            let Some(text) = self.name_text(&name) else {
                continue;
            };
            self.define(scope, text, target, name.span);
        }
    }

    fn resolve_item(&mut self, item_id: ir::hir::HirId, parent_scope: ScopeId) {
        let Some(item) = self.item(item_id) else {
            self.invalid_hir_reference(Span::dummy());
            return;
        };
        for annotation_id in item.annotations {
            self.resolve_annotation(annotation_id, parent_scope);
        }
        match item.kind {
            ItemKind::Const(constant) => self.resolve_expr(constant.value, parent_scope),
            ItemKind::Struct(structure) => {
                for field_id in structure.fields {
                    self.resolve_field(field_id, parent_scope);
                }
            }
            ItemKind::Enum(_) | ItemKind::Unsupported { .. } => {}
            ItemKind::Function(function) => {
                let function_scope = self.scopes.create(Some(parent_scope));
                for parameter_id in function.params {
                    self.define_parameter(parameter_id, function_scope);
                }
                let body_scope = self.scopes.create(Some(function_scope));
                self.resolve_body(function.body, body_scope);
            }
        }
    }

    fn resolve_field(&mut self, field_id: ir::hir::HirId, scope: ScopeId) {
        let Some(HirNode::Field(field)) = self.hir.arena.node(field_id).cloned() else {
            self.invalid_hir_reference(Span::dummy());
            return;
        };
        for annotation_id in field.annotations {
            self.resolve_annotation(annotation_id, scope);
        }
        match field.shape {
            FieldShape::Inferred { default } | FieldShape::Typed { default, .. } => {
                self.resolve_expr(default, scope)
            }
            FieldShape::Required { .. } => {}
        }
    }

    fn define_parameter(&mut self, parameter_id: ir::hir::HirId, scope: ScopeId) {
        let Some(HirNode::Param(parameter)) = self.hir.arena.node(parameter_id).cloned() else {
            self.invalid_hir_reference(Span::dummy());
            return;
        };
        let Some(name) = self.name_text(&parameter.name) else {
            return;
        };
        self.define(
            scope,
            name,
            DefTarget::Param(parameter_id),
            parameter.name.span,
        );
    }

    fn resolve_body(&mut self, body_id: ir::hir::BodyId, scope: ScopeId) {
        let Some(body) = self.hir.arena.body(body_id).cloned() else {
            self.invalid_hir_reference(Span::dummy());
            return;
        };
        for statement in body.statements {
            match statement {
                Stmt::Expr { expr, .. } => self.resolve_expr(expr, scope),
                Stmt::Assign {
                    annotations,
                    target,
                    value,
                    ..
                } => {
                    for annotation_id in annotations {
                        self.resolve_annotation(annotation_id, scope);
                    }
                    self.resolve_expr(value, scope);
                    self.resolve_assignment_target(target, scope);
                }
                Stmt::Item(item_id) => self.resolve_nested_item(item_id, scope),
                Stmt::Error(_) => {}
            }
        }
    }

    fn resolve_assignment_target(&mut self, target: ir::hir::ExprId, scope: ScopeId) {
        let Some(expression) = self.hir.arena.expr(target).cloned() else {
            self.invalid_hir_reference(Span::dummy());
            return;
        };
        match expression.kind {
            ExprKind::Name(name) => {
                let Some(text) = self.name_text(&name) else {
                    return;
                };
                let local = LocalId::new(self.next_local);
                if self.define(scope, text.clone(), DefTarget::Local(local), name.span) {
                    self.assignment_locals[target.index()] = Some(local);
                    self.locals.push(LocalBinding {
                        id: local,
                        name: text,
                        span: name.span,
                        scope,
                    });
                    self.next_local += 1;
                }
            }
            _ => self.resolve_expr(target, scope),
        }
    }

    fn resolve_nested_item(&mut self, item_id: ir::hir::HirId, scope: ScopeId) {
        let Some(item) = self.item(item_id) else {
            self.invalid_hir_reference(Span::dummy());
            return;
        };
        if let Some((name, target)) = item_name_and_target(&item.kind, item_id)
            && let Some(text) = self.name_text(&name)
        {
            self.define(scope, text, target, name.span);
        }
        self.resolve_item(item_id, scope);
    }

    fn resolve_annotation(&mut self, annotation_id: ir::hir::HirId, scope: ScopeId) {
        let Some(HirNode::Annotation(annotation)) = self.hir.arena.node(annotation_id).cloned()
        else {
            self.invalid_hir_reference(Span::dummy());
            return;
        };
        for argument in annotation.arguments {
            self.resolve_expr(argument.value, scope);
        }
    }

    fn resolve_expr(&mut self, expr_id: ir::hir::ExprId, scope: ScopeId) {
        let Some(expression) = self.hir.arena.expr(expr_id).cloned() else {
            self.invalid_hir_reference(Span::dummy());
            return;
        };
        match expression.kind {
            ExprKind::Literal(_) | ExprKind::Error => {}
            ExprKind::Name(name) => self.resolve_name(expr_id, name, scope),
            ExprKind::Unary { expr, .. } => self.resolve_expr(expr, scope),
            ExprKind::Binary { lhs, rhs, .. } => {
                self.resolve_expr(lhs, scope);
                self.resolve_expr(rhs, scope);
            }
            ExprKind::Call { callee, arguments } => {
                self.resolve_expr(callee, scope);
                for argument in arguments {
                    self.resolve_expr(argument.value, scope);
                }
            }
            ExprKind::Chain { base, steps } => {
                self.resolve_expr(base, scope);
                for step in steps {
                    match step.kind {
                        ir::hir::ChainStepKind::Field { .. } => {}
                        ir::hir::ChainStepKind::Call { arguments } => {
                            for argument in arguments {
                                self.resolve_expr(argument.value, scope);
                            }
                        }
                        ir::hir::ChainStepKind::Index { index } => self.resolve_expr(index, scope),
                    }
                }
            }
            ExprKind::Index { base, index } => {
                self.resolve_expr(base, scope);
                self.resolve_expr(index, scope);
            }
            ExprKind::Match(match_id) => self.resolve_match(match_id, scope),
            ExprKind::Quantifier { conditions, .. } => {
                for condition in conditions {
                    self.resolve_expr(condition, scope);
                }
            }
        }
    }

    fn resolve_name(&mut self, expr_id: ir::hir::ExprId, name: Name, scope: ScopeId) {
        let Some(text) = self.name_text(&name) else {
            return;
        };
        let Some(binding) = self.scopes.lookup(scope, &text) else {
            self.diagnostics.push(undefined_name(&text, name.span));
            return;
        };
        let Some(target) = self
            .definitions
            .get(binding.index())
            .map(|definition| definition.target)
        else {
            self.invalid_hir_reference(name.span);
            return;
        };
        self.expr_targets[expr_id.index()] = Some(target);
    }

    fn resolve_match(&mut self, match_id: ir::hir::MatchId, parent_scope: ScopeId) {
        let Some(expression) = self.hir.arena.match_expr(match_id).cloned() else {
            self.invalid_hir_reference(Span::dummy());
            return;
        };
        self.resolve_expr(expression.scrutinee, parent_scope);
        for arm_id in expression.arms {
            let Some(arm) = self.hir.arena.match_arm(arm_id).cloned() else {
                self.invalid_hir_reference(Span::dummy());
                continue;
            };
            let arm_scope = self.scopes.create(Some(parent_scope));
            if let Some(condition_id) = arm.condition {
                self.resolve_condition(condition_id, arm_scope);
            }
            self.resolve_body(arm.body, arm_scope);
        }
    }

    fn resolve_condition(&mut self, condition_id: ir::hir::ConditionId, scope: ScopeId) {
        let Some(condition) = self.hir.arena.condition(condition_id).cloned() else {
            self.invalid_hir_reference(Span::dummy());
            return;
        };
        match condition.kind {
            ConditionKind::Equals(expr) | ConditionKind::Predicate { rhs: expr, .. } => {
                self.resolve_expr(expr, scope)
            }
        }
    }

    fn define(&mut self, scope: ScopeId, name: String, target: DefTarget, span: Span) -> bool {
        if self.scopes.contains_current(scope, &name) {
            self.diagnostics.push(duplicate_definition(&name, span));
            return false;
        }
        let binding = BindingId::new(self.definitions.len());
        self.definitions.push(DefinitionRecord {
            id: binding,
            target,
            name: name.clone(),
            span,
            scope,
        });
        self.scopes.insert(scope, name, binding);
        true
    }

    fn item(&self, item_id: ir::hir::HirId) -> Option<ir::hir::Item> {
        match self.hir.arena.node(item_id) {
            Some(HirNode::Item(item)) => Some(item.clone()),
            _ => None,
        }
    }

    fn name_text(&mut self, name: &Name) -> Option<String> {
        let Some(text) = self.source.get(name.span.start..name.span.end) else {
            self.invalid_hir_reference(name.span);
            return None;
        };
        Some(text.to_owned())
    }

    fn invalid_hir_reference(&mut self, span: Span) {
        self.diagnostics.push(invalid_reference(span));
    }
}

fn item_name_and_target(kind: &ItemKind, item_id: ir::hir::HirId) -> Option<(Name, DefTarget)> {
    match kind {
        ItemKind::Const(constant) => Some((constant.name.clone(), DefTarget::Item(item_id))),
        ItemKind::Struct(structure) => Some((structure.name.clone(), DefTarget::Item(item_id))),
        ItemKind::Enum(enumeration) => Some((enumeration.name.clone(), DefTarget::Item(item_id))),
        ItemKind::Function(function) => Some((function.name.clone(), DefTarget::Item(item_id))),
        ItemKind::Unsupported { .. } => None,
    }
}
