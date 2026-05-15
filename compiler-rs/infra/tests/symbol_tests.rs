// infra/tests/symbol_tests.rs
use infra::Interner;

#[test]
fn symbol_identity_is_based_on_interned_string() {
    let mut interner = Interner::new();
    let a = interner.intern("foo");
    let b = interner.intern("foo");
    let c = interner.intern("bar");
    assert_eq!(a, b);
    assert_ne!(a, c);
}

#[test]
fn symbol_is_copy_and_comparable() {
    let mut interner = Interner::new();
    let a = interner.intern("x");
    let b = a;
    let c = interner.intern("y");
    assert_eq!(a, b);
    assert_ne!(a, c); // different interned string → different Symbol
}

#[test]
fn interner_len_and_empty() {
    let mut interner = Interner::new();
    assert!(interner.is_empty());
    assert_eq!(interner.len(), 0);
    interner.intern("first");
    assert_eq!(interner.len(), 1);
    interner.intern("first"); // duplicate, no change
    assert_eq!(interner.len(), 1);
    interner.intern("second");
    assert_eq!(interner.len(), 2);
}

#[test]
fn lookup_after_many_inserts() {
    let mut interner = Interner::new();
    let syms: Vec<_> = (0..100)
        .map(|i| interner.intern(&format!("s{i}")))
        .collect();
    for (i, sym) in syms.iter().enumerate() {
        assert_eq!(interner.lookup(*sym), Some(&format!("s{i}")[..]));
    }
}
