// compiler-rs/runtime/src/ops.rs

use ir::hir::{BinaryOp, ConditionOp};

use crate::error::RuntimeError;
use crate::value::{Value, ValueKind};

pub(super) fn values_equal(left: &Value, right: &Value) -> bool {
    match (left, right) {
        (Value::Integer(left), Value::Integer(right)) => left == right,
        (Value::Float(left), Value::Float(right)) => left == right,
        (Value::Integer(left), Value::Float(right)) => *left as f64 == *right,
        (Value::Float(left), Value::Integer(right)) => *left == *right as f64,
        (Value::String(left), Value::String(right)) => left == right,
        (Value::Bool(left), Value::Bool(right)) => left == right,
        (Value::Char(left), Value::Char(right)) => left == right,
        (Value::Regex(left), Value::Regex(right)) => left == right,
        _ => false,
    }
}

pub(super) fn evaluate_condition(
    op: ConditionOp,
    left: &Value,
    right: &Value,
    span: infra::Span,
) -> Result<bool, RuntimeError> {
    match op {
        ConditionOp::Equals => Ok(values_equal(left, right)),
        ConditionOp::Contains => match (left, right) {
            (Value::String(left), Value::String(right)) => Ok(left.contains(right)),
            _ => Err(RuntimeError::TypeMismatch {
                span,
                expected: ValueKind::String,
                found: right.kind(),
            }),
        },
        ConditionOp::Starts => match (left, right) {
            (Value::String(left), Value::String(right)) => Ok(left.starts_with(right)),
            _ => Err(RuntimeError::TypeMismatch {
                span,
                expected: ValueKind::String,
                found: right.kind(),
            }),
        },
        ConditionOp::Ends => match (left, right) {
            (Value::String(left), Value::String(right)) => Ok(left.ends_with(right)),
            _ => Err(RuntimeError::TypeMismatch {
                span,
                expected: ValueKind::String,
                found: right.kind(),
            }),
        },
        ConditionOp::Less | ConditionOp::Lt => compare_numeric(left, right, span, |a, b| a < b),
        ConditionOp::More | ConditionOp::Gt => compare_numeric(left, right, span, |a, b| a > b),
        ConditionOp::Least | ConditionOp::LtEq => compare_numeric(left, right, span, |a, b| a <= b),
        ConditionOp::Most | ConditionOp::GtEq => compare_numeric(left, right, span, |a, b| a >= b),
        ConditionOp::Matches | ConditionOp::In => Err(RuntimeError::Unsupported {
            span,
            feature: "match predicate",
        }),
    }
}

fn compare_numeric(
    left: &Value,
    right: &Value,
    span: infra::Span,
    compare: impl FnOnce(f64, f64) -> bool,
) -> Result<bool, RuntimeError> {
    let left = number_as_f64(left).ok_or(RuntimeError::TypeMismatch {
        span,
        expected: ValueKind::Integer,
        found: left.kind(),
    })?;
    let right = number_as_f64(right).ok_or(RuntimeError::TypeMismatch {
        span,
        expected: ValueKind::Integer,
        found: right.kind(),
    })?;
    Ok(compare(left, right))
}

fn number_as_f64(value: &Value) -> Option<f64> {
    match value {
        Value::Integer(value) => Some(*value as f64),
        Value::Float(value) => Some(*value),
        _ => None,
    }
}

pub(super) fn evaluate_binary(
    op: BinaryOp,
    left: Value,
    right: Value,
    span: infra::Span,
) -> Result<Value, RuntimeError> {
    if is_comparison(op) {
        return evaluate_condition(binary_condition(op), &left, &right, span).map(Value::Bool);
    }
    if !is_arithmetic(op) {
        return Err(RuntimeError::Unsupported {
            span,
            feature: "binary operator",
        });
    }
    match (left, right) {
        (Value::Integer(left), Value::Integer(right)) => integer_arithmetic(op, left, right, span),
        (left, right) if number_as_f64(&left).is_some() && number_as_f64(&right).is_some() => {
            float_arithmetic(
                op,
                number_as_f64(&left).unwrap_or_default(),
                number_as_f64(&right).unwrap_or_default(),
                span,
            )
        }
        (_left, right) => Err(RuntimeError::TypeMismatch {
            span,
            expected: ValueKind::Integer,
            found: right.kind(),
        }),
    }
}

fn integer_arithmetic(
    op: BinaryOp,
    left: i128,
    right: i128,
    span: infra::Span,
) -> Result<Value, RuntimeError> {
    let result = match op {
        BinaryOp::Add => left.checked_add(right),
        BinaryOp::Subtract => left.checked_sub(right),
        BinaryOp::Multiply => left.checked_mul(right),
        BinaryOp::Divide => {
            if right == 0 {
                return Err(RuntimeError::DivisionByZero { span });
            }
            left.checked_div(right)
        }
        BinaryOp::Remainder => {
            if right == 0 {
                return Err(RuntimeError::DivisionByZero { span });
            }
            left.checked_rem(right)
        }
        _ => None,
    };
    result.map(Value::Integer).ok_or(RuntimeError::Unsupported {
        span,
        feature: "integer overflow",
    })
}

fn float_arithmetic(
    op: BinaryOp,
    left: f64,
    right: f64,
    span: infra::Span,
) -> Result<Value, RuntimeError> {
    if matches!(op, BinaryOp::Divide | BinaryOp::Remainder) && right == 0.0 {
        return Err(RuntimeError::DivisionByZero { span });
    }
    let result = match op {
        BinaryOp::Add => left + right,
        BinaryOp::Subtract => left - right,
        BinaryOp::Multiply => left * right,
        BinaryOp::Divide => left / right,
        BinaryOp::Remainder => left % right,
        _ => {
            return Err(RuntimeError::Unsupported {
                span,
                feature: "float operator",
            });
        }
    };
    Ok(Value::Float(result))
}

fn is_arithmetic(op: BinaryOp) -> bool {
    matches!(
        op,
        BinaryOp::Add
            | BinaryOp::Subtract
            | BinaryOp::Multiply
            | BinaryOp::Divide
            | BinaryOp::Remainder
    )
}

fn is_comparison(op: BinaryOp) -> bool {
    matches!(
        op,
        BinaryOp::Lt
            | BinaryOp::Less
            | BinaryOp::Gt
            | BinaryOp::More
            | BinaryOp::LtEq
            | BinaryOp::Least
            | BinaryOp::GtEq
            | BinaryOp::Most
            | BinaryOp::Equals
            | BinaryOp::Contains
            | BinaryOp::Matches
            | BinaryOp::Starts
            | BinaryOp::Ends
            | BinaryOp::In
    )
}

fn binary_condition(op: BinaryOp) -> ConditionOp {
    match op {
        BinaryOp::Lt => ConditionOp::Lt,
        BinaryOp::Less => ConditionOp::Less,
        BinaryOp::Gt => ConditionOp::Gt,
        BinaryOp::More => ConditionOp::More,
        BinaryOp::LtEq => ConditionOp::LtEq,
        BinaryOp::Least => ConditionOp::Least,
        BinaryOp::GtEq => ConditionOp::GtEq,
        BinaryOp::Most => ConditionOp::Most,
        BinaryOp::Equals => ConditionOp::Equals,
        BinaryOp::Contains => ConditionOp::Contains,
        BinaryOp::Matches => ConditionOp::Matches,
        BinaryOp::Starts => ConditionOp::Starts,
        BinaryOp::Ends => ConditionOp::Ends,
        BinaryOp::In => ConditionOp::In,
        _ => ConditionOp::Equals,
    }
}
