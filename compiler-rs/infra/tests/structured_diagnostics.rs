// compiler-rs/infra/tests/structured_diagnostics.rs

use infra::{Applicability, Diagnostic, DiagnosticArg, DiagnosticValue, MessageId, Severity, Span};

#[test]
fn structured_error_keeps_message_id_and_named_arguments() {
    let diagnostic = Diagnostic::error(
        "type-mismatch",
        MessageId::new("analysis.type-mismatch"),
        Span::new(4, 7),
    )
    .with_arg("expected", DiagnosticValue::Type(String::from("int")))
    .with_arg("found", DiagnosticValue::Type(String::from("string")));

    assert_eq!(diagnostic.severity, Severity::Error);
    assert_eq!(diagnostic.code, "type-mismatch");
    assert_eq!(
        diagnostic.message_id,
        MessageId::new("analysis.type-mismatch")
    );
    assert_eq!(diagnostic.primary, Span::new(4, 7));
    assert_eq!(
        diagnostic.args,
        vec![
            DiagnosticArg::new("expected", DiagnosticValue::Type(String::from("int")),),
            DiagnosticArg::new("found", DiagnosticValue::Type(String::from("string")),),
        ]
    );
}

#[test]
fn structured_diagnostic_supports_labels_notes_and_suggestions() {
    let diagnostic = Diagnostic::warning(
        "deprecated-syntax",
        MessageId::new("syntax.deprecated"),
        Span::new(0, 2),
    )
    .with_label(Span::new(5, 8), MessageId::new("syntax.deprecated.label"))
    .with_note(MessageId::new("syntax.deprecated.note"))
    .with_suggestion(
        Span::new(0, 2),
        String::from("new"),
        Applicability::MachineApplicable,
    );

    assert_eq!(diagnostic.labels.len(), 1);
    assert_eq!(diagnostic.labels[0].span, Span::new(5, 8));
    assert_eq!(
        diagnostic.labels[0].message_id,
        MessageId::new("syntax.deprecated.label")
    );
    assert_eq!(
        diagnostic.notes[0].message_id,
        MessageId::new("syntax.deprecated.note")
    );
    assert_eq!(diagnostic.suggestions[0].replacement, "new");
    assert_eq!(
        diagnostic.suggestions[0].applicability,
        Applicability::MachineApplicable
    );
}

#[test]
fn diagnostic_values_preserve_machine_readable_kinds() {
    let values = [
        DiagnosticValue::Text(String::from("text")),
        DiagnosticValue::Identifier(String::from("name")),
        DiagnosticValue::Type(String::from("int")),
        DiagnosticValue::Integer(-1),
        DiagnosticValue::Unsigned(2),
        DiagnosticValue::Boolean(true),
    ];

    assert!(matches!(values[0], DiagnosticValue::Text(_)));
    assert!(matches!(values[1], DiagnosticValue::Identifier(_)));
    assert!(matches!(values[2], DiagnosticValue::Type(_)));
    assert_eq!(values[3], DiagnosticValue::Integer(-1));
    assert_eq!(values[4], DiagnosticValue::Unsigned(2));
    assert_eq!(values[5], DiagnosticValue::Boolean(true));
}
