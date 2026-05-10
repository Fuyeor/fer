// src/interpreter/value.rs
#[derive(Debug, Clone, PartialEq)]
pub enum RuntimeValue {
    Int(i64),
    String(String),
    Bool(bool),
    Array(Vec<RuntimeValue>),
    Null,
}
