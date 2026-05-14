// src/parser/ast.rs
use crate::parser::Token;

#[derive(Debug, Clone, PartialEq)]
pub struct Module {
    // /// @/examples/example.fer
    pub path: String,
    // 导入块
    pub imports: Vec<Import>,
    // 顶层声明
    pub body: Vec<Statement>,
    // 导出的名称
    pub exports: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ImportItem {
    pub name: String,
    // handle { xx = yy }
    pub alias: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Import {
    // { candidates score-value }
    pub items: Vec<ImportItem>,
    // e.g., "@fer/std", "./constants"
    pub source: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
    // 变量/常量定义: score-value = 85
    Declaration {
        name: String,
        value: Expression,
        is_mut: bool,
    },
    // 类型定义: Status = Enum { ... }
    TypeDefinition {
        name: String,
        kind: TypeKind,
    },
    // 纯表达式语句: print(...)
    Expression(Expression),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expression {
    // 字面量: 85, `hello`, true
    Literal(Literal),

    // 标识符: score-value
    Identifier(String),

    // 数组操作: candidates[0]
    Index {
        target: Box<Expression>,
        index: Box<Expression>,
    },

    // 匹配/分支块: result = score-value { ... }
    Match {
        target: Box<Expression>,
        arms: Vec<MatchArm>,
    },

    // 函数调用: print(...)
    Call {
        callee: Box<Expression>,
        args: Vec<Expression>,
    },

    // 实例化: Candidate::{ ... }
    Instance {
        type_name: String,
        fields: Vec<(String, Expression)>,
    },

    // `score is {score}`
    InterpolatedString(Vec<Expression>),

    // 1 + 2
    BinaryOp {
        left: Box<Expression>,
        op: Op,
        right: Box<Expression>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum TypeKind {
    // Enum { nice pass failed }
    Enum(Vec<String>),
    // Struct { id = i32, nickname = string }
    Struct(Vec<Field>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Field {
    pub name: String,
    pub type_name: String, // 我们暂存为字符串，在语义分析阶段再链接
}

#[derive(Debug, Clone, PartialEq)]
pub struct MatchArm {
    // >= 90
    pub pattern: Pattern,
    // { Status.nice }
    pub body: Box<Expression>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Pattern {
    // Match a specific value, like 90
    Literal(Literal),
    // Matching range, such as >= 90
    Compare(Token, Literal),
    // { }
    Default,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    Int(i64),
    Float(f64),
    String(String),
    Boolean(bool),
    Array(Vec<Expression>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Op {
    Add,
    Sub,
    Mul,
    Div,
    Eq,
    Ne,
    Gt,
    Lt,
    Ge,
    Le,
    Contains,
}
