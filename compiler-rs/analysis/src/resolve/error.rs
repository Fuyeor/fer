// compiler-rs/analysis/src/resolve/error.rs

use infra::{Diagnostic, DiagnosticValue, MessageId, Span};

/// Create a diagnostic for a name that has no visible definition.
pub(crate) fn undefined_name(name: &str, span: Span) -> Diagnostic {
    Diagnostic::error(
        "undefined-name",
        MessageId::new("analysis.undefined-name"),
        span,
    )
    .with_arg("name", DiagnosticValue::Identifier(name.to_owned()))
}

/// Create a diagnostic for a second definition in one lexical scope.
pub(crate) fn duplicate_definition(name: &str, span: Span) -> Diagnostic {
    Diagnostic::error(
        "duplicate-definition",
        MessageId::new("analysis.duplicate-definition"),
        span,
    )
    .with_arg("name", DiagnosticValue::Identifier(name.to_owned()))
}

/// Create a diagnostic for an inconsistent internal HIR reference.
pub(crate) fn invalid_reference(span: Span) -> Diagnostic {
    Diagnostic::error(
        "invalid-resolution-reference",
        MessageId::new("analysis.invalid-resolution-reference"),
        span,
    )
}
