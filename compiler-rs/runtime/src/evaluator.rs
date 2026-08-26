// compiler-rs/runtime/src/evaluator.rs

use std::collections::{HashMap, HashSet};

use analysis::resolve::ResolutionTable;
use ir::hir::{HirFile, HirId, HirNode, ItemKind, Stmt};

use crate::ExecutionReport;
use crate::error::RuntimeError;
use crate::value::Value;

/// Evaluates flat HIR without modifying HIR or analysis tables.
pub struct Interpreter<'a> {
    pub(crate) hir: &'a HirFile,
    pub(crate) resolution: &'a ResolutionTable,
    pub(crate) constant_values: Vec<Option<Value>>,
    pub(crate) constant_states: Vec<ConstantState>,
    pub(crate) frames: Vec<Frame>,
    pub(crate) call_stack: HashSet<HirId>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum ConstantState {
    Unvisited,
    Evaluating,
    Done,
}

pub(crate) struct Frame {
    pub(crate) parameters: HashMap<HirId, Value>,
    pub(crate) locals: Vec<Option<Value>>,
}

impl Frame {
    fn new(local_count: usize) -> Self {
        Self {
            parameters: HashMap::new(),
            locals: vec![None; local_count],
        }
    }
}

impl<'a> Interpreter<'a> {
    /// Create an interpreter over an immutable HIR and resolution snapshot.
    pub fn new(hir: &'a HirFile, resolution: &'a ResolutionTable) -> Self {
        Self {
            hir,
            resolution,
            constant_values: vec![None; hir.arena.nodes.len()],
            constant_states: vec![ConstantState::Unvisited; hir.arena.nodes.len()],
            frames: Vec::new(),
            call_stack: HashSet::new(),
        }
    }

    /// Evaluate the module body in source order.
    pub fn run(&mut self) -> Result<ExecutionReport, RuntimeError> {
        let result = self.eval_body(self.hir.module_body)?;
        Ok(ExecutionReport {
            result,
            output: Vec::new(),
        })
    }

    /// Invoke one function item with positional runtime arguments.
    pub fn run_function(
        &mut self,
        item_id: HirId,
        arguments: Vec<Value>,
    ) -> Result<Value, RuntimeError> {
        self.eval_function(item_id, arguments)
    }

    pub(crate) fn eval_body(&mut self, body_id: ir::hir::BodyId) -> Result<Value, RuntimeError> {
        let body = self
            .hir
            .arena
            .body(body_id)
            .cloned()
            .ok_or(RuntimeError::InvalidReference {
                span: infra::Span::dummy(),
                arena: "body",
            })?;
        let mut result = Value::Unit;
        for statement in body.statements {
            result = match statement {
                Stmt::Expr { expr, .. } => self.eval_expr(expr)?,
                Stmt::Assign { target, value, .. } => {
                    let value = self.eval_expr(value)?;
                    let target_span = self.expr_span(target);
                    let local = self.resolution.assignment_local(target).ok_or(
                        RuntimeError::InvalidReference {
                            span: target_span,
                            arena: "assignment",
                        },
                    )?;
                    let frame = self
                        .frames
                        .last_mut()
                        .ok_or(RuntimeError::InvalidReference {
                            span: target_span,
                            arena: "frame",
                        })?;
                    let slot = frame.locals.get_mut(local.index()).ok_or(
                        RuntimeError::InvalidReference {
                            span: target_span,
                            arena: "local",
                        },
                    )?;
                    *slot = Some(value);
                    Value::Unit
                }
                Stmt::Item(item_id) => {
                    self.eval_item(item_id)?;
                    Value::Unit
                }
                Stmt::Error(_) => {
                    return Err(RuntimeError::Unsupported {
                        span: body.span,
                        feature: "error statement",
                    });
                }
            };
        }
        Ok(result)
    }

    pub(crate) fn eval_item(&mut self, item_id: HirId) -> Result<Value, RuntimeError> {
        let index = item_id.index();
        if index >= self.hir.arena.nodes.len() {
            return Err(RuntimeError::InvalidReference {
                span: infra::Span::dummy(),
                arena: "item",
            });
        }
        if let Some(value) = self.constant_values[index].clone() {
            return Ok(value);
        }
        let Some(HirNode::Item(item)) = self.hir.arena.node(item_id).cloned() else {
            return Err(RuntimeError::InvalidReference {
                span: infra::Span::dummy(),
                arena: "item",
            });
        };
        match item.kind {
            ItemKind::Const(constant) => {
                if self.constant_states[index] == ConstantState::Evaluating {
                    return Err(RuntimeError::CyclicConstant { span: item.span });
                }
                self.constant_states[index] = ConstantState::Evaluating;
                let result = self.eval_expr(constant.value);
                match result {
                    Ok(value) => {
                        self.constant_values[index] = Some(value.clone());
                        self.constant_states[index] = ConstantState::Done;
                        Ok(value)
                    }
                    Err(error) => {
                        self.constant_states[index] = ConstantState::Done;
                        Err(error)
                    }
                }
            }
            ItemKind::Function(_) => Ok(Value::Function(item_id)),
            ItemKind::Struct(_) | ItemKind::Enum(_) | ItemKind::Unsupported { .. } => {
                Err(RuntimeError::Unsupported {
                    span: item.span,
                    feature: "declaration value",
                })
            }
        }
    }

    pub(crate) fn eval_function(
        &mut self,
        item_id: HirId,
        arguments: Vec<Value>,
    ) -> Result<Value, RuntimeError> {
        if !self.call_stack.insert(item_id) {
            return Err(RuntimeError::CyclicCall {
                span: self.item_span(item_id),
            });
        }
        let Some(HirNode::Item(item)) = self.hir.arena.node(item_id).cloned() else {
            self.call_stack.remove(&item_id);
            return Err(RuntimeError::InvalidReference {
                span: infra::Span::dummy(),
                arena: "function",
            });
        };
        let ItemKind::Function(function) = item.kind else {
            self.call_stack.remove(&item_id);
            return Err(RuntimeError::Unsupported {
                span: item.span,
                feature: "non-function call",
            });
        };
        if function.params.len() != arguments.len() {
            self.call_stack.remove(&item_id);
            return Err(RuntimeError::ArgumentCount {
                span: item.span,
                expected: function.params.len(),
                found: arguments.len(),
            });
        }
        let mut frame = Frame::new(self.resolution.locals.len());
        for (parameter, value) in function.params.into_iter().zip(arguments) {
            frame.parameters.insert(parameter, value);
        }
        self.frames.push(frame);
        let result = self.eval_body(function.body);
        self.frames.pop();
        self.call_stack.remove(&item_id);
        result
    }
}
