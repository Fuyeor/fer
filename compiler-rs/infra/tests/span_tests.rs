// infra/tests/span_tests.rs
use infra::Span;

#[test]
fn creates_span_with_offsets() {
    let span = Span::new(10, 20);
    assert_eq!(span.start, 10);
    assert_eq!(span.end, 20);
}

#[test]
fn dummy_span_is_zero_length() {
    let span = Span::dummy();
    assert_eq!(span.start, 0);
    assert_eq!(span.end, 0);
}

#[test]
fn spans_are_comparable() {
    let a = Span::new(0, 5);
    let b = Span::new(0, 5);
    let c = Span::new(1, 5);
    assert_eq!(a, b);
    assert_ne!(a, c);
}
