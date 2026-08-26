// infra/src/diag/diagnostic.rs

use crate::span::Span;

/// Severity of a diagnostic message.
/// Ordered by severity: Error > Warning > Note.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    Note,
    Warning,
    Error,
}

/// Stable identifier for a translatable diagnostic message.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct MessageId(&'static str);

impl MessageId {
    /// Create a message identifier from a compile-time catalog key.
    pub const fn new(value: &'static str) -> Self {
        Self(value)
    }

    /// Return the catalog key used to look up this message.
    pub const fn as_str(self) -> &'static str {
        self.0
    }
}

/// A typed value supplied to a localized diagnostic message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiagnosticValue {
    Text(String),
    Identifier(String),
    Type(String),
    Integer(i128),
    Unsigned(u128),
    Boolean(bool),
}

/// One named interpolation argument for a diagnostic message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiagnosticArg {
    pub name: String,
    pub value: DiagnosticValue,
}

impl DiagnosticArg {
    /// Build one named diagnostic argument.
    pub fn new(name: impl Into<String>, value: DiagnosticValue) -> Self {
        Self {
            name: name.into(),
            value,
        }
    }
}

/// A secondary source label attached to a diagnostic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiagnosticLabel {
    pub span: Span,
    pub message_id: MessageId,
    pub args: Vec<DiagnosticArg>,
}

/// A related note attached to a diagnostic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiagnosticNote {
    pub message_id: MessageId,
    pub args: Vec<DiagnosticArg>,
}

/// Applicability of an automated source replacement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Applicability {
    MachineApplicable,
    MaybeIncorrect,
    Unspecified,
}

/// A structured source replacement attached to a diagnostic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiagnosticSuggestion {
    pub span: Span,
    pub replacement: String,
    pub applicability: Applicability,
}

/// A locale-independent diagnostic event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub severity: Severity,
    /// Machine-readable error code in stable kebab-case.
    pub code: &'static str,
    /// Catalog key for the primary diagnostic message.
    pub message_id: MessageId,
    /// Named values supplied to the catalog template.
    pub args: Vec<DiagnosticArg>,
    /// Primary source span.
    pub primary: Span,
    pub labels: Vec<DiagnosticLabel>,
    pub notes: Vec<DiagnosticNote>,
    pub suggestions: Vec<DiagnosticSuggestion>,
}

impl Diagnostic {
    /// Build a diagnostic event from a severity, stable code, message key, and span.
    pub fn new(
        severity: Severity,
        code: &'static str,
        message_id: MessageId,
        primary: Span,
    ) -> Self {
        Self {
            severity,
            code,
            message_id,
            args: Vec::new(),
            primary,
            labels: Vec::new(),
            notes: Vec::new(),
            suggestions: Vec::new(),
        }
    }

    /// Build an error diagnostic event.
    pub fn error(code: &'static str, message_id: MessageId, primary: Span) -> Self {
        Self::new(Severity::Error, code, message_id, primary)
    }

    /// Build a warning diagnostic event.
    pub fn warning(code: &'static str, message_id: MessageId, primary: Span) -> Self {
        Self::new(Severity::Warning, code, message_id, primary)
    }

    /// Build a note diagnostic event.
    pub fn note(code: &'static str, message_id: MessageId, primary: Span) -> Self {
        Self::new(Severity::Note, code, message_id, primary)
    }

    /// Add one named interpolation argument.
    pub fn with_arg(mut self, name: impl Into<String>, value: DiagnosticValue) -> Self {
        self.args.push(DiagnosticArg::new(name, value));
        self
    }

    /// Add a secondary source label without interpolation arguments.
    pub fn with_label(mut self, span: Span, message_id: MessageId) -> Self {
        self.labels.push(DiagnosticLabel {
            span,
            message_id,
            args: Vec::new(),
        });
        self
    }

    /// Add a related note without interpolation arguments.
    pub fn with_note(mut self, message_id: MessageId) -> Self {
        self.notes.push(DiagnosticNote {
            message_id,
            args: Vec::new(),
        });
        self
    }

    /// Add a source replacement suggestion.
    pub fn with_suggestion(
        mut self,
        span: Span,
        replacement: String,
        applicability: Applicability,
    ) -> Self {
        self.suggestions.push(DiagnosticSuggestion {
            span,
            replacement,
            applicability,
        });
        self
    }
}
