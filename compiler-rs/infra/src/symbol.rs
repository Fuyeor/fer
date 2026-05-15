// infra/src/symbol.rs

use std::collections::HashMap;

/// A unique identifier for an interned string.
///
/// Created by [`Interner::intern`].  Two `Symbol`s are equal if and only if
/// they were created from the same byte sequence (UTF‑8).
///
/// `Symbol` is `Copy` and comparison is a simple integer equality check.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Symbol(u32);

impl Symbol {
    /// Returns the raw index.  This is useful for serialization or
    /// low‑level interop, but should not be used for identity checks
    /// (use `==` instead, which is always correct).
    pub fn raw(self) -> u32 {
        self.0
    }
}

/// A string interner that assigns a unique [`Symbol`] to each distinct string.
///
/// This is the single source of truth for all identifiers and paths in the
/// Fer compiler.  It is intentionally *not* thread‑safe – the compiler
/// currently runs single‑threaded, and this keeps the design simple.
#[derive(Debug, Default)]
pub struct Interner {
    /// Lookup from string content to existing symbol.
    string_to_symbol: HashMap<String, Symbol>,
    /// Reverse mapping, indexed by `Symbol.0`.
    symbols: Vec<String>,
}

impl Interner {
    /// Create an empty interner.
    pub fn new() -> Self {
        Self {
            string_to_symbol: HashMap::new(),
            symbols: Vec::new(),
        }
    }

    /// Intern a string, returning its [`Symbol`].
    ///
    /// If the string has already been interned, the existing symbol is
    /// returned without allocating.
    pub fn intern(&mut self, s: &str) -> Symbol {
        if let Some(&sym) = self.string_to_symbol.get(s) {
            return sym;
        }
        let idx = self.symbols.len() as u32;
        let sym = Symbol(idx);
        self.symbols.push(s.to_owned());
        self.string_to_symbol.insert(s.to_owned(), sym);
        sym
    }

    /// Look up the string for a previously interned [`Symbol`].
    ///
    /// Returns `None` if the symbol was not created by this interner.
    pub fn lookup(&self, symbol: Symbol) -> Option<&str> {
        self.symbols.get(symbol.0 as usize).map(|s| s.as_str())
    }

    /// Returns the number of interned strings.
    pub fn len(&self) -> usize {
        self.symbols.len()
    }

    /// Returns true if the interner contains no strings.
    pub fn is_empty(&self) -> bool {
        self.symbols.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interning_same_string_returns_same_symbol() {
        let mut interner = Interner::new();
        let a = interner.intern("hello");
        let b = interner.intern("hello");
        assert_eq!(a, b);
    }

    #[test]
    fn interning_different_strings_returns_different_symbols() {
        let mut interner = Interner::new();
        let a = interner.intern("hello");
        let b = interner.intern("world");
        assert_ne!(a, b);
    }

    #[test]
    fn lookup_retrieves_string() {
        let mut interner = Interner::new();
        let sym = interner.intern("Fer");
        assert_eq!(interner.lookup(sym), Some("Fer"));
    }

    #[test]
    fn lookup_invalid_symbol_returns_none() {
        let interner = Interner::new();
        assert_eq!(interner.lookup(Symbol(999)), None);
    }

    #[test]
    fn symbol_is_copy() {
        let mut interner = Interner::new();
        let a = interner.intern("copy");
        let b = a;
        assert_eq!(a, b);
    }

    #[test]
    fn symbol_raw_roundtrip() {
        let mut interner = Interner::new();
        let sym = interner.intern("raw");
        let raw = sym.raw();
        let sym2 = Symbol(raw);
        assert_eq!(interner.lookup(sym2), Some("raw"));
    }
}
