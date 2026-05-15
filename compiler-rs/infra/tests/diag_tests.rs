// infra/tests/diag_tests.rs
use infra::{Diagnostic, DiagnosticBag, Severity, Span};

// ---- Diagnostic ----

#[test]
fn creates_error_diagnostic() {
    let span = Span::new(0, 1);
    let diag = Diagnostic::error("E0001", "something went wrong".into(), span);
    assert_eq!(diag.severity, Severity::Error);
    assert_eq!(diag.code, "E0001");
    assert_eq!(diag.message, "something went wrong");
    assert_eq!(diag.span, span);
}

#[test]
fn creates_warning_and_note() {
    let span = Span::dummy();
    let warn = Diagnostic::warning("W0001", "deprecated".into(), span);
    assert_eq!(warn.severity, Severity::Warning);

    let note = Diagnostic::note("N0001", "consider using X".into(), span);
    assert_eq!(note.severity, Severity::Note);
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
    bag.add(Diagnostic::error("E0002", "fail".into(), Span::dummy()));
    assert!(bag.has_errors());
    assert_eq!(bag.count(Severity::Error), 1);
}

#[test]
fn bag_preserves_insertion_order() {
    let mut bag = DiagnosticBag::new();
    let first = Diagnostic::error("E0001", "first".into(), Span::new(0, 1));
    let second = Diagnostic::warning("W0001", "second".into(), Span::new(2, 3));
    bag.add(first.clone());
    bag.add(second.clone());

    let items: Vec<_> = bag.iter().collect();
    assert_eq!(items[0], &first);
    assert_eq!(items[1], &second);
}

#[test]
fn into_diagnostics_consumes_bag() {
    let mut bag = DiagnosticBag::new();
    bag.add(Diagnostic::note("N0001", "note".into(), Span::dummy()));
    let diags = bag.into_diagnostics();
    assert_eq!(diags.len(), 1);
}

#[test]
fn multiple_severities_are_counted_separately() {
    let mut bag = DiagnosticBag::new();
    bag.add(Diagnostic::error("E", "e".into(), Span::dummy()));
    bag.add(Diagnostic::error("E", "e".into(), Span::dummy()));
    bag.add(Diagnostic::warning("W", "w".into(), Span::dummy()));
    assert_eq!(bag.count(Severity::Error), 2);
    assert_eq!(bag.count(Severity::Warning), 1);
    assert_eq!(bag.count(Severity::Note), 0);
}
