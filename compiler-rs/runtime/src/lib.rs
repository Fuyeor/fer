// compiler-rs/runtime/src/lib.rs

mod error;
mod evaluator;
mod expression;
mod ops;
mod value;

pub use error::RuntimeError;
pub use evaluator::Interpreter;
pub use value::Value;
pub use value::ValueKind;

/// The result of evaluating a module, including host-visible output.
#[derive(Debug, Clone, PartialEq)]
pub struct ExecutionReport {
    pub result: Value,
    pub output: Vec<String>,
}
