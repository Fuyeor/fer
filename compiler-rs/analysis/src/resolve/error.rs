// compiler-rs/analysis/src/resolve/error.rs

use infra::{Diagnostic, Span};

/// Create a diagnostic for a name that has no visible definition.
pub(crate) fn undefined_name(name: &str, span: Span) -> Diagnostic {
    Diagnostic::error(
        "undefined-name",
        format!("cannot resolve name `{name}`"),
        span,
    )
}

/// Create a diagnostic for a second definition in one lexical scope.
pub(crate) fn duplicate_definition(name: &str, span: Span) -> Diagnostic {
    Diagnostic::error(
        "duplicate-definition",
        format!("name `{name}` is already defined in this scope"),
        span,
    )
}

/// Create a diagnostic for an inconsistent internal HIR reference.
pub(crate) fn invalid_reference(span: Span) -> Diagnostic {
    Diagnostic::error(
        "invalid-resolution-reference",
        "resolution encountered an invalid HIR reference".into(),
        span,
    )
}
