// infra/tests/diag_tests.rs
use infra::{Diagnostic, DiagnosticBag, MessageId, Severity, Span};

// ---- Diagnostic ----

#[test]
fn creates_error_diagnostic() {
    let span = Span::new(0, 1);
    let diag = Diagnostic::error(
        "unexpected-token",
        MessageId::new("syntax.unexpected-token"),
        span,
    );
    assert_eq!(diag.severity, Severity::Error);
    assert_eq!(diag.code, "unexpected-token");
    assert_eq!(diag.message_id, MessageId::new("syntax.unexpected-token"));
    assert_eq!(diag.primary, span);
}

#[test]
fn creates_warning_and_note() {
    let span = Span::dummy();
    let warn = Diagnostic::warning(
        "deprecated-syntax",
        MessageId::new("syntax.deprecated-syntax"),
        span,
    );
    assert_eq!(warn.severity, Severity::Warning);
    assert_eq!(warn.code, "deprecated-syntax");

    let note = Diagnostic::note("suggestion", MessageId::new("syntax.suggestion"), span);
    assert_eq!(note.severity, Severity::Note);
    assert_eq!(note.code, "suggestion");
}

#[test]
fn severity_ordering() {
    assert!(Severity::Error > Severity::Warning);
    assert!(Severity::Warning > Severity::Note);
}

// ---- DiagnosticBag ----

#[test]
fn empty_bag_has_no_errors() {
    let bag = DiagnosticBag::new();
    assert!(!bag.has_errors());
    assert_eq!(bag.count(Severity::Error), 0);
}

#[test]
fn adding_error_makes_bag_report_error() {
    let mut bag = DiagnosticBag::new();
    bag.add(Diagnostic::error(
        "unexpected-char",
        MessageId::new("syntax.unexpected-char"),
        Span::dummy(),
    ));
    assert!(bag.has_errors());
    assert_eq!(bag.count(Severity::Error), 1);
}

#[test]
fn bag_preserves_insertion_order() {
    let mut bag = DiagnosticBag::new();
    let first = Diagnostic::error(
        "parse-error",
        MessageId::new("syntax.parse-error"),
        Span::new(0, 1),
    );
    let second = Diagnostic::warning(
        "unused-import",
        MessageId::new("syntax.unused-import"),
        Span::new(2, 3),
    );
    bag.add(first.clone());
    bag.add(second.clone());

    let items: Vec<_> = bag.iter().collect();
    assert_eq!(items[0], &first);
    assert_eq!(items[1], &second);
}

#[test]
fn into_diagnostics_consumes_bag() {
    let mut bag = DiagnosticBag::new();
    bag.add(Diagnostic::note(
        "helpful-hint",
        MessageId::new("syntax.helpful-hint"),
        Span::dummy(),
    ));
    let diags = bag.into_diagnostics();
    assert_eq!(diags.len(), 1);
}

#[test]
fn multiple_severities_are_counted_separately() {
    let mut bag = DiagnosticBag::new();
    bag.add(Diagnostic::error(
        "type-mismatch",
        MessageId::new("analysis.type-mismatch"),
        Span::dummy(),
    ));
    bag.add(Diagnostic::error(
        "missing-field",
        MessageId::new("analysis.missing-field"),
        Span::dummy(),
    ));
    bag.add(Diagnostic::warning(
        "unreachable-code",
        MessageId::new("analysis.unreachable-code"),
        Span::dummy(),
    ));
    assert_eq!(bag.count(Severity::Error), 2);
    assert_eq!(bag.count(Severity::Warning), 1);
    assert_eq!(bag.count(Severity::Note), 0);
}
