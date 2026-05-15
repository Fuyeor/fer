// infra/src/diag/diagnostic.rs
/// The core diagnostic types for Fer's compiler.
/// These represent a single diagnostic event: an error, warning, or note.
use crate::span::Span;

/// Severity of a diagnostic message.
/// Ordered by severity: Error > Warning > Note.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    Note,
    Warning,
    Error,
}

/// A single diagnostic entry.
///
/// # Error codes
///
/// The `code` field uses a human‑readable, URL‑friendly kebab‑case identifier
/// (e.g. `"unexpected-token"`, `"missing-semicolon"`).  Numeric codes such as
/// `"E0001"` are intentionally avoided – they are unstable and hard to
/// remember.  Kebab‑case codes also serve as documentation anchors.
///
/// # Design constraint
/// Diagnostic must remain a pure value object.
/// No logic that depends on external state (e.g. file source) lives here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub severity: Severity,
    /// Machine‑readable error code (kebab‑case, e.g. `"type-mismatch"`).
    pub code: &'static str,
    /// Human‑readable explanation.
    pub message: String,
    /// Primary source span.
    pub span: Span,
}

impl Diagnostic {
    pub fn new(severity: Severity, code: &'static str, message: String, span: Span) -> Self {
        Self {
            severity,
            code,
            message,
            span,
        }
    }

    /// Convenience constructor for an error.
    pub fn error(code: &'static str, message: String, span: Span) -> Self {
        Self::new(Severity::Error, code, message, span)
    }

    /// Convenience constructor for a warning.
    pub fn warning(code: &'static str, message: String, span: Span) -> Self {
        Self::new(Severity::Warning, code, message, span)
    }

    /// Convenience constructor for a note (often attached to another diagnostic).
    pub fn note(code: &'static str, message: String, span: Span) -> Self {
        Self::new(Severity::Note, code, message, span)
    }
}
