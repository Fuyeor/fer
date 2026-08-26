// compiler-rs/runtime/src/error.rs

use std::fmt::{Display, Formatter};

use infra::Span;

use crate::value::ValueKind;

/// Structured failures raised while evaluating valid HIR.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeError {
    InvalidReference {
        span: Span,
        arena: &'static str,
    },
    Unsupported {
        span: Span,
        feature: &'static str,
    },
    TypeMismatch {
        span: Span,
        expected: ValueKind,
        found: ValueKind,
    },
    DivisionByZero {
        span: Span,
    },
    ArgumentCount {
        span: Span,
        expected: usize,
        found: usize,
    },
    CyclicConstant {
        span: Span,
    },
    CyclicCall {
        span: Span,
    },
}

impl Display for RuntimeError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidReference { arena, .. } => {
                write!(formatter, "invalid {arena} reference")
            }
            Self::Unsupported { feature, .. } => {
                write!(formatter, "unsupported runtime feature: {feature}")
            }
            Self::TypeMismatch {
                expected, found, ..
            } => write!(formatter, "expected {expected:?}, found {found:?}"),
            Self::DivisionByZero { .. } => formatter.write_str("division by zero"),
            Self::ArgumentCount {
                expected, found, ..
            } => write!(formatter, "expected {expected} arguments, found {found}"),
            Self::CyclicConstant { .. } => formatter.write_str("cyclic constant dependency"),
            Self::CyclicCall { .. } => formatter.write_str("cyclic function call"),
        }
    }
}

impl std::error::Error for RuntimeError {}
