// syntax/src/cst.rs

use infra::Span;

/// Opaque identifier for a CST node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeId(pub u32);

/// A node in the lossless concrete syntax tree.
#[derive(Debug, Clone)]
pub struct CstNode {
    pub id: NodeId,
    pub kind: NodeKind,
    pub span: Span,
    pub children: Vec<NodeId>,
}

#[derive(Debug, Clone)]
pub enum NodeKind {
    /// The whole file: `Module { statements: ... }`
    Module,

    // ---- Declarations ----
    FunctionDef {
        name: Span,
        params: Vec<NodeId>,
        return_type: Option<NodeId>,
        body: NodeId,
    },
    StructDef {
        name: Span,
        fields: Vec<NodeId>,
    },
    LitRegex,
    EnumDef {
        name: Span,
        variants: Vec<NodeId>,
    },
    FieldDef {
        name: Span,
        type_annotation: NodeId,
    },

    // ---- Statements ----
    ExprStmt {
        expr: NodeId,
    },
    AssignStmt {
        target: NodeId,
        value: NodeId,
    },
    NamedArg {
        name: Span,
        value: NodeId,
    },
    MatchArm {
        pattern: Option<NodeId>,
        body: NodeId,
    },

    // ---- Expressions ----
    LitInteger,
    LitFloat,
    LitString,
    LitChar,
    LitBool(bool),
    Ident(Span),
    BinaryOp {
        op: Span,
        lhs: NodeId,
        rhs: NodeId,
    },
    UnaryOp {
        op: Span,
        expr: NodeId,
    },
    Call {
        func: NodeId,
        args: Vec<NodeId>,
    },
    ChainExpr {
        base: NodeId,
        steps: Vec<ChainStep>,
    },
    InterpolatedString {
        parts: Vec<InterpolatedPart>,
    },
    MatchExpr {
        scrutinee: NodeId,
        arms: Vec<NodeId>,
    },
    ConditionExpr {
        // Condition expression used as a pattern (e.g. `< 18`)
        op: Span,
        rhs: NodeId,
    },
    Block {
        statements: Vec<NodeId>,
    },

    Param {
        name: Span,
        type_annotation: NodeId,
    },

    // ---- Patterns ----
    PatternLiteral,
    PatternWildcard,
    PatternCondition {
        op: Span,
        rhs: NodeId,
    },
    PatternDestructure {
        fields: Vec<NodeId>,
    },

    // ---- Module level ----
    Index {
        base: NodeId,
        open_bracket: Span,
        index: NodeId,
        close_bracket: Span,
    },

    ImportItem {
        name: Span,
        alias: Option<NodeId>, // alias is an Ident node
    },

    ImportDecl,
    ExportDecl,

    // ---- Placeholder for error recovery ----
    Error,
}

#[derive(Debug, Clone)]
pub struct ChainStep {
    pub dot_token: Span,
    pub kind: ChainStepKind,
}

#[derive(Debug, Clone)]
pub enum ChainStepKind {
    FieldAccess(Span),
    Call {
        open_paren: Span,
        args: Vec<NodeId>,
        close_paren: Span,
    },
    Index {
        open_bracket: Span,
        index: NodeId,
        close_bracket: Span,
    },
}

#[derive(Debug, Clone)]
pub enum InterpolatedPart {
    Text(String),
    Expr(NodeId),
}
