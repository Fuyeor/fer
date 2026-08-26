// ir/src/hir/mod.rs

mod arena;
mod body;
mod expr;
mod file;
mod id;
mod item;
mod r#match;

pub use arena::HirArena;
pub use body::{Body, Stmt};
pub use expr::{
    BinaryOp, CallArg, ChainStep, ChainStepKind, Expr, ExprKind, InterpolatedPart, Literal,
    QuantifierKind, UnaryOp,
};
pub use file::HirFile;
pub use id::{BodyId, ConditionId, ExprId, HirId, MatchArmId, MatchId};
pub use item::{
    Annotation, AnnotationArg, ConstDef, EnumDef, EnumVariant, Field, FieldShape, FunctionDef,
    HirNode, Item, ItemKind, Name, Param, StructDef, TypeRef,
};
pub use r#match::{Condition, ConditionKind, ConditionOp, Match, MatchArm};
