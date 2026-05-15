// infra/tests/symbol_tests.rs
use infra::Symbol;

#[test]
fn symbol_wraps_u32() {
    let sym = Symbol::new(42);
    assert_eq!(sym.0, 42);
}

#[test]
fn symbols_with_same_id_are_equal() {
    let a = Symbol::new(1);
    let b = Symbol::new(1);
    assert_eq!(a, b);
}

#[test]
fn symbols_with_different_id_are_not_equal() {
    let a = Symbol::new(1);
    let b = Symbol::new(2);
    assert_ne!(a, b);
}

#[test]
fn symbol_is_copy() {
    let a = Symbol::new(10);
    let b = a;
    assert_eq!(a, b); // a is still valid because Symbol is Copy
}
